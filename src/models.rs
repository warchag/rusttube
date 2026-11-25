use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AppLanguage {
    #[default]
    Th,
    En,
}

impl AppLanguage {
    pub fn label(&self) -> &'static str {
        match self {
            AppLanguage::Th => "ไทย",
            AppLanguage::En => "English",
        }
    }

    pub fn short_badge(&self) -> &'static str {
        match self {
            AppLanguage::Th => "🌐 ไทย",
            AppLanguage::En => "🌐 EN",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            AppLanguage::Th => AppLanguage::En,
            AppLanguage::En => AppLanguage::Th,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Download,
    Tools,
    History,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AppTheme {
    #[default]
    MidnightStudio,
    TokyoNight,
    CatppuccinMocha,
    CyberpunkNeon,
    NordicFrost,
    Dracula,
    SakuraBlossom,
    AppleSnow,
}

impl AppTheme {
    pub const ALL: &'static [AppTheme] = &[
        AppTheme::MidnightStudio,
        AppTheme::TokyoNight,
        AppTheme::CatppuccinMocha,
        AppTheme::CyberpunkNeon,
        AppTheme::NordicFrost,
        AppTheme::Dracula,
        AppTheme::SakuraBlossom,
        AppTheme::AppleSnow,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            AppTheme::MidnightStudio => "Midnight Studio",
            AppTheme::TokyoNight => "Tokyo Night",
            AppTheme::CatppuccinMocha => "Catppuccin Mocha",
            AppTheme::CyberpunkNeon => "Cyberpunk Neon",
            AppTheme::NordicFrost => "Nordic Frost",
            AppTheme::Dracula => "Dracula Velvet",
            AppTheme::SakuraBlossom => "Sakura Blossom",
            AppTheme::AppleSnow => "Apple Snow",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            AppTheme::MidnightStudio => "🌑",
            AppTheme::TokyoNight => "🌃",
            AppTheme::CatppuccinMocha => "☕",
            AppTheme::CyberpunkNeon => "⚡",
            AppTheme::NordicFrost => "❄",
            AppTheme::Dracula => "🧛",
            AppTheme::SakuraBlossom => "🌸",
            AppTheme::AppleSnow => "☀",
        }
    }

    pub fn is_light(&self) -> bool {
        matches!(self, AppTheme::SakuraBlossom | AppTheme::AppleSnow)
    }

    pub fn next(&self) -> Self {
        let idx = Self::ALL.iter().position(|t| t == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AccentColor {
    #[default]
    ThemeDefault,
    Red,
    Purple,
    Blue,
    Emerald,
    Amber,
    Pink,
}

impl AccentColor {
    pub const ALL: &'static [AccentColor] = &[
        AccentColor::ThemeDefault,
        AccentColor::Red,
        AccentColor::Purple,
        AccentColor::Blue,
        AccentColor::Emerald,
        AccentColor::Amber,
        AccentColor::Pink,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            AccentColor::ThemeDefault => "Theme Auto",
            AccentColor::Red => "Crimson Red",
            AccentColor::Purple => "Violet",
            AccentColor::Blue => "Ocean Blue",
            AccentColor::Emerald => "Emerald",
            AccentColor::Amber => "Sunset Amber",
            AccentColor::Pink => "Neon Pink",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaMode {
    Video,
    Audio,
}

impl MediaMode {
    pub fn label(&self) -> &'static str {
        match self {
            MediaMode::Video => "Video + Audio",
            MediaMode::Audio => "Audio Only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VideoCodec {
    #[default]
    Auto,
    H264,
    Vp9,
    Av1,
}

impl VideoCodec {
    pub const ALL: &'static [VideoCodec] = &[
        VideoCodec::Auto,
        VideoCodec::H264,
        VideoCodec::Vp9,
        VideoCodec::Av1,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            VideoCodec::Auto => "Auto (Best)",
            VideoCodec::H264 => "H.264 (iPhone / Mac)",
            VideoCodec::Vp9 => "VP9",
            VideoCodec::Av1 => "AV1 (Smallest)",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            VideoCodec::Auto => "Auto",
            VideoCodec::H264 => "H.264",
            VideoCodec::Vp9 => "VP9",
            VideoCodec::Av1 => "AV1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoQuality {
    Best,
    Q2160p,
    Q1440p,
    Q1080p,
    Q720p,
    Q480p,
    Q360p,
}

impl VideoQuality {
    pub const ALL: &'static [VideoQuality] = &[
        VideoQuality::Best,
        VideoQuality::Q2160p,
        VideoQuality::Q1440p,
        VideoQuality::Q1080p,
        VideoQuality::Q720p,
        VideoQuality::Q480p,
        VideoQuality::Q360p,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            VideoQuality::Best => "Best Available",
            VideoQuality::Q2160p => "4K (2160p)",
            VideoQuality::Q1440p => "2K (1440p)",
            VideoQuality::Q1080p => "Full HD (1080p)",
            VideoQuality::Q720p => "HD (720p)",
            VideoQuality::Q480p => "SD (480p)",
            VideoQuality::Q360p => "Low (360p)",
        }
    }

    pub fn height(&self) -> Option<u32> {
        match self {
            VideoQuality::Best => None,
            VideoQuality::Q2160p => Some(2160),
            VideoQuality::Q1440p => Some(1440),
            VideoQuality::Q1080p => Some(1080),
            VideoQuality::Q720p => Some(720),
            VideoQuality::Q480p => Some(480),
            VideoQuality::Q360p => Some(360),
        }
    }

    pub fn format_selector(&self, codec: VideoCodec) -> String {
        let h_filter = match self.height() {
            Some(h) => format!("[height<={h}]"),
            None => String::new(),
        };
        match codec {
            VideoCodec::Auto => format!("bestvideo{h_filter}+bestaudio/best{h_filter}"),
            VideoCodec::H264 => format!(
                "bestvideo[vcodec^=avc1]{h_filter}+bestaudio[ext=m4a]/bestvideo[vcodec^=avc1]{h_filter}+bestaudio/bestvideo{h_filter}+bestaudio/best{h_filter}"
            ),
            VideoCodec::Vp9 => format!(
                "bestvideo[vcodec^=vp9]{h_filter}+bestaudio/bestvideo{h_filter}+bestaudio/best{h_filter}"
            ),
            VideoCodec::Av1 => format!(
                "bestvideo[vcodec^=av01]{h_filter}+bestaudio/bestvideo{h_filter}+bestaudio/best{h_filter}"
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoFormat {
    Mp4,
    Mkv,
    Webm,
}

impl VideoFormat {
    pub const ALL: &'static [VideoFormat] = &[
        VideoFormat::Mp4,
        VideoFormat::Mkv,
        VideoFormat::Webm,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            VideoFormat::Mp4 => "MP4",
            VideoFormat::Mkv => "MKV",
            VideoFormat::Webm => "WEBM",
        }
    }

    pub fn ext(&self) -> &'static str {
        match self {
            VideoFormat::Mp4 => "mp4",
            VideoFormat::Mkv => "mkv",
            VideoFormat::Webm => "webm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    Mp3,
    M4a,
    Flac,
    Wav,
    Opus,
}

impl AudioFormat {
    pub const ALL: &'static [AudioFormat] = &[
        AudioFormat::Mp3,
        AudioFormat::M4a,
        AudioFormat::Flac,
        AudioFormat::Wav,
        AudioFormat::Opus,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "MP3",
            AudioFormat::M4a => "M4A",
            AudioFormat::Flac => "FLAC",
            AudioFormat::Wav => "WAV",
            AudioFormat::Opus => "OPUS",
        }
    }

    pub fn ext(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::M4a => "m4a",
            AudioFormat::Flac => "flac",
            AudioFormat::Wav => "wav",
            AudioFormat::Opus => "opus",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioBitrate {
    K320,
    K192,
    K128,
}

impl AudioBitrate {
    pub const ALL: &'static [AudioBitrate] = &[
        AudioBitrate::K320,
        AudioBitrate::K192,
        AudioBitrate::K128,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            AudioBitrate::K320 => "320 kbps (Best)",
            AudioBitrate::K192 => "192 kbps (High)",
            AudioBitrate::K128 => "128 kbps (Standard)",
        }
    }

    pub fn ytdlp_arg(&self) -> &'static str {
        match self {
            AudioBitrate::K320 => "320K",
            AudioBitrate::K192 => "192K",
            AudioBitrate::K128 => "128K",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeedLimit {
    Unlimited,
    Mb10,
    Mb5,
    Mb2,
    Mb1,
}

impl SpeedLimit {
    pub const ALL: &'static [SpeedLimit] = &[
        SpeedLimit::Unlimited,
        SpeedLimit::Mb10,
        SpeedLimit::Mb5,
        SpeedLimit::Mb2,
        SpeedLimit::Mb1,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            SpeedLimit::Unlimited => "Unlimited",
            SpeedLimit::Mb10 => "10 MB/s",
            SpeedLimit::Mb5 => "5 MB/s",
            SpeedLimit::Mb2 => "2 MB/s",
            SpeedLimit::Mb1 => "1 MB/s",
        }
    }

    pub fn ytdlp_arg(&self) -> Option<&'static str> {
        match self {
            SpeedLimit::Unlimited => None,
            SpeedLimit::Mb10 => Some("10M"),
            SpeedLimit::Mb5 => Some("5M"),
            SpeedLimit::Mb2 => Some("2M"),
            SpeedLimit::Mb1 => Some("1M"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserCookies {
    None,
    Chrome,
    Safari,
    Firefox,
    Brave,
    Edge,
}

impl BrowserCookies {
    pub const ALL: &'static [BrowserCookies] = &[
        BrowserCookies::None,
        BrowserCookies::Chrome,
        BrowserCookies::Safari,
        BrowserCookies::Firefox,
        BrowserCookies::Brave,
        BrowserCookies::Edge,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            BrowserCookies::None => "None",
            BrowserCookies::Chrome => "Google Chrome",
            BrowserCookies::Safari => "Safari",
            BrowserCookies::Firefox => "Firefox",
            BrowserCookies::Brave => "Brave",
            BrowserCookies::Edge => "Microsoft Edge",
        }
    }

    pub fn ytdlp_arg(&self) -> Option<&'static str> {
        match self {
            BrowserCookies::None => None,
            BrowserCookies::Chrome => Some("chrome"),
            BrowserCookies::Safari => Some("safari"),
            BrowserCookies::Firefox => Some("firefox"),
            BrowserCookies::Brave => Some("brave"),
            BrowserCookies::Edge => Some("edge"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadOptions {
    pub mode: MediaMode,
    pub video_quality: VideoQuality,
    pub video_format: VideoFormat,
    pub video_codec: VideoCodec,
    pub audio_format: AudioFormat,
    pub audio_bitrate: AudioBitrate,
    pub output_dir: PathBuf,
    pub download_subtitles: bool,
    pub subtitle_langs: String,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub sponsorblock: bool,
    pub download_playlist: bool,
    pub trim_enabled: bool,
    pub trim_start: String,
    pub trim_end: String,
    pub speed_limit: SpeedLimit,
    pub cookies_browser: BrowserCookies,
    pub filename_template: String,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        let default_dir = dirs::download_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            mode: MediaMode::Video,
            video_quality: VideoQuality::Q1080p,
            video_format: VideoFormat::Mp4,
            video_codec: VideoCodec::Auto,
            audio_format: AudioFormat::Mp3,
            audio_bitrate: AudioBitrate::K320,
            output_dir: default_dir,
            download_subtitles: false,
            subtitle_langs: "th,en".to_string(),
            embed_thumbnail: true,
            embed_metadata: true,
            sponsorblock: false,
            download_playlist: false,
            trim_enabled: false,
            trim_start: "00:00".to_string(),
            trim_end: "01:00".to_string(),
            speed_limit: SpeedLimit::Unlimited,
            cookies_browser: BrowserCookies::None,
            filename_template: "%(title)s.%(ext)s".to_string(),
        }
    }
}

impl DownloadOptions {
    pub fn summary_badge(&self) -> String {
        match self.mode {
            MediaMode::Video => {
                if self.video_codec == VideoCodec::Auto {
                    format!("{} • {}", self.video_quality.label(), self.video_format.label())
                } else {
                    format!(
                        "{} • {} • {}",
                        self.video_quality.label(),
                        self.video_format.label(),
                        self.video_codec.short_label()
                    )
                }
            }
            MediaMode::Audio => {
                format!("{} • {}", self.audio_format.label(), self.audio_bitrate.label())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormatEstimate {
    pub height: u32,
    pub size_bytes: u64,
}

impl FormatEstimate {
    pub fn formatted_size(&self) -> String {
        format_bytes(self.size_bytes)
    }
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.0} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{bytes} B")
    }
}

#[derive(Debug, Clone)]
pub struct PlaylistEntry {
    pub index: usize,
    pub title: String,
    pub url: String,
    pub duration_secs: Option<u64>,
    pub selected: bool,
}

impl PlaylistEntry {
    pub fn formatted_duration(&self) -> String {
        match self.duration_secs {
            Some(secs) => format_duration_secs(secs),
            None => "—".to_string(),
        }
    }
}

pub fn format_duration_secs(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

#[derive(Debug, Clone)]
pub struct VideoInfo {
    pub url: String,
    pub title: String,
    pub uploader: String,
    pub duration_secs: Option<u64>,
    pub view_count: Option<u64>,
    pub upload_date: Option<String>,
    pub thumbnail_url: Option<String>,
    pub is_playlist: bool,
    pub playlist_entries: Vec<PlaylistEntry>,
    pub format_estimates: Vec<FormatEstimate>,
    pub audio_size_bytes: Option<u64>,
}

impl VideoInfo {
    pub fn formatted_duration(&self) -> String {
        match self.duration_secs {
            Some(secs) => format_duration_secs(secs),
            None => {
                if self.is_playlist {
                    format!("{} videos", self.playlist_entries.len())
                } else {
                    "Live / Unknown".to_string()
                }
            }
        }
    }

    pub fn formatted_views(&self) -> String {
        match self.view_count {
            Some(v) if v >= 1_000_000_000 => format!("{:.1}B views", v as f64 / 1e9),
            Some(v) if v >= 1_000_000 => format!("{:.1}M views", v as f64 / 1e6),
            Some(v) if v >= 1_000 => format!("{:.1}K views", v as f64 / 1e3),
            Some(v) => format!("{v} views"),
            None => String::new(),
        }
    }

    pub fn formatted_date(&self) -> String {
        if let Some(d) = &self.upload_date {
            if d.len() == 8 {
                return format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8]);
            }
        }
        String::new()
    }

    pub fn estimate_for_quality(&self, quality: VideoQuality) -> Option<String> {
        let target_h = quality.height();
        let best = match target_h {
            Some(h) => self
                .format_estimates
                .iter()
                .filter(|f| f.height <= h)
                .max_by_key(|f| f.height),
            None => self.format_estimates.iter().max_by_key(|f| f.height),
        };
        best.map(|f| format!("~{}", f.formatted_size()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Queued,
    Downloading,
    Processing(String),
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub id: String,
    pub url: String,
    pub title: String,
    pub uploader: String,
    pub options: DownloadOptions,
    pub status: TaskStatus,
    pub progress: f32, // 0.0..=1.0
    pub speed: String,
    pub eta: String,
    pub total_size: String,
    pub output_file: Option<PathBuf>,
    pub cancel_flag: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub uploader: String,
    pub format_badge: String,
    pub output_path: PathBuf,
    pub completed_at: DateTime<Local>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub options: DownloadOptions,
    pub max_concurrent: usize,
    pub auto_fetch_on_paste: bool,
    pub clipboard_watch: bool,
    pub notify_on_complete: bool,
    pub theme: AppTheme,
    pub accent_color: AccentColor,
    pub language: AppLanguage,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            options: DownloadOptions::default(),
            max_concurrent: 2,
            auto_fetch_on_paste: true,
            clipboard_watch: false,
            notify_on_complete: true,
            theme: AppTheme::MidnightStudio,
            accent_color: AccentColor::ThemeDefault,
            language: AppLanguage::Th,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DependencyStatus {
    pub ytdlp_path: Option<PathBuf>,
    pub ytdlp_version: Option<String>,
    pub ffmpeg_path: Option<PathBuf>,
    pub ffmpeg_version: Option<String>,
}

// --- Media Tools (FFmpeg) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaToolMode {
    Convert,
    Compress,
    Trim,
    ExtractAudio,
}

impl MediaToolMode {
    pub const ALL: &'static [MediaToolMode] = &[
        MediaToolMode::Convert,
        MediaToolMode::Compress,
        MediaToolMode::Trim,
        MediaToolMode::ExtractAudio,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            MediaToolMode::Convert => "🔄 Convert Format",
            MediaToolMode::Compress => "🗜 Compress Video",
            MediaToolMode::Trim => "✂ Trim / Cut",
            MediaToolMode::ExtractAudio => "🎵 Extract Audio",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertTarget {
    Mp4H264,
    Mkv,
    Webm,
    Mp3,
    M4a,
    Wav,
    Flac,
    Gif,
}

impl ConvertTarget {
    pub const ALL: &'static [ConvertTarget] = &[
        ConvertTarget::Mp4H264,
        ConvertTarget::Mkv,
        ConvertTarget::Webm,
        ConvertTarget::Mp3,
        ConvertTarget::M4a,
        ConvertTarget::Wav,
        ConvertTarget::Flac,
        ConvertTarget::Gif,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ConvertTarget::Mp4H264 => "MP4 (H.264 / Universal)",
            ConvertTarget::Mkv => "MKV (Matroska)",
            ConvertTarget::Webm => "WEBM (VP9)",
            ConvertTarget::Mp3 => "MP3 Audio (320k)",
            ConvertTarget::M4a => "M4A Audio (AAC)",
            ConvertTarget::Wav => "WAV Audio (Lossless)",
            ConvertTarget::Flac => "FLAC Audio (Lossless)",
            ConvertTarget::Gif => "Animated GIF (15fps)",
        }
    }

    pub fn ext(&self) -> &'static str {
        match self {
            ConvertTarget::Mp4H264 => "mp4",
            ConvertTarget::Mkv => "mkv",
            ConvertTarget::Webm => "webm",
            ConvertTarget::Mp3 => "mp3",
            ConvertTarget::M4a => "m4a",
            ConvertTarget::Wav => "wav",
            ConvertTarget::Flac => "flac",
            ConvertTarget::Gif => "gif",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressQuality {
    High,
    Balanced,
    Small,
    Tiny,
}

impl CompressQuality {
    pub const ALL: &'static [CompressQuality] = &[
        CompressQuality::High,
        CompressQuality::Balanced,
        CompressQuality::Small,
        CompressQuality::Tiny,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            CompressQuality::High => "High Quality (CRF 20)",
            CompressQuality::Balanced => "Balanced (CRF 24)",
            CompressQuality::Small => "Small Size (CRF 28)",
            CompressQuality::Tiny => "Tiny / Chat Share (CRF 32)",
        }
    }

    pub fn crf(&self) -> &'static str {
        match self {
            CompressQuality::High => "20",
            CompressQuality::Balanced => "24",
            CompressQuality::Small => "28",
            CompressQuality::Tiny => "32",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleResolution {
    Original,
    P1080,
    P720,
    P480,
}

impl ScaleResolution {
    pub const ALL: &'static [ScaleResolution] = &[
        ScaleResolution::Original,
        ScaleResolution::P1080,
        ScaleResolution::P720,
        ScaleResolution::P480,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ScaleResolution::Original => "Original Resolution",
            ScaleResolution::P1080 => "1080p",
            ScaleResolution::P720 => "720p",
            ScaleResolution::P480 => "480p",
        }
    }

    pub fn scale_filter(&self) -> Option<&'static str> {
        match self {
            ScaleResolution::Original => None,
            ScaleResolution::P1080 => Some("scale=-2:1080"),
            ScaleResolution::P720 => Some("scale=-2:720"),
            ScaleResolution::P480 => Some("scale=-2:480"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MediaToolState {
    pub input_file: Option<PathBuf>,
    pub mode: MediaToolMode,
    pub convert_target: ConvertTarget,
    pub compress_quality: CompressQuality,
    pub compress_scale: ScaleResolution,
    pub trim_start: String,
    pub trim_end: String,
    pub trim_fast_copy: bool,
    pub extract_format: AudioFormat,
    pub running: bool,
    pub status_msg: Option<String>,
    pub last_output: Option<PathBuf>,
    pub last_error: Option<String>,
}

impl Default for MediaToolState {
    fn default() -> Self {
        Self {
            input_file: None,
            mode: MediaToolMode::Convert,
            convert_target: ConvertTarget::Mp4H264,
            compress_quality: CompressQuality::Balanced,
            compress_scale: ScaleResolution::Original,
            trim_start: "00:00".to_string(),
            trim_end: "00:30".to_string(),
            trim_fast_copy: true,
            extract_format: AudioFormat::Mp3,
            running: false,
            status_msg: None,
            last_output: None,
            last_error: None,
        }
    }
}

/// Messages sent from background worker threads back to the UI thread.
pub enum WorkerMessage {
    DependenciesChecked(DependencyStatus),
    YtdlpUpdateFinished(Result<String, String>),
    InfoFetched(Result<VideoInfo, String>),
    ThumbnailLoaded(String, Result<(Vec<u8>, [usize; 2]), String>),
    TaskStarted {
        id: String,
    },
    TaskProgress {
        id: String,
        progress: f32,
        speed: String,
        eta: String,
        total_size: String,
    },
    TaskStage {
        id: String,
        stage: String,
    },
    TaskLog {
        id: String,
        line: String,
    },
    TaskFinished {
        id: String,
        result: Result<Option<PathBuf>, String>,
        cancelled: bool,
    },
    MediaToolFinished(Result<PathBuf, String>),
}
