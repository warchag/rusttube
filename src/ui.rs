use crate::config;
use crate::ffmpeg_tools;
use crate::i18n::t;
use crate::models::*;
use crate::theme::{self, Palette};
use crate::ytdlp;
use chrono::Local;
use eframe::egui::{self, Color32, FontId, Margin, RichText, Rounding, Sense, Stroke, Vec2};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct RustTubeApp {
    active_tab: ActiveTab,
    url_input: String,
    config: AppConfig,
    pal: Palette,
    deps: Option<DependencyStatus>,
    updating_ytdlp: bool,

    // Batch URLs drawer
    show_batch: bool,
    batch_input: String,

    // Single / Playlist preview
    fetching_info: bool,
    fetch_error: Option<String>,
    video_info: Option<VideoInfo>,
    thumbnail_texture: Option<egui::TextureHandle>,
    thumbnail_url_loaded: Option<String>,

    // Clipboard Watcher
    last_clipboard_check: Instant,
    last_seen_clipboard: String,

    show_advanced: bool,
    tasks: Vec<DownloadTask>,
    history: Vec<HistoryEntry>,
    logs: Vec<String>,
    toast: Option<(String, Instant)>,

    // Media Tools (FFmpeg)
    media_tool: MediaToolState,

    tx: Sender<WorkerMessage>,
    rx: Receiver<WorkerMessage>,
}

