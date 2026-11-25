use crate::models::{
    DependencyStatus, DownloadOptions, FormatEstimate, MediaMode, PlaylistEntry, VideoInfo,
    WorkerMessage,
};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc,
};
use std::thread;
use std::time::Duration;

/// Check whether a string looks like a YouTube video/shorts/playlist URL.
pub fn is_youtube_url(s: &str) -> bool {
    let t = s.trim();
    if !(t.starts_with("http://") || t.starts_with("https://")) {
        return false;
    }
    let lower = t.to_lowercase();
    lower.contains("youtube.com/watch")
        || lower.contains("youtu.be/")
        || lower.contains("youtube.com/shorts/")
        || lower.contains("youtube.com/playlist")
        || lower.contains("music.youtube.com/")
}

/// Collect all existing candidate paths for a binary across PATH and common macOS locations.
fn find_all_binaries(name: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();

    if let Ok(out) = Command::new("which").args(["-a", name]).output() {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let p = line.trim();
                if !p.is_empty() {
                    let pb = PathBuf::from(p);
                    if pb.exists() && !paths.contains(&pb) {
                        paths.push(pb);
                    }
                }
            }
        }
    }

    let mut candidates = vec![
        PathBuf::from(format!("/opt/homebrew/bin/{name}")),
        PathBuf::from(format!("/usr/local/bin/{name}")),
        PathBuf::from(format!("/opt/anaconda3/bin/{name}")),
        PathBuf::from(format!("/opt/miniconda3/bin/{name}")),
        PathBuf::from(format!("/usr/bin/{name}")),
    ];

    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(format!(".local/bin/{name}")));
        candidates.push(home.join(format!("anaconda3/bin/{name}")));
        candidates.push(home.join(format!("miniconda3/bin/{name}")));
    }

    for p in candidates {
        if p.exists() && !paths.contains(&p) {
            paths.push(p);
        }
    }

    paths
}

pub fn find_binary(name: &str) -> Option<PathBuf> {
    find_all_binaries(name).into_iter().next()
}

/// Automatically pick the newest yt-dlp installation (YYYY.MM.DD) across Homebrew/Anaconda/etc.
fn find_newest_ytdlp() -> (Option<PathBuf>, Option<String>) {
    let candidates = find_all_binaries("yt-dlp");
    let mut best: Option<(PathBuf, String)> = None;

    for path in candidates {
        let ver = Command::new(&path)
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty());

        if let Some(v) = ver {
            let is_newer = match &best {
                None => true,
                Some((_, best_v)) => v > *best_v,
            };
            if is_newer {
                best = Some((path, v));
            }
        }
    }

    match best {
        Some((p, v)) => (Some(p), Some(v)),
        None => (None, None),
    }
}

pub fn check_dependencies() -> DependencyStatus {
    let (ytdlp_path, ytdlp_version) = find_newest_ytdlp();

    let ffmpeg_path = find_binary("ffmpeg");
    let ffmpeg_version = ffmpeg_path.as_ref().and_then(|p| {
        Command::new(p)
            .arg("-version")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                let first = String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                first
                    .split_whitespace()
                    .nth(2)
                    .unwrap_or("installed")
                    .to_string()
            })
    });

    DependencyStatus {
        ytdlp_path,
        ytdlp_version,
        ffmpeg_path,
        ffmpeg_version,
    }
}

