use crate::models::{
    AudioFormat, CompressQuality, ConvertTarget, MediaToolMode, MediaToolState, ScaleResolution,
    WorkerMessage,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;

pub fn run_media_tool(
    ffmpeg_path: PathBuf,
    state: MediaToolState,
    output_dir: PathBuf,
    tx: Sender<WorkerMessage>,
    ctx: eframe::egui::Context,
) {
    let Some(input) = state.input_file.clone() else {
        let _ = tx.send(WorkerMessage::MediaToolFinished(Err(
            "Please select an input file first.".to_string(),
        )));
        ctx.request_repaint();
        return;
    };

    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let orig_ext = input
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("mp4");

    let (output_path, args) = match state.mode {
        MediaToolMode::Convert => {
            let out = unique_output(&output_dir, &format!("{stem}_converted"), state.convert_target.ext());
            let args = build_convert_args(&input, &out, state.convert_target);
            (out, args)
        }
        MediaToolMode::Compress => {
            let out = unique_output(&output_dir, &format!("{stem}_compressed"), "mp4");
            let args = build_compress_args(&input, &out, state.compress_quality, state.compress_scale);
            (out, args)
        }
        MediaToolMode::Trim => {
            let out = unique_output(&output_dir, &format!("{stem}_trimmed"), orig_ext);
            let args = build_trim_args(
                &input,
                &out,
                state.trim_start.trim(),
                state.trim_end.trim(),
                state.trim_fast_copy,
            );
            (out, args)
        }
        MediaToolMode::ExtractAudio => {
            let out = unique_output(&output_dir, &format!("{stem}_audio"), state.extract_format.ext());
            let args = build_extract_audio_args(&input, &out, state.extract_format);
            (out, args)
        }
    };

    let _ = tx.send(WorkerMessage::TaskLog {
        id: "ffmpeg".to_string(),
        line: format!("$ {} {}", ffmpeg_path.display(), args.join(" ")),
    });

    let res = Command::new(&ffmpeg_path).args(&args).output();
    match res {
        Ok(out) if out.status.success() => {
            let _ = tx.send(WorkerMessage::MediaToolFinished(Ok(output_path)));
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let last_line = stderr
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("FFmpeg processing failed")
                .to_string();
            let _ = tx.send(WorkerMessage::TaskLog {
                id: "ffmpeg".to_string(),
                line: last_line.clone(),
            });
            let _ = tx.send(WorkerMessage::MediaToolFinished(Err(last_line)));
        }
        Err(e) => {
            let _ = tx.send(WorkerMessage::MediaToolFinished(Err(format!(
                "Failed to run ffmpeg: {e}"
            ))));
        }
    }
    ctx.request_repaint();
}

fn unique_output(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let mut candidate = dir.join(format!("{base}.{ext}"));
    let mut counter = 1u32;
    while candidate.exists() {
        candidate = dir.join(format!("{base}_{counter}.{ext}"));
        counter += 1;
    }
    candidate
}

fn build_convert_args(input: &Path, output: &Path, target: ConvertTarget) -> Vec<String> {
    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        input.to_string_lossy().to_string(),
    ];

    match target {
        ConvertTarget::Mp4H264 => {
            args.extend([
                "-c:v".into(),
                "libx264".into(),
                "-preset".into(),
                "fast".into(),
                "-crf".into(),
                "21".into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                "192k".into(),
            ]);
        }
        ConvertTarget::Mkv => {
            args.extend(["-c".into(), "copy".into()]);
        }
        ConvertTarget::Webm => {
            args.extend([
                "-c:v".into(),
                "libvpx-vp9".into(),
                "-crf".into(),
                "30".into(),
                "-b:v".into(),
                "0".into(),
                "-c:a".into(),
                "libopus".into(),
            ]);
        }
        ConvertTarget::Mp3 => {
            args.extend([
                "-vn".into(),
                "-c:a".into(),
                "libmp3lame".into(),
                "-b:a".into(),
                "320k".into(),
            ]);
        }
        ConvertTarget::M4a => {
            args.extend([
                "-vn".into(),
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                "256k".into(),
            ]);
        }
        ConvertTarget::Wav => {
            args.extend(["-vn".into(), "-c:a".into(), "pcm_s16le".into()]);
        }
        ConvertTarget::Flac => {
            args.extend(["-vn".into(), "-c:a".into(), "flac".into()]);
        }
        ConvertTarget::Gif => {
            args.extend([
                "-vf".into(),
                "fps=15,scale=640:-1:flags=lanczos".into(),
                "-loop".into(),
                "0".into(),
            ]);
        }
    }

    args.push(output.to_string_lossy().to_string());
    args
}

fn build_compress_args(
    input: &Path,
    output: &Path,
    quality: CompressQuality,
    scale: ScaleResolution,
) -> Vec<String> {
    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        input.to_string_lossy().to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "medium".to_string(),
        "-crf".to_string(),
        quality.crf().to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        "128k".to_string(),
    ];

    if let Some(vf) = scale.scale_filter() {
        args.push("-vf".to_string());
        args.push(vf.to_string());
    }

    args.push(output.to_string_lossy().to_string());
    args
}

fn build_trim_args(
    input: &Path,
    output: &Path,
    start: &str,
    end: &str,
    fast_copy: bool,
) -> Vec<String> {
    let mut args = vec!["-y".to_string()];
    if !start.is_empty() {
        args.push("-ss".to_string());
        args.push(start.to_string());
    }
    args.push("-i".to_string());
    args.push(input.to_string_lossy().to_string());
    if !end.is_empty() {
        args.push("-to".to_string());
        args.push(end.to_string());
    }

    if fast_copy {
        args.extend(["-c".into(), "copy".into()]);
    } else {
        args.extend([
            "-c:v".into(),
            "libx264".into(),
            "-preset".into(),
            "fast".into(),
            "-crf".into(),
            "20".into(),
            "-c:a".into(),
            "aac".into(),
        ]);
    }

    args.push(output.to_string_lossy().to_string());
    args
}

fn build_extract_audio_args(input: &Path, output: &Path, fmt: AudioFormat) -> Vec<String> {
    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        input.to_string_lossy().to_string(),
        "-vn".to_string(),
    ];

    match fmt {
        AudioFormat::Mp3 => args.extend(["-c:a".into(), "libmp3lame".into(), "-b:a".into(), "320k".into()]),
        AudioFormat::M4a => args.extend(["-c:a".into(), "aac".into(), "-b:a".into(), "256k".into()]),
        AudioFormat::Flac => args.extend(["-c:a".into(), "flac".into()]),
        AudioFormat::Wav => args.extend(["-c:a".into(), "pcm_s16le".into()]),
        AudioFormat::Opus => args.extend(["-c:a".into(), "libopus".into(), "-b:a".into(), "192k".into()]),
    }

    args.push(output.to_string_lossy().to_string());
    args
}