impl RustTubeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::setup_fonts(&cc.egui_ctx);

        let (tx, rx) = mpsc::channel();
        let cfg = config::load_config();
        let pal = Palette::from_config(cfg.theme, cfg.accent_color);
        theme::apply_theme(&cc.egui_ctx, cfg.theme, &pal);
        let history = config::load_history();

        let tx_dep = tx.clone();
        let ctx_dep = cc.egui_ctx.clone();
        thread::spawn(move || {
            let status = ytdlp::check_dependencies();
            let _ = tx_dep.send(WorkerMessage::DependenciesChecked(status));
            ctx_dep.request_repaint();
        });

        let initial_clip = arboard::Clipboard::new()
            .ok()
            .and_then(|mut cb| cb.get_text().ok())
            .unwrap_or_default();

        Self {
            active_tab: ActiveTab::Download,
            url_input: String::new(),
            config: cfg,
            pal,
            deps: None,
            updating_ytdlp: false,
            show_batch: false,
            batch_input: String::new(),
            fetching_info: false,
            fetch_error: None,
            video_info: None,
            thumbnail_texture: None,
            thumbnail_url_loaded: None,
            last_clipboard_check: Instant::now(),
            last_seen_clipboard: initial_clip,
            show_advanced: false,
            tasks: Vec::new(),
            history,
            logs: Vec::new(),
            toast: None,
            media_tool: MediaToolState::default(),
            tx,
            rx,
        }
    }

    fn refresh_theme(&mut self, ctx: &egui::Context) {
        self.pal = Palette::from_config(self.config.theme, self.config.accent_color);
        theme::apply_theme(ctx, self.config.theme, &self.pal);
        config::save_config(&self.config);
    }

    fn set_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    fn poll_clipboard_watcher(&mut self, ctx: &egui::Context) {
        if !self.config.clipboard_watch {
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(900));
        if self.last_clipboard_check.elapsed() < Duration::from_millis(850) {
            return;
        }
        self.last_clipboard_check = Instant::now();

        if let Ok(mut cb) = arboard::Clipboard::new() {
            if let Ok(text) = cb.get_text() {
                let trimmed = text.trim().to_string();
                if trimmed != self.last_seen_clipboard {
                    self.last_seen_clipboard = trimmed.clone();
                    if ytdlp::is_youtube_url(&trimmed) && trimmed != self.url_input.trim() {
                        self.url_input = trimmed;
                        self.active_tab = ActiveTab::Download;
                        self.set_toast("📋 Detected YouTube link from clipboard!");
                        self.trigger_fetch_info(ctx);
                    }
                }
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if let Some(first) = dropped.into_iter().next() {
            if first.is_file() {
                self.media_tool.input_file = Some(first);
                self.media_tool.last_error = None;
                self.media_tool.last_output = None;
                self.active_tab = ActiveTab::Tools;
                self.set_toast("Loaded file into Media Tools");
            }
        }
    }

    fn process_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                WorkerMessage::DependenciesChecked(status) => {
                    self.deps = Some(status);
                }
                WorkerMessage::YtdlpUpdateFinished(res) => {
                    self.updating_ytdlp = false;
                    match res {
                        Ok(msg) => {
                            self.logs.push(format!("[update] {msg}"));
                            self.set_toast(msg);
                            let tx = self.tx.clone();
                            let ctx_c = ctx.clone();
                            thread::spawn(move || {
                                let st = ytdlp::check_dependencies();
                                let _ = tx.send(WorkerMessage::DependenciesChecked(st));
                                ctx_c.request_repaint();
                            });
                        }
                        Err(e) => {
                            self.logs.push(format!("[update] ERROR: {e}"));
                            self.set_toast("yt-dlp update failed — see logs");
                        }
                    }
                }
                WorkerMessage::InfoFetched(res) => {
                    self.fetching_info = false;
                    match res {
                        Ok(info) => {
                            self.fetch_error = None;
                            let thumb_url = info.thumbnail_url.clone();
                            self.video_info = Some(info);
                            if let Some(t_url) = thumb_url {
                                if self.thumbnail_url_loaded.as_deref() != Some(t_url.as_str()) {
                                    self.thumbnail_texture = None;
                                    self.thumbnail_url_loaded = Some(t_url.clone());
                                    let tx = self.tx.clone();
                                    let ctx_c = ctx.clone();
                                    thread::spawn(move || {
                                        let res = ytdlp::fetch_thumbnail_rgba(&t_url);
                                        let _ = tx.send(WorkerMessage::ThumbnailLoaded(t_url, res));
                                        ctx_c.request_repaint();
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            self.fetch_error = Some(e);
                        }
                    }
                }
                WorkerMessage::ThumbnailLoaded(url, res) => {
                    if self.thumbnail_url_loaded.as_deref() == Some(url.as_str()) {
                        if let Ok((rgba, size)) = res {
                            let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
                            let tex = ctx.load_texture(
                                "video_thumb",
                                color_img,
                                egui::TextureOptions::LINEAR,
                            );
                            self.thumbnail_texture = Some(tex);
                        }
                    }
                }
                WorkerMessage::TaskStarted { id } => {
                    if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
                        t.status = TaskStatus::Downloading;
                    }
                }
                WorkerMessage::TaskProgress {
                    id,
                    progress,
                    speed,
                    eta,
                    total_size,
                } => {
                    if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
                        t.status = TaskStatus::Downloading;
                        t.progress = progress;
                        t.speed = speed;
                        t.eta = eta;
                        t.total_size = total_size;
                    }
                }
                WorkerMessage::TaskStage { id, stage } => {
                    if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
                        t.status = TaskStatus::Processing(stage);
                    }
                }
                WorkerMessage::TaskLog { id, line } => {
                    let short_id = &id[..id.len().min(6)];
                    self.logs.push(format!("[{short_id}] {line}"));
                    if self.logs.len() > 600 {
                        self.logs.drain(0..120);
                    }
                }
                WorkerMessage::TaskFinished {
                    id,
                    result,
                    cancelled,
                } => {
                    let mut completed_title: Option<String> = None;
                    if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
                        if cancelled {
                            t.status = TaskStatus::Cancelled;
                        } else {
                            match result {
                                Ok(file_path) => {
                                    t.status = TaskStatus::Completed;
                                    t.progress = 1.0;
                                    t.eta = "Done".to_string();
                                    t.output_file = file_path.clone();
                                    completed_title = Some(t.title.clone());

                                    let entry = HistoryEntry {
                                        id: t.id.clone(),
                                        url: t.url.clone(),
                                        title: t.title.clone(),
                                        uploader: t.uploader.clone(),
                                        format_badge: t.options.summary_badge(),
                                        output_path: file_path
                                            .unwrap_or_else(|| t.options.output_dir.clone()),
                                        completed_at: Local::now(),
                                    };
                                    self.history.insert(0, entry);
                                    if self.history.len() > 200 {
                                        self.history.truncate(200);
                                    }
                                    config::save_history(&self.history);
                                }
                                Err(e) => {
                                    t.status = TaskStatus::Failed(e);
                                }
                            }
                        }
                    }

                    self.dispatch_queued_tasks(ctx);

                    // Notify via macOS Notification Center when queue finishes or item completes
                    if let Some(title) = completed_title {
                        let remaining = self
                            .tasks
                            .iter()
                            .filter(|t| {
                                matches!(
                                    t.status,
                                    TaskStatus::Queued
                                        | TaskStatus::Downloading
                                        | TaskStatus::Processing(_)
                                )
                            })
                            .count();
                        if self.config.notify_on_complete && remaining == 0 {
                            ytdlp::send_notification(
                                "RustTube — Download Complete",
                                &format!("Finished: {title}"),
                            );
                        }
                    }
                }
                WorkerMessage::MediaToolFinished(res) => {
                    self.media_tool.running = false;
                    self.media_tool.status_msg = None;
                    match res {
                        Ok(out_path) => {
                            let name = out_path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "file".to_string());
                            self.media_tool.last_output = Some(out_path);
                            self.media_tool.last_error = None;
                            self.set_toast(format!("✔ Exported {name}"));
                            if self.config.notify_on_complete {
                                ytdlp::send_notification(
                                    "RustTube — Media Tools",
                                    &format!("Exported: {name}"),
                                );
                            }
                        }
                        Err(e) => {
                            self.media_tool.last_error = Some(e);
                        }
                    }
                }
            }
        }
    }

    fn trigger_fetch_info(&mut self, ctx: &egui::Context) {
        let url = self.url_input.trim().to_string();
        if url.is_empty() {
            self.fetch_error = Some("Please enter a YouTube URL first.".to_string());
            return;
        }
        let Some(ytdlp_path) = self.deps.as_ref().and_then(|d| d.ytdlp_path.clone()) else {
            self.fetch_error = Some("yt-dlp not found. Please install yt-dlp first.".to_string());
            return;
        };

        self.fetching_info = true;
        self.fetch_error = None;
        let allow_playlist = self.config.options.download_playlist;
        let cookies = self
            .config
            .options
            .cookies_browser
            .ytdlp_arg()
            .map(|s| s.to_string());
        let tx = self.tx.clone();
        let ctx_c = ctx.clone();

        thread::spawn(move || {
            let res = ytdlp::fetch_video_info(&ytdlp_path, &url, allow_playlist, cookies.as_deref());
            let _ = tx.send(WorkerMessage::InfoFetched(res));
            ctx_c.request_repaint();
        });
    }

    fn enqueue_task(
        &mut self,
        url: String,
        title: String,
        uploader: String,
        mut options: DownloadOptions,
        force_single: bool,
    ) {
        if force_single {
            options.download_playlist = false;
        }
        let task = DownloadTask {
            id: Uuid::new_v4().to_string(),
            url,
            title,
            uploader,
            options,
            status: TaskStatus::Queued,
            progress: 0.0,
            speed: "—".to_string(),
            eta: "—".to_string(),
            total_size: "—".to_string(),
            output_file: None,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        };
        self.tasks.insert(0, task);
    }

    fn enqueue_current_download(&mut self, ctx: &egui::Context) {
        let url = self.url_input.trim().to_string();
        if url.is_empty() {
            self.set_toast("Please paste a YouTube URL first");
            return;
        }
        if self.deps.as_ref().and_then(|d| d.ytdlp_path.as_ref()).is_none() {
            self.set_toast("yt-dlp is not installed");
            return;
        }

        let (title, uploader) = if let Some(info) = &self.video_info {
            if info.url == url {
                (info.title.clone(), info.uploader.clone())
            } else {
                (url.clone(), "YouTube".to_string())
            }
        } else {
            (url.clone(), "YouTube".to_string())
        };

        self.enqueue_task(url, title, uploader, self.config.options.clone(), false);
        config::save_config(&self.config);
        self.set_toast("Added to download queue");
        self.dispatch_queued_tasks(ctx);
    }

    fn enqueue_batch_urls(&mut self, ctx: &egui::Context) {
        let urls: Vec<String> = self
            .batch_input
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| l.starts_with("http://") || l.starts_with("https://"))
            .collect();

        if urls.is_empty() {
            self.set_toast("No valid URLs found in batch input");
            return;
        }
        let count = urls.len();
        for url in urls {
            self.enqueue_task(
                url.clone(),
                url,
                "Batch Queue".to_string(),
                self.config.options.clone(),
                true,
            );
        }
        self.batch_input.clear();
        self.show_batch = false;
        self.set_toast(format!("Added {count} URLs to queue"));
        self.dispatch_queued_tasks(ctx);
    }

    fn enqueue_selected_playlist(&mut self, ctx: &egui::Context) {
        let selected: Vec<(String, String, String)> = self
            .video_info
            .as_ref()
            .map(|info| {
                info.playlist_entries
                    .iter()
                    .filter(|e| e.selected)
                    .map(|e| (e.url.clone(), e.title.clone(), info.uploader.clone()))
                    .collect()
            })
            .unwrap_or_default();

        if selected.is_empty() {
            self.set_toast("Select at least 1 video from the playlist");
            return;
        }
        let count = selected.len();
        for (url, title, uploader) in selected {
            self.enqueue_task(url, title, uploader, self.config.options.clone(), true);
        }
        self.set_toast(format!("Queued {count} playlist videos"));
        self.dispatch_queued_tasks(ctx);
    }

    fn dispatch_queued_tasks(&mut self, ctx: &egui::Context) {
        let Some(ytdlp_path) = self.deps.as_ref().and_then(|d| d.ytdlp_path.clone()) else {
            return;
        };
        let ffmpeg_path = self.deps.as_ref().and_then(|d| d.ffmpeg_path.clone());

        let active_count = self
            .tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Downloading | TaskStatus::Processing(_)))
            .count();

        let slots = self.config.max_concurrent.saturating_sub(active_count);
        if slots == 0 {
            return;
        }

        let mut started = 0;
        for task in self.tasks.iter_mut().rev() {
            if started >= slots {
                break;
            }
            if task.status == TaskStatus::Queued {
                task.status = TaskStatus::Downloading;
                task.cancel_flag.store(false, Ordering::Relaxed);
                let id = task.id.clone();
                let url = task.url.clone();
                let opts = task.options.clone();
                let cancel = task.cancel_flag.clone();
                let tx = self.tx.clone();
                let ctx_c = ctx.clone();
                let ytdlp_c = ytdlp_path.clone();
                let ffmpeg_c = ffmpeg_path.clone();

                thread::spawn(move || {
                    ytdlp::run_download_task(
                        id, url, opts, ytdlp_c, ffmpeg_c, cancel, tx, ctx_c,
                    );
                });
                started += 1;
            }
        }
    }

    fn paste_from_clipboard(&mut self, ctx: &egui::Context) {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            if let Ok(text) = cb.get_text() {
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    self.url_input = trimmed.clone();
                    self.last_seen_clipboard = trimmed;
                    if self.config.auto_fetch_on_paste {
                        self.trigger_fetch_info(ctx);
                    }
                }
            }
        }
    }

    fn pick_output_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new()
            .set_directory(&self.config.options.output_dir)
            .pick_folder()
        {
            self.config.options.output_dir = folder;
            config::save_config(&self.config);
        }
    }

    fn apply_preset(&mut self, preset: &str) {
        match preset {
            "music" => {
                self.config.options.mode = MediaMode::Audio;
                self.config.options.audio_format = AudioFormat::Mp3;
                self.config.options.audio_bitrate = AudioBitrate::K320;
                self.config.options.embed_thumbnail = true;
                self.config.options.embed_metadata = true;
                self.config.options.trim_enabled = false;
                self.set_toast("Preset: Music MP3 320k");
            }
            "iphone" => {
                self.config.options.mode = MediaMode::Video;
                self.config.options.video_quality = VideoQuality::Q1080p;
                self.config.options.video_format = VideoFormat::Mp4;
                self.config.options.video_codec = VideoCodec::H264;
                self.config.options.trim_enabled = false;
                self.set_toast("Preset: iPhone / QuickTime MP4 (H.264)");
            }
            "4k" => {
                self.config.options.mode = MediaMode::Video;
                self.config.options.video_quality = VideoQuality::Q2160p;
                self.config.options.video_format = VideoFormat::Mkv;
                self.config.options.video_codec = VideoCodec::Auto;
                self.config.options.trim_enabled = false;
                self.set_toast("Preset: 4K Master MKV");
            }
            "podcast" => {
                self.config.options.mode = MediaMode::Audio;
                self.config.options.audio_format = AudioFormat::M4a;
                self.config.options.audio_bitrate = AudioBitrate::K128;
                self.config.options.sponsorblock = true;
                self.config.options.trim_enabled = false;
                self.set_toast("Preset: Podcast M4A + SponsorBlock");
            }
            "clip" => {
                self.config.options.mode = MediaMode::Video;
                self.config.options.video_quality = VideoQuality::Q1080p;
                self.config.options.video_format = VideoFormat::Mp4;
                self.config.options.video_codec = VideoCodec::H264;
                self.config.options.trim_enabled = true;
                self.show_advanced = true;
                self.set_toast("Preset: Quick Clip (set Start/End below)");
            }
            _ => {}
        }
        config::save_config(&self.config);
    }

    fn render_sidebar(&mut self, ctx: &egui::Context) {
        let pal = self.pal;
        let lang = self.config.language;
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(228.0)
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_sidebar)
                    .inner_margin(Margin::symmetric(16.0, 20.0))
                    .stroke(Stroke::new(1.0, pal.border)),
            )
            .show(ctx, |ui| {
                // Brand header
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
                    ui.painter()
                        .rect_filled(rect, Rounding::same(9.0), pal.accent);
                    let c = rect.center();
                    let tri = vec![
                        egui::pos2(c.x - 4.0, c.y - 7.0),
                        egui::pos2(c.x - 4.0, c.y + 7.0),
                        egui::pos2(c.x + 7.0, c.y),
                    ];
                    ui.painter()
                        .add(egui::Shape::convex_polygon(tri, Color32::WHITE, Stroke::NONE));

                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("RustTube")
                                .size(19.0)
                                .strong()
                                .color(pal.text_primary),
                        );
                        ui.label(
                            RichText::new(t(lang, "Studio Pro Edition", "สตูดิโอดาวน์โหลด"))
                                .size(11.0)
                                .color(pal.text_muted),
                        );
                    });
                });

                ui.add_space(22.0);

                let active_count = self
                    .tasks
                    .iter()
                    .filter(|t| {
                        matches!(
                            t.status,
                            TaskStatus::Queued | TaskStatus::Downloading | TaskStatus::Processing(_)
                        )
                    })
                    .count();

                self.nav_button(
                    ui,
                    ActiveTab::Download,
                    t(lang, "⬇  Downloader", "⬇  ดาวน์โหลด"),
                    if active_count > 0 {
                        Some(active_count.to_string())
                    } else {
                        None
                    },
                );
                ui.add_space(5.0);
                self.nav_button(
                    ui,
                    ActiveTab::Tools,
                    t(lang, "🛠  Media Tools", "🛠  เครื่องมือไฟล์"),
                    if self.media_tool.running {
                        Some("…".to_string())
                    } else {
                        None
                    },
                );
                ui.add_space(5.0);
                self.nav_button(
                    ui,
                    ActiveTab::History,
                    t(lang, "🕘  History", "🕘  ประวัติ"),
                    if !self.history.is_empty() {
                        Some(self.history.len().to_string())
                    } else {
                        None
                    },
                );
                ui.add_space(5.0);
                self.nav_button(
                    ui,
                    ActiveTab::Settings,
                    t(lang, "⚙  Settings & Logs", "⚙  ตั้งค่า & บันทึก"),
                    None,
                );

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    // Engine status card
                    egui::Frame::none()
                        .fill(pal.bg_card)
                        .rounding(Rounding::same(10.0))
                        .stroke(Stroke::new(1.0, pal.border))
                        .inner_margin(Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                RichText::new(t(lang, "ENGINE STATUS", "สถานะระบบ"))
                                    .size(10.5)
                                    .strong()
                                    .color(pal.text_muted),
                            );
                            ui.add_space(6.0);
                            if let Some(deps) = &self.deps {
                                status_dot_row(ui, &pal, "yt-dlp", deps.ytdlp_version.as_deref());
                                ui.add_space(4.0);
                                status_dot_row(ui, &pal, "ffmpeg", deps.ffmpeg_version.as_deref());
                            } else {
                                ui.label(
                                    RichText::new(t(lang, "Checking tools...", "กำลังตรวจสอบ..."))
                                        .size(12.0)
                                        .color(pal.text_secondary),
                                );
                            }
                        });

                    ui.add_space(8.0);

                    // Quick action row: Language + Theme + Folder
                    ui.horizontal(|ui| {
                        let w = (ui.available_width() - 8.0) / 2.0;
                        if ui
                            .add_sized(
                                [w, 30.0],
                                egui::Button::new(
                                    RichText::new(lang.short_badge())
                                        .size(12.0)
                                        .color(pal.text_primary),
                                )
                                .fill(pal.bg_card_alt)
                                .stroke(Stroke::new(1.0, pal.border))
                                .rounding(Rounding::same(8.0)),
                            )
                            .on_hover_text(t(lang, "Switch to Thai", "เปลี่ยนเป็นภาษาอังกฤษ"))
                            .clicked()
                        {
                            self.config.language = self.config.language.toggle();
                            config::save_config(&self.config);
                            self.set_toast(t(
                                self.config.language,
                                "Language: English",
                                "เปลี่ยนภาษาเป็น: ภาษาไทย",
                            ));
                        }

                        let theme_btn_label = format!(
                            "{} {}",
                            self.config.theme.emoji(),
                            t(lang, "Theme", "ธีม")
                        );
                        if ui
                            .add_sized(
                                [w, 30.0],
                                egui::Button::new(
                                    RichText::new(theme_btn_label)
                                        .size(12.0)
                                        .color(pal.text_primary),
                                )
                                .fill(pal.bg_card_alt)
                                .stroke(Stroke::new(1.0, pal.border))
                                .rounding(Rounding::same(8.0)),
                            )
                            .on_hover_text(format!(
                                "{} (Click to switch)",
                                self.config.theme.label()
                            ))
                            .clicked()
                        {
                            self.config.theme = self.config.theme.next();
                            self.refresh_theme(ctx);
                            self.set_toast(format!(
                                "{} {}",
                                self.config.theme.emoji(),
                                self.config.theme.label()
                            ));
                        }
                    });

                    ui.add_space(6.0);

                    if ui
                        .add_sized(
                            [ui.available_width(), 30.0],
                            egui::Button::new(
                                RichText::new(t(lang, "📂 Open Downloads", "📂 เปิดโฟลเดอร์ดาวน์โหลด"))
                                    .size(12.0)
                                    .color(pal.text_primary),
                            )
                            .fill(pal.bg_card_alt)
                            .stroke(Stroke::new(1.0, pal.border))
                            .rounding(Rounding::same(8.0)),
                        )
                        .clicked()
                    {
                        ytdlp::open_in_finder(&self.config.options.output_dir, false);
                    }

                    ui.add_space(6.0);

                    // Clipboard Watcher quick toggle
                    let watch_on = self.config.clipboard_watch;
                    let watch_label = if watch_on {
                        t(lang, "📋 Watch Clipboard: ON", "📋 จับลิงก์อัตโนมัติ: เปิด")
                    } else {
                        t(lang, "📋 Watch Clipboard: OFF", "📋 จับลิงก์อัตโนมัติ: ปิด")
                    };
                    if ui
                        .add_sized(
                            [ui.available_width(), 32.0],
                            egui::Button::new(
                                RichText::new(watch_label).size(12.0).color(if watch_on {
                                    Color32::WHITE
                                } else {
                                    pal.text_secondary
                                }),
                            )
                            .fill(if watch_on { pal.green } else { pal.bg_card_alt })
                            .stroke(Stroke::new(
                                1.0,
                                if watch_on { pal.green } else { pal.border },
                            ))
                            .rounding(Rounding::same(8.0)),
                        )
                        .clicked()
                    {
                        self.config.clipboard_watch = !self.config.clipboard_watch;
                        config::save_config(&self.config);
                        self.set_toast(if self.config.clipboard_watch {
                            t(lang, "Clipboard Watcher enabled", "เปิดโหมดจับลิงก์จาก Clipboard แล้ว")
                        } else {
                            t(lang, "Clipboard Watcher paused", "ปิดโหมดจับลิงก์อัตโนมัติแล้ว")
                        });
                    }
                });
            });
    }

    fn nav_button(
        &mut self,
        ui: &mut egui::Ui,
        tab: ActiveTab,
        label: &str,
        badge: Option<String>,
    ) {
        let pal = self.pal;
        let selected = self.active_tab == tab;
        let fill = if selected {
            pal.accent_soft
        } else {
            Color32::TRANSPARENT
        };
        let stroke = if selected {
            Stroke::new(1.0, pal.accent.gamma_multiply(0.6))
        } else {
            Stroke::NONE
        };
        let text_color = if selected {
            pal.text_primary
        } else {
            pal.text_secondary
        };

        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 38.0), Sense::click());
        let bg = if response.hovered() && !selected {
            pal.bg_card_alt
        } else {
            fill
        };
        ui.painter().rect(rect, Rounding::same(9.0), bg, stroke);

        ui.painter().text(
            egui::pos2(rect.left() + 12.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(13.5),
            text_color,
        );

        if let Some(b) = badge {
            let badge_rect = egui::Rect::from_center_size(
                egui::pos2(rect.right() - 22.0, rect.center().y),
                Vec2::new(26.0, 20.0),
            );
            ui.painter().rect_filled(
                badge_rect,
                Rounding::same(10.0),
                if selected { pal.accent } else { pal.bg_card_alt },
            );
            ui.painter().text(
                badge_rect.center(),
                egui::Align2::CENTER_CENTER,
                b,
                FontId::proportional(11.0),
                if selected { Color32::WHITE } else { pal.text_primary },
            );
        }

        if response.clicked() {
            self.active_tab = tab;
        }
    }

    fn render_download_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let pal = self.pal;
        let lang = self.config.language;
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(t(lang, "Download Video & Audio", "ดาวน์โหลดวิดีโอ & เสียง"))
                                .size(22.0)
                                .strong()
                                .color(pal.text_primary),
                        );
                        ui.label(
                            RichText::new(t(
                                lang,
                                "Paste a link, pick a preset, batch-queue URLs, or drag & drop local media files.",
                                "วางลิงก์ YouTube เลือกโปรไฟล์ด่วน วางหลายลิงก์ หรือลากไฟล์มาวางเพื่อแปลงไฟล์ได้เลย",
                            ))
                            .size(12.5)
                            .color(pal.text_secondary),
                        );
                    });
                });

                ui.add_space(14.0);

                // URL Input Card
                card_frame(&pal).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let input_width = (ui.available_width() - 300.0).max(180.0);
                        let resp = ui.add_sized(
                            [input_width, 38.0],
                            egui::TextEdit::singleline(&mut self.url_input)
                                .hint_text(t(
                                    lang,
                                    "https://www.youtube.com/watch?v=... or playlist URL",
                                    "วางลิงก์ YouTube หรือลิงก์เพลย์ลิสต์ที่นี่...",
                                ))
                                .margin(Margin::symmetric(12.0, 9.0)),
                        );
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.trigger_fetch_info(ctx);
                        }

                        if ui
                            .add_sized(
                                [78.0, 38.0],
                                egui::Button::new(
                                    RichText::new(t(lang, "📋 Paste", "📋 วางลิงก์"))
                                        .size(13.0)
                                        .color(pal.text_primary),
                                )
                                .fill(pal.bg_card_alt)
                                .stroke(Stroke::new(1.0, pal.border))
                                .rounding(Rounding::same(8.0)),
                            )
                            .clicked()
                        {
                            self.paste_from_clipboard(ctx);
                        }

                        let fetch_label = if self.fetching_info {
                            t(lang, "⏳ Loading...", "⏳ กำลังดึง...")
                        } else {
                            t(lang, "🔍 Fetch Info", "🔍 ดึงข้อมูล")
                        };
                        if ui
                            .add_enabled(
                                !self.fetching_info,
                                egui::Button::new(
                                    RichText::new(fetch_label)
                                        .size(13.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .min_size(Vec2::new(112.0, 38.0))
                                .fill(pal.blue)
                                .rounding(Rounding::same(8.0)),
                            )
                            .clicked()
                        {
                            self.trigger_fetch_info(ctx);
                        }

                        if ui
                            .add_sized(
                                [88.0, 38.0],
                                egui::Button::new(
                                    RichText::new(t(lang, "📑 Batch", "📑 หลายลิงก์"))
                                        .size(13.0)
                                        .color(if self.show_batch {
                                            Color32::WHITE
                                        } else {
                                            pal.text_primary
                                        }),
                                )
                                .fill(if self.show_batch {
                                    pal.accent
                                } else {
                                    pal.bg_card_alt
                                })
                                .stroke(Stroke::new(1.0, pal.border))
                                .rounding(Rounding::same(8.0)),
                            )
                            .clicked()
                        {
                            self.show_batch = !self.show_batch;
                        }
                    });

                    // Expandable Batch Download Drawer
                    if self.show_batch {
                        ui.add_space(10.0);
                        egui::Frame::none()
                            .fill(pal.bg_card_alt)
                            .rounding(Rounding::same(10.0))
                            .stroke(Stroke::new(1.0, pal.border))
                            .inner_margin(Margin::same(12.0))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(t(
                                            lang,
                                            "Batch URLs (one link per line)",
                                            "วางหลายลิงก์พร้อมกัน (บรรทัดละ 1 ลิงก์)",
                                        ))
                                        .size(13.0)
                                        .strong()
                                        .color(pal.text_primary),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new(t(
                                                            lang,
                                                            "📄 Import .txt",
                                                            "📄 นำเข้า .txt",
                                                        ))
                                                        .size(12.0),
                                                    )
                                                    .fill(pal.bg_card)
                                                    .stroke(Stroke::new(1.0, pal.border))
                                                    .rounding(Rounding::same(6.0)),
                                                )
                                                .clicked()
                                            {
                                                if let Some(file) = rfd::FileDialog::new()
                                                    .add_filter("Text files", &["txt"])
                                                    .pick_file()
                                                {
                                                    if let Ok(content) = std::fs::read_to_string(file) {
                                                        if !self.batch_input.is_empty() {
                                                            self.batch_input.push('\n');
                                                        }
                                                        self.batch_input.push_str(&content);
                                                    }
                                                }
                                            }
                                        },
                                    );
                                });
                                ui.add_space(6.0);
                                ui.add_sized(
                                    [ui.available_width(), 92.0],
                                    egui::TextEdit::multiline(&mut self.batch_input)
                                        .hint_text("https://youtu.be/...\nhttps://www.youtube.com/watch?v=...")
                                        .desired_rows(4),
                                );
                                ui.add_space(8.0);
                                let valid_count = self
                                    .batch_input
                                    .lines()
                                    .map(|l| l.trim())
                                    .filter(|l| l.starts_with("http://") || l.starts_with("https://"))
                                    .count();
                                ui.horizontal(|ui| {
                                    let count_lbl = match lang {
                                        AppLanguage::En => format!("{valid_count} valid links detected"),
                                        AppLanguage::Th => format!("พบ {valid_count} ลิงก์ที่พร้อมดาวน์โหลด"),
                                    };
                                    ui.label(
                                        RichText::new(count_lbl)
                                            .size(12.0)
                                            .color(pal.text_secondary),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let btn_lbl = match lang {
                                                AppLanguage::En => format!("⬇ Queue All ({valid_count})"),
                                                AppLanguage::Th => format!("⬇ เพิ่มเข้าคิวทั้งหมด ({valid_count})"),
                                            };
                                            if ui
                                                .add_enabled(
                                                    valid_count > 0,
                                                    egui::Button::new(
                                                        RichText::new(btn_lbl)
                                                            .size(12.5)
                                                            .strong()
                                                            .color(Color32::WHITE),
                                                    )
                                                    .fill(pal.accent)
                                                    .rounding(Rounding::same(7.0)),
                                                )
                                                .clicked()
                                            {
                                                self.enqueue_batch_urls(ctx);
                                            }
                                        },
                                    );
                                });
                            });
                    }

                    if let Some(err) = &self.fetch_error {
                        ui.add_space(8.0);
                        ui.label(RichText::new(format!("⚠ {err}")).size(12.0).color(pal.red));
                    }
                });

                ui.add_space(12.0);

                // Video Preview Card
                if let Some(info) = self.video_info.clone() {
                    self.render_preview_card(ui, &info);
                    ui.add_space(12.0);

                    // Playlist Picker Card
                    if info.is_playlist && !info.playlist_entries.is_empty() {
                        self.render_playlist_picker(ui, ctx);
                        ui.add_space(12.0);
                    }
                }

                // Format & Presets Card
                self.render_options_card(ui, ctx);

                ui.add_space(16.0);

                // Queue Section
                self.render_queue_section(ui, ctx);
            });
    }

    fn render_preview_card(&self, ui: &mut egui::Ui, info: &VideoInfo) {
        let pal = self.pal;
        card_frame(&pal).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                let thumb_size = Vec2::new(208.0, 117.0);
                if let Some(tex) = &self.thumbnail_texture {
                    let img = egui::Image::new(tex)
                        .fit_to_exact_size(thumb_size)
                        .rounding(Rounding::same(8.0));
                    ui.add(img);
                } else {
                    let (rect, _) = ui.allocate_exact_size(thumb_size, Sense::hover());
                    ui.painter()
                        .rect_filled(rect, Rounding::same(8.0), pal.bg_card_alt);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Loading preview...",
                        FontId::proportional(12.0),
                        pal.text_muted,
                    );
                }

                ui.add_space(14.0);

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&info.title)
                            .size(16.0)
                            .strong()
                            .color(pal.text_primary),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("📺 {}", info.uploader))
                            .size(13.0)
                            .color(pal.text_secondary),
                    );
                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        pill_badge(
                            ui,
                            &format!("⏱ {}", info.formatted_duration()),
                            pal.bg_card_alt,
                            pal.text_primary,
                        );
                        let views = info.formatted_views();
                        if !views.is_empty() {
                            pill_badge(ui, &format!("👁 {views}"), pal.bg_card_alt, pal.text_secondary);
                        }
                        let date = info.formatted_date();
                        if !date.is_empty() {
                            pill_badge(ui, &format!("📅 {date}"), pal.bg_card_alt, pal.text_secondary);
                        }
                        if info.is_playlist {
                            pill_badge(
                                ui,
                                &format!("📑 Playlist ({} items)", info.playlist_entries.len()),
                                pal.accent_soft,
                                pal.accent,
                            );
                        }
                    });

                    // Estimated sizes per resolution
                    if !info.format_estimates.is_empty() || info.audio_size_bytes.is_some() {
                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new("Est. Size:")
                                    .size(11.5)
                                    .color(pal.text_muted),
                            );
                            for est in info.format_estimates.iter().take(4) {
                                pill_badge(
                                    ui,
                                    &format!("{}p ~{}", est.height, est.formatted_size()),
                                    pal.bg_card_alt,
                                    pal.blue,
                                );
                            }
                            if let Some(a_bytes) = info.audio_size_bytes {
                                pill_badge(
                                    ui,
                                    &format!("Audio ~{}", format_bytes(a_bytes)),
                                    pal.bg_card_alt,
                                    pal.green,
                                );
                            }
                        });
                    }
                });
            });
        });
    }

    fn render_playlist_picker(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let pal = self.pal;
        let mut download_clicked = false;
        card_frame(&pal).show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(info) = &mut self.video_info {
                let total = info.playlist_entries.len();
                let selected_count = info.playlist_entries.iter().filter(|e| e.selected).count();

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Playlist Picker ({selected_count} / {total} selected)"))
                            .size(14.5)
                            .strong()
                            .color(pal.text_primary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                selected_count > 0,
                                egui::Button::new(
                                    RichText::new(format!("⬇ Download Selected ({selected_count})"))
                                        .size(12.5)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(pal.accent)
                                .rounding(Rounding::same(7.0)),
                            )
                            .clicked()
                        {
                            download_clicked = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Deselect All").size(12.0))
                                    .fill(pal.bg_card_alt)
                                    .rounding(Rounding::same(6.0)),
                            )
                            .clicked()
                        {
                            for e in &mut info.playlist_entries {
                                e.selected = false;
                            }
                        }
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Select All").size(12.0))
                                    .fill(pal.bg_card_alt)
                                    .rounding(Rounding::same(6.0)),
                            )
                            .clicked()
                        {
                            for e in &mut info.playlist_entries {
                                e.selected = true;
                            }
                        }
                    });
                });

                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .id_salt("playlist_scroll")
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for entry in &mut info.playlist_entries {
                            ui.horizontal(|ui| {
                                ui.checkbox(
                                    &mut entry.selected,
                                    RichText::new(format!("{:02}. {}", entry.index, entry.title))
                                        .size(12.5)
                                        .color(pal.text_primary),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(entry.formatted_duration())
                                                .size(11.5)
                                                .color(pal.text_muted),
                                        );
                                    },
                                );
                            });
                        }
                    });
            }
        });

        if download_clicked {
            self.enqueue_selected_playlist(ctx);
        }
    }

    fn render_options_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let pal = self.pal;
        card_frame(&pal).show(ui, |ui| {
            ui.set_width(ui.available_width());

            // Quick Preset Profiles bar
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Quick Presets:")
                        .size(12.0)
                        .strong()
                        .color(pal.text_muted),
                );
                let presets = [
                    ("music", "🎵 Music MP3 320k"),
                    ("iphone", "📱 iPhone MP4 (H.264)"),
                    ("4k", "🎬 4K Master"),
                    ("podcast", "🎙 Podcast"),
                    ("clip", "✂ Quick Clip"),
                ];
                for (key, label) in presets {
                    if ui
                        .add(
                            egui::Button::new(RichText::new(label).size(11.5).color(pal.text_primary))
                                .fill(pal.bg_card_alt)
                                .stroke(Stroke::new(1.0, pal.border))
                                .rounding(Rounding::same(14.0)),
                        )
                        .clicked()
                    {
                        self.apply_preset(key);
                    }
                }
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // Mode + Quality / Format / Codec row
            ui.horizontal_wrapped(|ui| {
                for mode in [MediaMode::Video, MediaMode::Audio] {
                    let selected = self.config.options.mode == mode;
                    let btn = egui::Button::new(
                        RichText::new(mode.label())
                            .size(13.0)
                            .strong()
                            .color(if selected {
                                Color32::WHITE
                            } else {
                                pal.text_secondary
                            }),
                    )
                    .min_size(Vec2::new(108.0, 34.0))
                    .fill(if selected { pal.accent } else { pal.bg_card_alt })
                    .stroke(Stroke::new(
                        1.0,
                        if selected { pal.accent } else { pal.border },
                    ))
                    .rounding(Rounding::same(8.0));

                    if ui.add(btn).clicked() {
                        self.config.options.mode = mode;
                        config::save_config(&self.config);
                    }
                }

                ui.add_space(8.0);

                match self.config.options.mode {
                    MediaMode::Video => {
                        let sel_text = if let Some(est) = self
                            .video_info
                            .as_ref()
                            .and_then(|i| i.estimate_for_quality(self.config.options.video_quality))
                        {
                            format!("{} ({est})", self.config.options.video_quality.label())
                        } else {
                            self.config.options.video_quality.label().to_string()
                        };

                        ui.label(RichText::new("Quality:").size(12.5).color(pal.text_secondary));
                        egui::ComboBox::from_id_salt("video_quality")
                            .width(165.0)
                            .selected_text(sel_text)
                            .show_ui(ui, |ui| {
                                for q in VideoQuality::ALL {
                                    let opt_label = if let Some(est) = self
                                        .video_info
                                        .as_ref()
                                        .and_then(|i| i.estimate_for_quality(*q))
                                    {
                                        format!("{} ({est})", q.label())
                                    } else {
                                        q.label().to_string()
                                    };
                                    if ui
                                        .selectable_value(
                                            &mut self.config.options.video_quality,
                                            *q,
                                            opt_label,
                                        )
                                        .changed()
                                    {
                                        config::save_config(&self.config);
                                    }
                                }
                            });

                        ui.label(RichText::new("Format:").size(12.5).color(pal.text_secondary));
                        egui::ComboBox::from_id_salt("video_format")
                            .width(74.0)
                            .selected_text(self.config.options.video_format.label())
                            .show_ui(ui, |ui| {
                                for f in VideoFormat::ALL {
                                    if ui
                                        .selectable_value(
                                            &mut self.config.options.video_format,
                                            *f,
                                            f.label(),
                                        )
                                        .changed()
                                    {
                                        config::save_config(&self.config);
                                    }
                                }
                            });

                        ui.label(RichText::new("Codec:").size(12.5).color(pal.text_secondary));
                        egui::ComboBox::from_id_salt("video_codec")
                            .width(136.0)
                            .selected_text(self.config.options.video_codec.short_label())
                            .show_ui(ui, |ui| {
                                for c in VideoCodec::ALL {
                                    if ui
                                        .selectable_value(
                                            &mut self.config.options.video_codec,
                                            *c,
                                            c.label(),
                                        )
                                        .changed()
                                    {
                                        config::save_config(&self.config);
                                    }
                                }
                            });
                    }
                    MediaMode::Audio => {
                        ui.label(RichText::new("Format:").size(12.5).color(pal.text_secondary));
                        egui::ComboBox::from_id_salt("audio_format")
                            .width(85.0)
                            .selected_text(self.config.options.audio_format.label())
                            .show_ui(ui, |ui| {
                                for f in AudioFormat::ALL {
                                    if ui
                                        .selectable_value(
                                            &mut self.config.options.audio_format,
                                            *f,
                                            f.label(),
                                        )
                                        .changed()
                                    {
                                        config::save_config(&self.config);
                                    }
                                }
                            });

                        ui.add_space(6.0);
                        ui.label(RichText::new("Bitrate:").size(12.5).color(pal.text_secondary));
                        egui::ComboBox::from_id_salt("audio_bitrate")
                            .width(148.0)
                            .selected_text(self.config.options.audio_bitrate.label())
                            .show_ui(ui, |ui| {
                                for b in AudioBitrate::ALL {
                                    if ui
                                        .selectable_value(
                                            &mut self.config.options.audio_bitrate,
                                            *b,
                                            b.label(),
                                        )
                                        .changed()
                                    {
                                        config::save_config(&self.config);
                                    }
                                }
                            });
                    }
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(10.0);

            // Output folder + Pro Options + Download CTA
            ui.horizontal(|ui| {
                ui.label(RichText::new("Save to:").size(12.5).color(pal.text_secondary));
                let dir_display = shorten_path(&self.config.options.output_dir, 34);
                ui.label(RichText::new(dir_display).size(12.5).color(pal.text_primary));
                if ui
                    .add(
                        egui::Button::new(RichText::new("Change...").size(12.0))
                            .fill(pal.bg_card_alt)
                            .stroke(Stroke::new(1.0, pal.border))
                            .rounding(Rounding::same(6.0)),
                    )
                    .clicked()
                {
                    self.pick_output_folder();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("⬇  Download Now")
                                    .size(14.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .min_size(Vec2::new(152.0, 38.0))
                            .fill(pal.accent)
                            .rounding(Rounding::same(9.0)),
                        )
                        .clicked()
                    {
                        self.enqueue_current_download(ctx);
                    }

                    ui.add_space(8.0);

                    let adv_label = if self.show_advanced {
                        "▲ Pro Options"
                    } else {
                        "▼ Pro Options"
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(adv_label).size(12.5).color(pal.text_secondary),
                            )
                            .min_size(Vec2::new(112.0, 34.0))
                            .fill(pal.bg_card_alt)
                            .stroke(Stroke::new(1.0, pal.border))
                            .rounding(Rounding::same(8.0)),
                        )
                        .clicked()
                    {
                        self.show_advanced = !self.show_advanced;
                    }
                });
            });

            if self.show_advanced {
                ui.add_space(12.0);
                egui::Frame::none()
                    .fill(pal.bg_card_alt)
                    .rounding(Rounding::same(10.0))
                    .stroke(Stroke::new(1.0, pal.border))
                    .inner_margin(Margin::same(14.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let mut changed = false;

                        ui.horizontal_wrapped(|ui| {
                            changed |= ui
                                .checkbox(
                                    &mut self.config.options.embed_thumbnail,
                                    "Embed Thumbnail",
                                )
                                .changed();
                            ui.add_space(10.0);
                            changed |= ui
                                .checkbox(
                                    &mut self.config.options.embed_metadata,
                                    "Embed Metadata",
                                )
                                .changed();
                            ui.add_space(10.0);
                            changed |= ui
                                .checkbox(
                                    &mut self.config.options.sponsorblock,
                                    "SponsorBlock (Skip Ads)",
                                )
                                .changed();
                            ui.add_space(10.0);
                            changed |= ui
                                .checkbox(
                                    &mut self.config.options.download_playlist,
                                    "Allow Playlist",
                                )
                                .changed();
                        });

                        ui.add_space(10.0);

                        ui.horizontal_wrapped(|ui| {
                            changed |= ui
                                .checkbox(
                                    &mut self.config.options.download_subtitles,
                                    "Subtitles:",
                                )
                                .changed();
                            ui.add_enabled_ui(self.config.options.download_subtitles, |ui| {
                                changed |= ui
                                    .add_sized(
                                        [80.0, 24.0],
                                        egui::TextEdit::singleline(
                                            &mut self.config.options.subtitle_langs,
                                        )
                                        .hint_text("th,en"),
                                    )
                                    .changed();
                            });

                            ui.add_space(16.0);

                            changed |= ui
                                .checkbox(&mut self.config.options.trim_enabled, "Clip Range:")
                                .changed();
                            ui.add_enabled_ui(self.config.options.trim_enabled, |ui| {
                                changed |= ui
                                    .add_sized(
                                        [66.0, 24.0],
                                        egui::TextEdit::singleline(
                                            &mut self.config.options.trim_start,
                                        )
                                        .hint_text("00:00"),
                                    )
                                    .changed();
                                ui.label(RichText::new("to").size(12.0).color(pal.text_muted));
                                changed |= ui
                                    .add_sized(
                                        [66.0, 24.0],
                                        egui::TextEdit::singleline(
                                            &mut self.config.options.trim_end,
                                        )
                                        .hint_text("01:00"),
                                    )
                                    .changed();
                            });
                        });

                        ui.add_space(10.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new("Speed Limit:")
                                    .size(12.5)
                                    .color(pal.text_secondary),
                            );
                            egui::ComboBox::from_id_salt("speed_limit")
                                .width(110.0)
                                .selected_text(self.config.options.speed_limit.label())
                                .show_ui(ui, |ui| {
                                    for s in SpeedLimit::ALL {
                                        if ui
                                            .selectable_value(
                                                &mut self.config.options.speed_limit,
                                                *s,
                                                s.label(),
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                    }
                                });

                            ui.add_space(16.0);

                            ui.label(
                                RichText::new("Browser Cookies:")
                                    .size(12.5)
                                    .color(pal.text_secondary),
                            );
                            egui::ComboBox::from_id_salt("cookies_browser")
                                .width(135.0)
                                .selected_text(self.config.options.cookies_browser.label())
                                .show_ui(ui, |ui| {
                                    for b in BrowserCookies::ALL {
                                        if ui
                                            .selectable_value(
                                                &mut self.config.options.cookies_browser,
                                                *b,
                                                b.label(),
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                    }
                                });
                        });

                        if changed {
                            config::save_config(&self.config);
                        }
                    });
            }
        });
    }

    fn render_queue_section(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let pal = self.pal;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Download Queue ({})", self.tasks.len()))
                    .size(16.0)
                    .strong()
                    .color(pal.text_primary),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let has_done = self.tasks.iter().any(|t| {
                    matches!(
                        t.status,
                        TaskStatus::Completed | TaskStatus::Failed(_) | TaskStatus::Cancelled
                    )
                });
                if has_done
                    && ui
                        .add(
                            egui::Button::new(
                                RichText::new("Clear Finished")
                                    .size(12.0)
                                    .color(pal.text_secondary),
                            )
                            .fill(pal.bg_card)
                            .stroke(Stroke::new(1.0, pal.border))
                            .rounding(Rounding::same(7.0)),
                        )
                        .clicked()
                {
                    self.tasks.retain(|t| {
                        matches!(
                            t.status,
                            TaskStatus::Queued | TaskStatus::Downloading | TaskStatus::Processing(_)
                        )
                    });
                }
            });
        });

        ui.add_space(10.0);

        if self.tasks.is_empty() {
            card_frame(&pal).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new("No active downloads")
                            .size(15.0)
                            .strong()
                            .color(pal.text_secondary),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Paste a YouTube URL above or enable Clipboard Watch to begin.")
                            .size(12.5)
                            .color(pal.text_muted),
                    );
                    ui.add_space(20.0);
                });
            });
            return;
        }

        let mut retry_ids = Vec::new();
        let mut remove_ids = Vec::new();

        for task in &self.tasks {
            card_frame(&pal).show(ui, |ui| {
                ui.set_width(ui.available_width());

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&task.title)
                                .size(14.0)
                                .strong()
                                .color(pal.text_primary),
                        );
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&task.uploader).size(11.5).color(pal.text_muted));
                            ui.label(RichText::new("•").size(11.5).color(pal.text_muted));
                            ui.label(
                                RichText::new(task.options.summary_badge())
                                    .size(11.5)
                                    .color(pal.blue),
                            );
                        });
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        match &task.status {
                            TaskStatus::Downloading
                            | TaskStatus::Processing(_)
                            | TaskStatus::Queued => {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("✕ Cancel").size(12.0).color(pal.red),
                                        )
                                        .fill(pal.red_soft)
                                        .rounding(Rounding::same(6.0)),
                                    )
                                    .clicked()
                                {
                                    task.cancel_flag.store(true, Ordering::Relaxed);
                                }
                            }
                            TaskStatus::Completed => {
                                if ui
                                    .add(
                                        egui::Button::new(RichText::new("✕").size(12.0))
                                            .fill(pal.bg_card_alt)
                                            .rounding(Rounding::same(6.0)),
                                    )
                                    .clicked()
                                {
                                    remove_ids.push(task.id.clone());
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("📂 Reveal")
                                                .size(12.0)
                                                .color(pal.text_primary),
                                        )
                                        .fill(pal.bg_card_alt)
                                        .stroke(Stroke::new(1.0, pal.border))
                                        .rounding(Rounding::same(6.0)),
                                    )
                                    .clicked()
                                {
                                    let p = task
                                        .output_file
                                        .as_deref()
                                        .unwrap_or(&task.options.output_dir);
                                    ytdlp::open_in_finder(p, true);
                                }
                                if let Some(file) = &task.output_file {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("▶ Play")
                                                    .size(12.0)
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(pal.green)
                                            .rounding(Rounding::same(6.0)),
                                        )
                                        .clicked()
                                    {
                                        ytdlp::open_file_default(file);
                                    }
                                }
                            }
                            TaskStatus::Failed(_) | TaskStatus::Cancelled => {
                                if ui
                                    .add(
                                        egui::Button::new(RichText::new("✕").size(12.0))
                                            .fill(pal.bg_card_alt)
                                            .rounding(Rounding::same(6.0)),
                                    )
                                    .clicked()
                                {
                                    remove_ids.push(task.id.clone());
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("↻ Retry")
                                                .size(12.0)
                                                .color(pal.text_primary),
                                        )
                                        .fill(pal.bg_card_alt)
                                        .stroke(Stroke::new(1.0, pal.border))
                                        .rounding(Rounding::same(6.0)),
                                    )
                                    .clicked()
                                {
                                    retry_ids.push(task.id.clone());
                                }
                            }
                        }
                    });
                });

                ui.add_space(10.0);

                let bar_color = match &task.status {
                    TaskStatus::Completed => pal.green,
                    TaskStatus::Failed(_) => pal.red,
                    TaskStatus::Cancelled => pal.text_muted,
                    TaskStatus::Processing(_) => pal.amber,
                    _ => pal.accent,
                };
                let is_indeterminate = matches!(task.status, TaskStatus::Processing(_))
                    || (task.status == TaskStatus::Downloading && task.progress <= 0.001);
                draw_progress_bar(ui, ctx, &pal, task.progress, bar_color, is_indeterminate);

                ui.add_space(6.0);

                ui.horizontal(|ui| match &task.status {
                    TaskStatus::Queued => {
                        ui.label(RichText::new("⏳ Queued...").size(12.0).color(pal.text_muted));
                    }
                    TaskStatus::Downloading => {
                        if task.progress <= 0.001 {
                            ui.label(
                                RichText::new("🔄 Connecting & preparing stream...")
                                    .size(12.0)
                                    .strong()
                                    .color(pal.accent),
                            );
                        } else {
                            pill_badge(
                                ui,
                                &format!("{:.1}%", task.progress * 100.0),
                                pal.accent_soft,
                                pal.accent,
                            );
                            ui.label(
                                RichText::new(format!(
                                    "  {}  •  ⚡ {}  •  ETA {}",
                                    task.total_size, task.speed, task.eta
                                ))
                                .size(12.0)
                                .color(pal.text_secondary),
                            );
                        }
                    }
                    TaskStatus::Processing(stage) => {
                        ui.label(
                            RichText::new(format!("⚙ {stage}"))
                                .size(12.0)
                                .strong()
                                .color(pal.amber),
                        );
                    }
                    TaskStatus::Completed => {
                        ui.label(
                            RichText::new("✔ Completed")
                                .size(12.0)
                                .strong()
                                .color(pal.green),
                        );
                        if let Some(p) = &task.output_file {
                            if let Some(name) = p.file_name() {
                                ui.label(
                                    RichText::new(format!("  •  {}", name.to_string_lossy()))
                                        .size(11.5)
                                        .color(pal.text_muted),
                                );
                            }
                        }
                    }
                    TaskStatus::Failed(err) => {
                        ui.label(RichText::new(format!("✖ Failed: {err}")).size(12.0).color(pal.red));
                    }
                    TaskStatus::Cancelled => {
                        ui.label(RichText::new("Cancelled").size(12.0).color(pal.text_muted));
                    }
                });
            });
            ui.add_space(8.0);
        }

        if !remove_ids.is_empty() {
            self.tasks.retain(|t| !remove_ids.contains(&t.id));
        }

        if !retry_ids.is_empty() {
            for t in &mut self.tasks {
                if retry_ids.contains(&t.id) {
                    t.status = TaskStatus::Queued;
                    t.progress = 0.0;
                    t.cancel_flag.store(false, Ordering::Relaxed);
                }
            }
            self.dispatch_queued_tasks(ctx);
        }
    }

    fn render_tools_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let pal = self.pal;
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.label(
                    RichText::new("FFmpeg Media Tools")
                        .size(22.0)
                        .strong()
                        .color(pal.text_primary),
                );
                ui.label(
                    RichText::new(
                        "Convert formats, compress heavy videos, trim clips, or extract audio locally.",
                    )
                    .size(12.5)
                    .color(pal.text_secondary),
                );
                ui.add_space(16.0);

                // Input File Drop / Picker Card
                card_frame(&pal).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("Source Media File")
                                    .size(14.0)
                                    .strong()
                                    .color(pal.text_primary),
                            );
                            if let Some(file) = &self.media_tool.input_file {
                                let name = file
                                    .file_name()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_else(|| file.display().to_string());
                                let size_str = std::fs::metadata(file)
                                    .map(|m| format_bytes(m.len()))
                                    .unwrap_or_default();
                                ui.label(
                                    RichText::new(format!("🎬 {name}  ({size_str})"))
                                        .size(13.0)
                                        .color(pal.blue),
                                );
                            } else {
                                ui.label(
                                    RichText::new("Drag & drop any video/audio file here, or click Choose File")
                                        .size(12.5)
                                        .color(pal.text_muted),
                                );
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("📂 Choose File")
                                            .size(13.0)
                                            .color(Color32::WHITE),
                                    )
                                    .min_size(Vec2::new(124.0, 36.0))
                                    .fill(pal.blue)
                                    .rounding(Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                if let Some(file) = rfd::FileDialog::new()
                                    .set_directory(&self.config.options.output_dir)
                                    .pick_file()
                                {
                                    self.media_tool.input_file = Some(file);
                                    self.media_tool.last_error = None;
                                    self.media_tool.last_output = None;
                                }
                            }
                        });
                    });
                });

                ui.add_space(14.0);

                // Tool Mode & Settings Card
                card_frame(&pal).show(ui, |ui| {
                    ui.set_width(ui.available_width());

                    // Mode selector buttons
                    ui.horizontal_wrapped(|ui| {
                        for mode in MediaToolMode::ALL {
                            let selected = self.media_tool.mode == *mode;
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(mode.label())
                                            .size(13.0)
                                            .strong()
                                            .color(if selected {
                                                Color32::WHITE
                                            } else {
                                                pal.text_secondary
                                            }),
                                    )
                                    .min_size(Vec2::new(138.0, 36.0))
                                    .fill(if selected { pal.accent } else { pal.bg_card_alt })
                                    .stroke(Stroke::new(
                                        1.0,
                                        if selected { pal.accent } else { pal.border },
                                    ))
                                    .rounding(Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                self.media_tool.mode = *mode;
                            }
                        }
                    });

                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(14.0);

                    match self.media_tool.mode {
                        MediaToolMode::Convert => {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Target Format:")
                                        .size(13.0)
                                        .color(pal.text_secondary),
                                );
                                egui::ComboBox::from_id_salt("tool_convert_target")
                                    .width(220.0)
                                    .selected_text(self.media_tool.convert_target.label())
                                    .show_ui(ui, |ui| {
                                        for t in ConvertTarget::ALL {
                                            ui.selectable_value(
                                                &mut self.media_tool.convert_target,
                                                *t,
                                                t.label(),
                                            );
                                        }
                                    });
                            });
                        }
                        MediaToolMode::Compress => {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new("Compression Level:")
                                        .size(13.0)
                                        .color(pal.text_secondary),
                                );
                                egui::ComboBox::from_id_salt("tool_compress_q")
                                    .width(210.0)
                                    .selected_text(self.media_tool.compress_quality.label())
                                    .show_ui(ui, |ui| {
                                        for q in CompressQuality::ALL {
                                            ui.selectable_value(
                                                &mut self.media_tool.compress_quality,
                                                *q,
                                                q.label(),
                                            );
                                        }
                                    });

                                ui.add_space(14.0);

                                ui.label(
                                    RichText::new("Resolution:")
                                        .size(13.0)
                                        .color(pal.text_secondary),
                                );
                                egui::ComboBox::from_id_salt("tool_compress_scale")
                                    .width(165.0)
                                    .selected_text(self.media_tool.compress_scale.label())
                                    .show_ui(ui, |ui| {
                                        for s in ScaleResolution::ALL {
                                            ui.selectable_value(
                                                &mut self.media_tool.compress_scale,
                                                *s,
                                                s.label(),
                                            );
                                        }
                                    });
                            });
                        }
                        MediaToolMode::Trim => {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("Start:").size(13.0).color(pal.text_secondary));
                                ui.add_sized(
                                    [82.0, 28.0],
                                    egui::TextEdit::singleline(&mut self.media_tool.trim_start)
                                        .hint_text("00:00"),
                                );
                                ui.add_space(8.0);
                                ui.label(RichText::new("End:").size(13.0).color(pal.text_secondary));
                                ui.add_sized(
                                    [82.0, 28.0],
                                    egui::TextEdit::singleline(&mut self.media_tool.trim_end)
                                        .hint_text("00:30"),
                                );
                                ui.add_space(14.0);
                                ui.checkbox(
                                    &mut self.media_tool.trim_fast_copy,
                                    "Fast Stream Copy (instant, no quality loss)",
                                );
                            });
                        }
                        MediaToolMode::ExtractAudio => {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Audio Output Format:")
                                        .size(13.0)
                                        .color(pal.text_secondary),
                                );
                                egui::ComboBox::from_id_salt("tool_extract_fmt")
                                    .width(140.0)
                                    .selected_text(self.media_tool.extract_format.label())
                                    .show_ui(ui, |ui| {
                                        for f in AudioFormat::ALL {
                                            ui.selectable_value(
                                                &mut self.media_tool.extract_format,
                                                *f,
                                                f.label(),
                                            );
                                        }
                                    });
                            });
                        }
                    }

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Output folder: {}",
                                shorten_path(&self.config.options.output_dir, 40)
                            ))
                            .size(12.0)
                            .color(pal.text_muted),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let can_run = self.media_tool.input_file.is_some()
                                && !self.media_tool.running
                                && self.deps.as_ref().and_then(|d| d.ffmpeg_path.as_ref()).is_some();
                            let label = if self.media_tool.running {
                                "⏳ Processing with FFmpeg..."
                            } else {
                                "⚡ Run FFmpeg"
                            };
                            if ui
                                .add_enabled(
                                    can_run,
                                    egui::Button::new(
                                        RichText::new(label)
                                            .size(14.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .min_size(Vec2::new(160.0, 38.0))
                                    .fill(pal.accent)
                                    .rounding(Rounding::same(9.0)),
                                )
                                .clicked()
                            {
                                if let Some(ffmpeg) =
                                    self.deps.as_ref().and_then(|d| d.ffmpeg_path.clone())
                                {
                                    self.media_tool.running = true;
                                    self.media_tool.last_error = None;
                                    self.media_tool.last_output = None;
                                    let state_clone = self.media_tool.clone();
                                    let out_dir = self.config.options.output_dir.clone();
                                    let tx = self.tx.clone();
                                    let ctx_c = ctx.clone();
                                    thread::spawn(move || {
                                        ffmpeg_tools::run_media_tool(
                                            ffmpeg,
                                            state_clone,
                                            out_dir,
                                            tx,
                                            ctx_c,
                                        );
                                    });
                                }
                            }
                        });
                    });

                    if let Some(err) = &self.media_tool.last_error {
                        ui.add_space(10.0);
                        ui.label(RichText::new(format!("✖ {err}")).size(12.5).color(pal.red));
                    }

                    if let Some(out) = &self.media_tool.last_output {
                        ui.add_space(12.0);
                        egui::Frame::none()
                            .fill(pal.bg_card_alt)
                            .rounding(Rounding::same(9.0))
                            .stroke(Stroke::new(1.0, pal.green.gamma_multiply(0.7)))
                            .inner_margin(Margin::same(12.0))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    let name = out
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| out.display().to_string());
                                    ui.label(
                                        RichText::new(format!("✔ Ready: {name}"))
                                            .size(13.0)
                                            .strong()
                                            .color(pal.green),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new("📂 Reveal").size(12.0),
                                                    )
                                                    .fill(pal.bg_card)
                                                    .stroke(Stroke::new(1.0, pal.border))
                                                    .rounding(Rounding::same(6.0)),
                                                )
                                                .clicked()
                                            {
                                                ytdlp::open_in_finder(out, true);
                                            }
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new("▶ Play")
                                                            .size(12.0)
                                                            .color(Color32::WHITE),
                                                    )
                                                    .fill(pal.green)
                                                    .rounding(Rounding::same(6.0)),
                                                )
                                                .clicked()
                                            {
                                                ytdlp::open_file_default(out);
                                            }
                                        },
                                    );
                                });
                            });
                    }
                });
            });
    }

    fn render_history_tab(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Download History")
                    .size(22.0)
                    .strong()
                    .color(pal.text_primary),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !self.history.is_empty()
                    && ui
                        .add(
                            egui::Button::new(
                                RichText::new("🗑 Clear History").size(12.5).color(pal.red),
                            )
                            .fill(pal.red_soft)
                            .rounding(Rounding::same(8.0)),
                        )
                        .clicked()
                {
                    self.history.clear();
                    config::save_history(&self.history);
                }
            });
        });

        ui.add_space(14.0);

        if self.history.is_empty() {
            card_frame(&pal).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.add_space(36.0);
                    ui.label(
                        RichText::new("No completed downloads yet")
                            .size(15.0)
                            .strong()
                            .color(pal.text_secondary),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Your finished videos and audio files will appear here.")
                            .size(12.5)
                            .color(pal.text_muted),
                    );
                    ui.add_space(36.0);
                });
            });
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let mut copied_url: Option<String> = None;
                for item in &self.history {
                    card_frame(&pal).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(&item.title)
                                        .size(14.0)
                                        .strong()
                                        .color(pal.text_primary),
                                );
                                ui.add_space(2.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&item.uploader)
                                            .size(12.0)
                                            .color(pal.text_secondary),
                                    );
                                    ui.label(RichText::new("•").size(12.0).color(pal.text_muted));
                                    ui.label(
                                        RichText::new(&item.format_badge)
                                            .size(12.0)
                                            .color(pal.blue),
                                    );
                                    ui.label(RichText::new("•").size(12.0).color(pal.text_muted));
                                    ui.label(
                                        RichText::new(
                                            item.completed_at.format("%Y-%m-%d %H:%M").to_string(),
                                        )
                                        .size(11.5)
                                        .color(pal.text_muted),
                                    );
                                });
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(RichText::new("🔗 Copy URL").size(12.0))
                                            .fill(pal.bg_card_alt)
                                            .stroke(Stroke::new(1.0, pal.border))
                                            .rounding(Rounding::same(6.0)),
                                    )
                                    .clicked()
                                {
                                    copied_url = Some(item.url.clone());
                                }
                                if ui
                                    .add(
                                        egui::Button::new(RichText::new("📂 Reveal").size(12.0))
                                            .fill(pal.bg_card_alt)
                                            .stroke(Stroke::new(1.0, pal.border))
                                            .rounding(Rounding::same(6.0)),
                                    )
                                    .clicked()
                                {
                                    ytdlp::open_in_finder(&item.output_path, true);
                                }
                                if item.output_path.is_file()
                                    && ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("▶ Play")
                                                    .size(12.0)
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(pal.green)
                                            .rounding(Rounding::same(6.0)),
                                        )
                                        .clicked()
                                {
                                    ytdlp::open_file_default(&item.output_path);
                                }
                            });
                        });
                    });
                    ui.add_space(8.0);
                }

                if let Some(u) = copied_url {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        let _ = cb.set_text(u);
                        self.set_toast("Copied URL to clipboard");
                    }
                }
            });
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let pal = self.pal;
        let mut theme_changed = false;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Settings & Engine Logs")
                        .size(22.0)
                        .strong()
                        .color(pal.text_primary),
                );
                ui.add_space(14.0);

                // Appearance & Theme Gallery Card
                card_frame(&pal).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("🎨 Appearance & Themes")
                            .size(15.5)
                            .strong()
                            .color(pal.text_primary),
                    );
                    ui.label(
                        RichText::new("Pick a curated studio theme and optional accent color.")
                            .size(12.0)
                            .color(pal.text_muted),
                    );
                    ui.add_space(12.0);

                    // Visual Theme Preview Cards Grid (4 per row)
                    egui::Grid::new("theme_gallery_grid")
                        .spacing(Vec2::new(10.0, 10.0))
                        .show(ui, |ui| {
                            for (idx, theme_opt) in AppTheme::ALL.iter().enumerate() {
                                let preview = Palette::from_config(*theme_opt, AccentColor::ThemeDefault);
                                let selected = self.config.theme == *theme_opt;
                                let card_w = ((ui.available_width() - 30.0) / 4.0).clamp(150.0, 220.0);
                                let (rect, resp) = ui.allocate_exact_size(
                                    Vec2::new(card_w, 78.0),
                                    Sense::click(),
                                );

                                // Outer card background
                                let border_stroke = if selected {
                                    Stroke::new(2.2, pal.accent)
                                } else if resp.hovered() {
                                    Stroke::new(1.5, preview.accent.gamma_multiply(0.8))
                                } else {
                                    Stroke::new(1.0, preview.border)
                                };
                                ui.painter().rect(
                                    rect,
                                    Rounding::same(10.0),
                                    preview.bg_main,
                                    border_stroke,
                                );

                                // Mini sidebar strip on left
                                let sb_rect = egui::Rect::from_min_size(
                                    rect.min + Vec2::new(6.0, 6.0),
                                    Vec2::new(26.0, 40.0),
                                );
                                ui.painter().rect_filled(sb_rect, Rounding::same(5.0), preview.bg_sidebar);

                                // Mini accent dot in sidebar
                                ui.painter().circle_filled(
                                    sb_rect.min + Vec2::new(13.0, 11.0),
                                    4.5,
                                    preview.accent,
                                );

                                // Mini content card on right
                                let mc_rect = egui::Rect::from_min_max(
                                    rect.min + Vec2::new(38.0, 6.0),
                                    egui::pos2(rect.max.x - 6.0, rect.min.y + 46.0),
                                );
                                ui.painter().rect_filled(mc_rect, Rounding::same(5.0), preview.bg_card);

                                // Mini accent progress bar inside content card
                                let bar_rect = egui::Rect::from_min_size(
                                    mc_rect.min + Vec2::new(8.0, 26.0),
                                    Vec2::new((mc_rect.width() - 16.0) * 0.68, 5.0),
                                );
                                ui.painter().rect_filled(bar_rect, Rounding::same(2.5), preview.accent);

                                // Mini text lines
                                let line1 = egui::Rect::from_min_size(
                                    mc_rect.min + Vec2::new(8.0, 9.0),
                                    Vec2::new((mc_rect.width() - 16.0) * 0.75, 4.0),
                                );
                                ui.painter().rect_filled(line1, Rounding::same(2.0), preview.text_primary.gamma_multiply(0.75));
                                let line2 = egui::Rect::from_min_size(
                                    mc_rect.min + Vec2::new(8.0, 17.0),
                                    Vec2::new((mc_rect.width() - 16.0) * 0.48, 3.5),
                                );
                                ui.painter().rect_filled(line2, Rounding::same(2.0), preview.text_secondary.gamma_multiply(0.55));

                                // Theme name label at bottom
                                let title = if selected {
                                    format!("✔ {} {}", theme_opt.emoji(), theme_opt.label())
                                } else {
                                    format!("{} {}", theme_opt.emoji(), theme_opt.label())
                                };
                                ui.painter().text(
                                    egui::pos2(rect.center().x, rect.max.y - 14.0),
                                    egui::Align2::CENTER_CENTER,
                                    title,
                                    FontId::proportional(11.5),
                                    preview.text_primary,
                                );

                                if resp.clicked() {
                                    self.config.theme = *theme_opt;
                                    self.config.accent_color = AccentColor::ThemeDefault;
                                    theme_changed = true;
                                }

                                if (idx + 1) % 4 == 0 {
                                    ui.end_row();
                                }
                            }
                        });

                    ui.add_space(14.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("Accent Override:")
                                .size(12.5)
                                .color(pal.text_secondary),
                        );
                        for accent in AccentColor::ALL {
                            let preview = Palette::from_config(self.config.theme, *accent);
                            let selected = self.config.accent_color == *accent;
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(accent.label())
                                            .size(11.5)
                                            .color(Color32::WHITE),
                                    )
                                    .fill(preview.accent)
                                    .stroke(Stroke::new(
                                        if selected { 2.5 } else { 0.0 },
                                        pal.text_primary,
                                    ))
                                    .rounding(Rounding::same(14.0)),
                                )
                                .clicked()
                            {
                                self.config.accent_color = *accent;
                                theme_changed = true;
                            }
                        }
                    });
                });

                ui.add_space(14.0);

                // General settings card
                card_frame(&pal).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("General Preferences")
                            .size(15.0)
                            .strong()
                            .color(pal.text_primary),
                    );
                    ui.add_space(10.0);

                    let mut changed = false;

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Simultaneous Downloads:")
                                .size(13.0)
                                .color(pal.text_secondary),
                        );
                        changed |= ui
                            .add(egui::Slider::new(&mut self.config.max_concurrent, 1..=4))
                            .changed();
                    });

                    ui.add_space(6.0);
                    changed |= ui
                        .checkbox(
                            &mut self.config.auto_fetch_on_paste,
                            "Automatically fetch video info when clicking Paste",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut self.config.clipboard_watch,
                            "Auto-watch clipboard for YouTube URLs",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut self.config.notify_on_complete,
                            "Show macOS Notification with sound when finished",
                        )
                        .changed();

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Filename Template:")
                                .size(13.0)
                                .color(pal.text_secondary),
                        );
                        changed |= ui
                            .add_sized(
                                [300.0, 28.0],
                                egui::TextEdit::singleline(
                                    &mut self.config.options.filename_template,
                                ),
                            )
                            .changed();
                    });

                    if changed {
                        config::save_config(&self.config);
                    }
                });

                ui.add_space(14.0);

                // Engine Diagnostics + One-Click Update Card
                card_frame(&pal).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("System Dependencies")
                                .size(15.0)
                                .strong()
                                .color(pal.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("↻ Re-check").size(12.0))
                                        .fill(pal.bg_card_alt)
                                        .stroke(Stroke::new(1.0, pal.border))
                                        .rounding(Rounding::same(6.0)),
                                )
                                .clicked()
                            {
                                let tx = self.tx.clone();
                                let ctx_c = ctx.clone();
                                thread::spawn(move || {
                                    let st = ytdlp::check_dependencies();
                                    let _ = tx.send(WorkerMessage::DependenciesChecked(st));
                                    ctx_c.request_repaint();
                                });
                            }

                            let update_label = if self.updating_ytdlp {
                                "⏳ Updating yt-dlp..."
                            } else {
                                "⬆ Update yt-dlp"
                            };
                            let has_ytdlp = self
                                .deps
                                .as_ref()
                                .and_then(|d| d.ytdlp_path.as_ref())
                                .is_some();
                            if ui
                                .add_enabled(
                                    has_ytdlp && !self.updating_ytdlp,
                                    egui::Button::new(
                                        RichText::new(update_label)
                                            .size(12.0)
                                            .color(Color32::WHITE),
                                    )
                                    .fill(pal.blue)
                                    .rounding(Rounding::same(6.0)),
                                )
                                .clicked()
                            {
                                if let Some(path) =
                                    self.deps.as_ref().and_then(|d| d.ytdlp_path.clone())
                                {
                                    self.updating_ytdlp = true;
                                    let tx = self.tx.clone();
                                    let ctx_c = ctx.clone();
                                    thread::spawn(move || {
                                        let res = ytdlp::update_ytdlp(&path);
                                        let _ = tx.send(WorkerMessage::YtdlpUpdateFinished(res));
                                        ctx_c.request_repaint();
                                    });
                                }
                            }
                        });
                    });

                    ui.add_space(8.0);
                    if let Some(deps) = &self.deps {
                        ui.label(
                            RichText::new(format!(
                                "yt-dlp: {} ({})",
                                deps.ytdlp_version.as_deref().unwrap_or("Not Found"),
                                deps.ytdlp_path
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|| "Install via: brew install yt-dlp".to_string())
                            ))
                            .size(12.5)
                            .color(pal.text_secondary),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(format!(
                                "ffmpeg: {} ({})",
                                deps.ffmpeg_version.as_deref().unwrap_or("Not Found"),
                                deps.ffmpeg_path
                                    .as_ref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|| "Install via: brew install ffmpeg".to_string())
                            ))
                            .size(12.5)
                            .color(pal.text_secondary),
                        );
                    }
                });

                ui.add_space(14.0);

                // Live Console Log
                card_frame(&pal).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Engine Console Output (yt-dlp / ffmpeg)")
                                .size(15.0)
                                .strong()
                                .color(pal.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("Clear Logs").size(12.0))
                                        .fill(pal.bg_card_alt)
                                        .rounding(Rounding::same(6.0)),
                                )
                                .clicked()
                            {
                                self.logs.clear();
                            }
                        });
                    });
                    ui.add_space(8.0);

                    egui::Frame::none()
                        .fill(pal.bg_main)
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            egui::ScrollArea::vertical()
                                .max_height(220.0)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    if self.logs.is_empty() {
                                        ui.label(
                                            RichText::new("No logs yet.")
                                                .monospace()
                                                .size(11.5)
                                                .color(pal.text_muted),
                                        );
                                    } else {
                                        for line in &self.logs {
                                            ui.label(
                                                RichText::new(line)
                                                    .monospace()
                                                    .size(11.5)
                                                    .color(pal.text_secondary),
                                            );
                                        }
                                    }
                                });
                        });
                });
            });

        if theme_changed {
            self.refresh_theme(ctx);
        }
    }
}