/// Update yt-dlp using the appropriate package manager (pip / brew / self-update).
pub fn update_ytdlp(ytdlp_path: &Path) -> Result<String, String> {
    let path_str = ytdlp_path.to_string_lossy();

    // If installed inside anaconda/miniconda/python bin, use matching pip
    if let Some(parent) = ytdlp_path.parent() {
        let pip_candidate = parent.join("pip");
        let pip3_candidate = parent.join("pip3");
        let pip_bin = if pip_candidate.exists() {
            Some(pip_candidate)
        } else if pip3_candidate.exists() {
            Some(pip3_candidate)
        } else {
            None
        };

        if !path_str.contains("/homebrew/") && pip_bin.is_some() {
            let pip = pip_bin.unwrap();
            let out = Command::new(&pip)
                .args(["install", "-U", "yt-dlp"])
                .output()
                .map_err(|e| format!("Failed to run pip: {e}"))?;
            if out.status.success() {
                let ver = Command::new(ytdlp_path)
                    .arg("--version")
                    .output()
                    .ok()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_else(|| "latest".to_string());
                return Ok(format!("Updated yt-dlp to {ver} via pip"));
            }
        }
    }

    // If installed via Homebrew
    if path_str.contains("/homebrew/") || path_str.contains("/Cellar/") {
        if let Some(brew) = find_binary("brew") {
            let out = Command::new(&brew)
                .args(["upgrade", "yt-dlp"])
                .output()
                .map_err(|e| format!("Failed to run brew: {e}"))?;
            if out.status.success() {
                let ver = Command::new(ytdlp_path)
                    .arg("--version")
                    .output()
                    .ok()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_else(|| "latest".to_string());
                return Ok(format!("Updated yt-dlp to {ver} via Homebrew"));
            }
        }
    }

    // Fallback: yt-dlp -U
    let out = Command::new(ytdlp_path)
        .arg("-U")
        .output()
        .map_err(|e| format!("Failed to run yt-dlp -U: {e}"))?;
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout).trim(),
        String::from_utf8_lossy(&out.stderr).trim()
    );
    if out.status.success() {
        Ok(combined.lines().last().unwrap_or("yt-dlp updated").to_string())
    } else {
        Err(combined.trim().to_string())
    }
}

pub fn fetch_video_info(
    ytdlp: &Path,
    url: &str,
    allow_playlist: bool,
    cookies: Option<&str>,
) -> Result<VideoInfo, String> {
    let url_is_explicit_playlist = url.to_lowercase().contains("/playlist?");
    let treat_as_playlist = allow_playlist || url_is_explicit_playlist;

    let mut cmd = Command::new(ytdlp);
    cmd.args(["--dump-single-json", "--no-warnings", "--skip-download"]);
    if !treat_as_playlist {
        cmd.arg("--no-playlist");
    } else {
        cmd.arg("--flat-playlist");
    }
    if let Some(browser) = cookies {
        cmd.args(["--cookies-from-browser", browser]);
    }
    cmd.arg(url);

    let out = cmd
        .output()
        .map_err(|e| format!("Failed to run yt-dlp: {e}"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Could not fetch video metadata. Check the URL and try again.".to_string()
        } else {
            stderr
        });
    }

    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("Failed to parse video metadata: {e}"))?;

    let is_playlist = json.get("_type").and_then(|v| v.as_str()) == Some("playlist");

    let mut playlist_entries = Vec::new();
    if let Some(entries) = json.get("entries").and_then(|v| v.as_array()) {
        for (i, item) in entries.iter().enumerate() {
            let entry_title = item
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string();
            let entry_url = item
                .get("url")
                .and_then(|v| v.as_str())
                .map(|u| {
                    if u.starts_with("http") {
                        u.to_string()
                    } else {
                        format!("https://www.youtube.com/watch?v={u}")
                    }
                })
                .or_else(|| {
                    item.get("id")
                        .and_then(|v| v.as_str())
                        .map(|id| format!("https://www.youtube.com/watch?v={id}"))
                })
                .unwrap_or_default();

            if !entry_url.is_empty() {
                playlist_entries.push(PlaylistEntry {
                    index: i + 1,
                    title: entry_title,
                    url: entry_url,
                    duration_secs: item.get("duration").and_then(|v| v.as_f64()).map(|d| d as u64),
                    selected: true,
                });
            }
        }
    }

    let title = json
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled Video")
        .to_string();

    let uploader = json
        .get("uploader")
        .or_else(|| json.get("channel"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Channel")
        .to_string();

    let duration_secs = json.get("duration").and_then(|v| v.as_f64()).map(|d| d as u64);
    let view_count = json.get("view_count").and_then(|v| v.as_u64());
    let upload_date = json
        .get("upload_date")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let thumbnail_url = json
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            json.get("thumbnails")
                .and_then(|t| t.as_array())
                .and_then(|arr| arr.last())
                .and_then(|item| item.get("url"))
                .and_then(|u| u.as_str())
                .map(|s| s.to_string())
        });

    let (format_estimates, audio_size_bytes) = parse_format_estimates(&json, duration_secs);

    Ok(VideoInfo {
        url: url.to_string(),
        title,
        uploader,
        duration_secs,
        view_count,
        upload_date,
        thumbnail_url,
        is_playlist,
        playlist_entries,
        format_estimates,
        audio_size_bytes,
    })
}