impl eframe::App for RustTubeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_messages(ctx);
        self.poll_clipboard_watcher(ctx);
        self.handle_dropped_files(ctx);

        self.render_sidebar(ctx);

        let pal = self.pal;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_main)
                    .inner_margin(Margin::symmetric(24.0, 20.0)),
            )
            .show(ctx, |ui| match self.active_tab {
                ActiveTab::Download => self.render_download_tab(ui, ctx),
                ActiveTab::Tools => self.render_tools_tab(ui, ctx),
                ActiveTab::History => self.render_history_tab(ui),
                ActiveTab::Settings => self.render_settings_tab(ui, ctx),
            });

        // Floating toast notification
        if let Some((msg, created)) = &self.toast {
            if created.elapsed().as_secs_f32() < 2.8 {
                egui::Area::new(egui::Id::new("toast_overlay"))
                    .anchor(egui::Align2::RIGHT_BOTTOM, Vec2::new(-24.0, -24.0))
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(pal.bg_card_alt)
                            .stroke(Stroke::new(1.0, pal.accent.gamma_multiply(0.8)))
                            .rounding(Rounding::same(10.0))
                            .inner_margin(Margin::symmetric(14.0, 10.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new(msg).size(13.0).color(pal.text_primary));
                            });
                    });
                ctx.request_repaint_after(Duration::from_millis(120));
            } else {
                self.toast = None;
            }
        }
    }
}