fn parse_format_estimates(
    json: &serde_json::Value,
    duration_secs: Option<u64>,
) -> (Vec<FormatEstimate>, Option<u64>) {
    let Some(formats) = json.get("formats").and_then(|v| v.as_array()) else {
        return (Vec::new(), None);
    };

    let mut best_audio: u64 = 0;
    let mut video_by_height: BTreeMap<u32, u64> = BTreeMap::new();

    for f in formats {
        let vcodec = f.get("vcodec").and_then(|v| v.as_str()).unwrap_or("none");
        let acodec = f.get("acodec").and_then(|v| v.as_str()).unwrap_or("none");
        let size = f
            .get("filesize")
            .and_then(|v| v.as_u64())
            .or_else(|| f.get("filesize_approx").and_then(|v| v.as_u64()))
            .or_else(|| {
                let tbr = f.get("tbr").and_then(|v| v.as_f64())?;
                let dur = duration_secs? as f64;
                Some(((tbr * 1000.0 / 8.0) * dur) as u64)
            });

        let Some(bytes) = size else { continue };
        if bytes == 0 {
            continue;
        }

        if vcodec == "none" && acodec != "none" {
            best_audio = best_audio.max(bytes);
        } else if vcodec != "none" {
            if let Some(h) = f.get("height").and_then(|v| v.as_u64()).map(|h| h as u32) {
                let bucket = normalize_height_bucket(h);
                let has_audio = acodec != "none";
                let total = if has_audio { bytes } else { bytes };
                let entry = video_by_height.entry(bucket).or_insert(0);
                if total > *entry {
                    *entry = total;
                }
            }
        }
    }

    let estimates = video_by_height
        .into_iter()
        .rev()
        .map(|(height, vbytes)| FormatEstimate {
            height,
            size_bytes: vbytes + best_audio,
        })
        .collect();

    let audio_opt = if best_audio > 0 { Some(best_audio) } else { None };
    (estimates, audio_opt)
}

fn normalize_height_bucket(h: u32) -> u32 {
    if h >= 2000 {
        2160
    } else if h >= 1300 {
        1440
    } else if h >= 1000 {
        1080
    } else if h >= 680 {
        720
    } else if h >= 440 {
        480
    } else {
        360
    }
}

pub fn fetch_thumbnail_rgba(url: &str) -> Result<(Vec<u8>, [usize; 2]), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())?;

    let bytes = client
        .get(url)
        .send()
        .map_err(|e| e.to_string())?
        .bytes()
        .map_err(|e| e.to_string())?;

    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let resized = img.thumbnail(480, 270);
    let rgba = resized.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Ok((rgba.into_raw(), size))
}

pub fn build_download_args(
    url: &str,
    opts: &DownloadOptions,
    ffmpeg_path: Option<&Path>,
) -> Vec<String> {
    let mut args = vec![
        "--newline".to_string(),
        "--progress".to_string(),
        "--no-colors".to_string(),
        "--progress-template".to_string(),
        "download:RUSTTUBE_PROG|%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s|%(progress._total_bytes_str)s|%(progress._total_bytes_estimate_str)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.fragment_index)s|%(progress.fragment_count)s".to_string(),
        "--print".to_string(),
        "after_move:RUSTTUBE_FILE|%(filepath)s".to_string(),
    ];

    if let Some(ffmpeg) = ffmpeg_path {
        if let Some(dir) = ffmpeg.parent() {
            args.push("--ffmpeg-location".to_string());
            args.push(dir.to_string_lossy().to_string());
        }
    }

    if !opts.download_playlist {
        args.push("--no-playlist".to_string());
    }

    match opts.mode {
        MediaMode::Video => {
            args.push("-f".to_string());
            args.push(opts.video_quality.format_selector(opts.video_codec));
            args.push("--merge-output-format".to_string());
            args.push(opts.video_format.ext().to_string());
        }
        MediaMode::Audio => {
            args.push("-x".to_string());
            args.push("--audio-format".to_string());
            args.push(opts.audio_format.ext().to_string());
            args.push("--audio-quality".to_string());
            args.push(opts.audio_bitrate.ytdlp_arg().to_string());
        }
    }

    if opts.download_subtitles {
        args.push("--write-subs".to_string());
        args.push("--write-auto-subs".to_string());
        let langs = if opts.subtitle_langs.trim().is_empty() {
            "th,en"
        } else {
            opts.subtitle_langs.trim()
        };
        args.push("--sub-langs".to_string());
        args.push(langs.to_string());
        if opts.mode == MediaMode::Video {
            args.push("--embed-subs".to_string());
        }
    }

    if opts.embed_thumbnail {
        args.push("--embed-thumbnail".to_string());
    }

    if opts.embed_metadata {
        args.push("--embed-metadata".to_string());
    }

    if opts.sponsorblock {
        args.push("--sponsorblock-remove".to_string());
        args.push("sponsor,selfpromo,interaction".to_string());
    }

    if opts.trim_enabled {
        let start = opts.trim_start.trim();
        let end = opts.trim_end.trim();
        if !start.is_empty() || !end.is_empty() {
            args.push("--download-sections".to_string());
            args.push(format!("*{start}-{end}"));
            args.push("--force-keyframes-at-cuts".to_string());
        }
    }

    if let Some(limit) = opts.speed_limit.ytdlp_arg() {
        args.push("--limit-rate".to_string());
        args.push(limit.to_string());
    }

    if let Some(browser) = opts.cookies_browser.ytdlp_arg() {
        args.push("--cookies-from-browser".to_string());
        args.push(browser.to_string());
    }

    let template = if opts.filename_template.trim().is_empty() {
        "%(title)s.%(ext)s"
    } else {
        opts.filename_template.trim()
    };
    let output_template = opts.output_dir.join(template);
    args.push("-o".to_string());
    args.push(output_template.to_string_lossy().to_string());

    args.push(url.to_string());
    args
}

pub fn run_download_task(
    task_id: String,
    url: String,
    options: DownloadOptions,
    ytdlp_path: PathBuf,
    ffmpeg_path: Option<PathBuf>,
    cancel_flag: Arc<AtomicBool>,
    tx: Sender<WorkerMessage>,
    ctx: eframe::egui::Context,
) {
    let _ = tx.send(WorkerMessage::TaskStarted {
        id: task_id.clone(),
    });
    ctx.request_repaint();

    let args = build_download_args(&url, &options, ffmpeg_path.as_deref());
    let _ = tx.send(WorkerMessage::TaskLog {
        id: task_id.clone(),
        line: format!("$ {} {}", ytdlp_path.display(), args.join(" ")),
    });

    let mut child = match Command::new(&ytdlp_path)
        .env("PYTHONUNBUFFERED", "1")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(WorkerMessage::TaskFinished {
                id: task_id,
                result: Err(format!("Failed to launch yt-dlp: {e}")),
                cancelled: false,
            });
            ctx.request_repaint();
            return;
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Note: when `--print` is used alongside `--progress`, yt-dlp writes
    // progress & status lines to stderr while stdout receives `--print` lines.
    // So we parse progress/stages on BOTH stderr and stdout!
    let tx_err = tx.clone();
    let id_err = task_id.clone();
    let ctx_err = ctx.clone();
    let stderr_handle = thread::spawn(move || {
        let mut last_err = String::new();
        if let Some(err) = stderr {
            let reader = BufReader::new(err);
            for line in reader.lines().map_while(Result::ok) {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed.contains("ERROR:") {
                    last_err = trimmed.to_string();
                }
                handle_output_line(trimmed, &id_err, &tx_err, &ctx_err);
            }
        }
        last_err
    });

    let mut final_file: Option<PathBuf> = None;

    let cancel_check = cancel_flag.clone();
    if let Some(out) = stdout {
        let reader = BufReader::new(out);
        for line in reader.lines().map_while(Result::ok) {
            if cancel_check.load(Ordering::Relaxed) {
                let _ = child.kill();
                break;
            }

            let trimmed = line.trim();
            if let Some(path_str) = trimmed.strip_prefix("RUSTTUBE_FILE|") {
                let p = PathBuf::from(path_str.trim());
                if !path_str.trim().is_empty() {
                    final_file = Some(p);
                }
            } else if !trimmed.is_empty() {
                handle_output_line(trimmed, &task_id, &tx, &ctx);
            }
        }
    }

    let was_cancelled = cancel_flag.load(Ordering::Relaxed);
    if was_cancelled {
        let _ = child.kill();
    }

    let status = child.wait();
    let last_err = stderr_handle.join().unwrap_or_default();

    if was_cancelled {
        let _ = tx.send(WorkerMessage::TaskFinished {
            id: task_id,
            result: Ok(None),
            cancelled: true,
        });
    } else {
        match status {
            Ok(s) if s.success() => {
                let _ = tx.send(WorkerMessage::TaskFinished {
                    id: task_id,
                    result: Ok(final_file),
                    cancelled: false,
                });
            }
            Ok(s) => {
                let msg = if !last_err.is_empty() {
                    last_err
                } else {
                    format!("yt-dlp exited with code {:?}", s.code())
                };
                let _ = tx.send(WorkerMessage::TaskFinished {
                    id: task_id,
                    result: Err(msg),
                    cancelled: false,
                });
            }
            Err(e) => {
                let _ = tx.send(WorkerMessage::TaskFinished {
                    id: task_id,
                    result: Err(format!("Process error: {e}")),
                    cancelled: false,
                });
            }
        }
    }

    ctx.request_repaint();
}