fn card_frame(pal: &Palette) -> egui::Frame {
    egui::Frame::none()
        .fill(pal.bg_card)
        .rounding(Rounding::same(12.0))
        .stroke(Stroke::new(1.0, pal.border))
        .inner_margin(Margin::same(16.0))
}

fn pill_badge(ui: &mut egui::Ui, text: &str, bg: Color32, fg: Color32) {
    egui::Frame::none()
        .fill(bg)
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::symmetric(9.0, 4.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).color(fg));
        });
}

fn status_dot_row(ui: &mut egui::Ui, pal: &Palette, name: &str, version: Option<&str>) {
    ui.horizontal(|ui| {
        let ok = version.is_some();
        let color = if ok { pal.green } else { pal.red };
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0, color);
        ui.label(
            RichText::new(name)
                .size(12.0)
                .strong()
                .color(pal.text_primary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(version.unwrap_or("Missing"))
                    .size(11.0)
                    .color(if ok { pal.text_secondary } else { pal.red }),
            );
        });
    });
}

fn draw_progress_bar(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    pal: &Palette,
    progress: f32,
    color: Color32,
    indeterminate: bool,
) {
    let desired = Vec2::new(ui.available_width(), 9.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(4.5), pal.bg_card_alt);

    if indeterminate {
        let t = ctx.input(|i| i.time) as f32;
        let seg_w = (rect.width() * 0.28).max(40.0);
        let travel = rect.width() + seg_w;
        let x_start = rect.left() - seg_w + ((t * 1.15) % 1.0) * travel;
        let fill_rect = egui::Rect::from_min_max(
            egui::pos2(x_start.max(rect.left()), rect.top()),
            egui::pos2((x_start + seg_w).min(rect.right()), rect.bottom()),
        );
        if fill_rect.width() > 2.0 {
            ui.painter()
                .rect_filled(fill_rect, Rounding::same(4.5), color);
        }
        ctx.request_repaint_after(Duration::from_millis(33));
    } else {
        let p = progress.clamp(0.0, 1.0);
        if p > 0.0 {
            let mut fill_rect = rect;
            fill_rect.set_width((rect.width() * p).max(9.0));
            ui.painter()
                .rect_filled(fill_rect, Rounding::same(4.5), color);
        }
    }
}

fn shorten_path(path: &PathBuf, max_chars: usize) -> String {
    let s = path.display().to_string();
    if s.chars().count() <= max_chars {
        s
    } else {
        let tail: String = s
            .chars()
            .rev()
            .take(max_chars.saturating_sub(3))
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("...{tail}")
    }
}