fn handle_output_line(
    trimmed: &str,
    task_id: &str,
    tx: &Sender<WorkerMessage>,
    ctx: &eframe::egui::Context,
) {
    if let Some(rest) = trimmed.strip_prefix("RUSTTUBE_PROG|") {
        let parts: Vec<&str> = rest.split('|').collect();
        if parts.len() >= 4 {
            let pct_raw = parts[0].trim().trim_end_matches('%');
            let mut pct = pct_raw.parse::<f32>().ok().map(|v| (v / 100.0).clamp(0.0, 1.0));

            // Fallback 1: downloaded_bytes / total_bytes (or estimate)
            if pct.is_none() && parts.len() >= 8 {
                let dl = parts[5].trim().parse::<f64>().ok();
                let tot = parts[6]
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .or_else(|| parts[7].trim().parse::<f64>().ok());
                if let (Some(d), Some(t)) = (dl, tot) {
                    if t > 0.0 {
                        pct = Some((d / t).clamp(0.0, 1.0) as f32);
                    }
                }
            }

            // Fallback 2: HLS fragment_index / fragment_count
            if pct.is_none() && parts.len() >= 10 {
                let fi = parts[8].trim().parse::<f64>().ok();
                let fc = parts[9].trim().parse::<f64>().ok();
                if let (Some(i), Some(c)) = (fi, fc) {
                    if c > 0.0 {
                        pct = Some((i / c).clamp(0.0, 1.0) as f32);
                    }
                }
            }

            let speed = clean_na(parts[1]);
            let eta = clean_na(parts[2]);
            let mut total_size = clean_na(parts[3]);
            if total_size == "—" && parts.len() >= 5 {
                let est = clean_na(parts[4]);
                if est != "—" {
                    total_size = format!("~{est}");
                }
            }

            let _ = tx.send(WorkerMessage::TaskProgress {
                id: task_id.to_string(),
                progress: pct.unwrap_or(0.0),
                speed,
                eta,
                total_size,
            });
            ctx.request_repaint();
        }
    } else if trimmed.starts_with("[Merger]") {
        let _ = tx.send(WorkerMessage::TaskStage {
            id: task_id.to_string(),
            stage: "Merging video & audio...".to_string(),
        });
        ctx.request_repaint();
    } else if trimmed.starts_with("[ExtractAudio]") {
        let _ = tx.send(WorkerMessage::TaskStage {
            id: task_id.to_string(),
            stage: "Extracting audio...".to_string(),
        });
        ctx.request_repaint();
    } else if trimmed.starts_with("[EmbedThumbnail]") || trimmed.starts_with("[Metadata]") {
        let _ = tx.send(WorkerMessage::TaskStage {
            id: task_id.to_string(),
            stage: "Embedding metadata & cover...".to_string(),
        });
        ctx.request_repaint();
    } else if trimmed.starts_with("[SponsorBlock]") || trimmed.starts_with("[ModifyChapters]") {
        let _ = tx.send(WorkerMessage::TaskStage {
            id: task_id.to_string(),
            stage: "Removing sponsor segments...".to_string(),
        });
        ctx.request_repaint();
    } else {
        let _ = tx.send(WorkerMessage::TaskLog {
            id: task_id.to_string(),
            line: trimmed.to_string(),
        });
    }
}

fn clean_na(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() || t == "NA" || t == "N/A" || t == "Unknown" || t == "Unknown B/s" {
        "—".to_string()
    } else {
        t.to_string()
    }
}

/// Send a native macOS Notification Center alert with sound.
pub fn send_notification(title: &str, body: &str) {
    #[cfg(target_os = "macos")]
    {
        let safe_title = title.replace('"', "\\\"");
        let safe_body = body.replace('"', "\\\"");
        let script = format!(
            "display notification \"{safe_body}\" with title \"{safe_title}\" sound name \"Glass\""
        );
        let _ = Command::new("osascript").args(["-e", &script]).spawn();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (title, body);
    }
}

pub fn open_in_finder(path: &Path, reveal: bool) {
    #[cfg(target_os = "macos")]
    {
        if reveal && path.is_file() {
            let _ = Command::new("open").args(["-R"]).arg(path).spawn();
        } else {
            let target = if path.is_file() {
                path.parent().unwrap_or(path)
            } else {
                path
            };
            let _ = Command::new("open").arg(target).spawn();
        }
    }

    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("explorer").arg(path).spawn();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("xdg-open").arg(path).spawn();
    }
}

pub fn open_file_default(path: &Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(path).spawn();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(path)
            .spawn();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("xdg-open").arg(path).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AudioBitrate, AudioFormat, BrowserCookies, SpeedLimit, VideoCodec, VideoFormat,
        VideoQuality,
    };

    #[test]
    fn builds_video_download_args_with_pro_options() {
        let opts = DownloadOptions {
            mode: MediaMode::Video,
            video_quality: VideoQuality::Q1080p,
            video_format: VideoFormat::Mp4,
            video_codec: VideoCodec::H264,
            sponsorblock: true,
            download_subtitles: true,
            subtitle_langs: "th,en".to_string(),
            trim_enabled: true,
            trim_start: "00:10".to_string(),
            trim_end: "00:45".to_string(),
            speed_limit: SpeedLimit::Mb5,
            cookies_browser: BrowserCookies::Chrome,
            ..Default::default()
        };

        let args = build_download_args("https://youtu.be/dQw4w9WgXcQ", &opts, None);
        assert!(args.iter().any(|a| a.contains("vcodec^=avc1")));
        assert!(args.contains(&"--merge-output-format".to_string()));
        assert!(args.contains(&"mp4".to_string()));
        assert!(args.contains(&"--sponsorblock-remove".to_string()));
        assert!(args.contains(&"--write-subs".to_string()));
        assert!(args.contains(&"*00:10-00:45".to_string()));
        assert!(args.contains(&"--limit-rate".to_string()));
        assert!(args.contains(&"5M".to_string()));
        assert!(args.contains(&"--cookies-from-browser".to_string()));
        assert!(args.contains(&"chrome".to_string()));
    }

    #[test]
    fn builds_audio_download_args() {
        let opts = DownloadOptions {
            mode: MediaMode::Audio,
            audio_format: AudioFormat::Mp3,
            audio_bitrate: AudioBitrate::K320,
            ..Default::default()
        };

        let args = build_download_args("https://youtu.be/dQw4w9WgXcQ", &opts, None);
        assert!(args.contains(&"-x".to_string()));
        assert!(args.contains(&"--audio-format".to_string()));
        assert!(args.contains(&"mp3".to_string()));
        assert!(args.contains(&"320K".to_string()));
    }

    #[test]
    fn detects_youtube_urls() {
        assert!(is_youtube_url("https://www.youtube.com/watch?v=abc123"));
        assert!(is_youtube_url("https://youtu.be/abc123"));
        assert!(is_youtube_url("https://youtube.com/shorts/xyz"));
        assert!(is_youtube_url("https://www.youtube.com/playlist?list=PL123"));
        assert!(!is_youtube_url("https://example.com/video"));
        assert!(!is_youtube_url("not a url"));
    }
}
