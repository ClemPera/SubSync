// SubSync - Subtitle Synchronization & Batch Renaming Tool
// A tool for shifting subtitle timestamps and renaming them to match video files

use regex::Regex;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static SXXEXX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bS\d{1,2}E(\d{1,3})\b").unwrap());
static EP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:\b|_)E(?:P)?(\d{1,3})\b").unwrap());
static EPISODE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bepisode[_\s.-]*(\d{1,3})\b").unwrap());
static TRAILING_EP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|[\s\-_])(\d{2,3})(?:$|[\s\-_.\[(])").unwrap());

fn parse_timestamp_srt(ts: &str) -> Option<i64> {
    let parts: Vec<&str> = ts.split(&[':', ','][..]).collect();
    if parts.len() != 4 {
        return None;
    }

    let hours: i64 = parts[0].parse().ok()?;
    let minutes: i64 = parts[1].parse().ok()?;
    let seconds: i64 = parts[2].parse().ok()?;
    let millis: i64 = parts[3].parse().ok()?;

    Some(hours * 3600000 + minutes * 60000 + seconds * 1000 + millis)
}

fn parse_timestamp_ass(ts: &str) -> Option<i64> {
    // ASS format: H:MM:SS.CC (centiseconds, not milliseconds)
    let parts: Vec<&str> = ts.split(&[':', '.'][..]).collect();
    if parts.len() != 4 {
        return None;
    }

    let hours: i64 = parts[0].parse().ok()?;
    let minutes: i64 = parts[1].parse().ok()?;
    let seconds: i64 = parts[2].parse().ok()?;
    let centiseconds: i64 = parts[3].parse().ok()?;

    Some(hours * 3600000 + minutes * 60000 + seconds * 1000 + centiseconds * 10)
}

fn format_timestamp_srt(ms: i64) -> String {
    let hours = ms / 3600000;
    let minutes = (ms % 3600000) / 60000;
    let seconds = (ms % 60000) / 1000;
    let millis = ms % 1000;

    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
}

fn format_timestamp_ass(ms: i64) -> String {
    let hours = ms / 3600000;
    let minutes = (ms % 3600000) / 60000;
    let seconds = (ms % 60000) / 1000;
    let centiseconds = (ms % 1000) / 10;

    format!(
        "{}:{:02}:{:02}.{:02}",
        hours, minutes, seconds, centiseconds
    )
}

fn shift_srt(content: &str, shift_ms: i64) -> String {
    let mut result = String::new();

    for line in content.lines() {
        if line.contains(" --> ") {
            let parts: Vec<&str> = line.split(" --> ").collect();
            if parts.len() == 2
                && let (Some(start_ms), Some(end_ms)) =
                    (parse_timestamp_srt(parts[0]), parse_timestamp_srt(parts[1]))
            {
                let new_start = (start_ms + shift_ms).max(0);
                let new_end = (end_ms + shift_ms).max(0);
                result.push_str(&format!(
                    "{} --> {}\n",
                    format_timestamp_srt(new_start),
                    format_timestamp_srt(new_end)
                ));
                continue;
            }
        }
        result.push_str(line);
        result.push('\n');
    }

    result
}

fn shift_ass(content: &str, shift_ms: i64) -> String {
    let dialogue_re =
        Regex::new(r"^(Dialogue: \d+,)(\d+:\d+:\d+\.\d+),(\d+:\d+:\d+\.\d+),(.+)$").unwrap();
    let mut result = String::new();

    for line in content.lines() {
        if let Some(caps) = dialogue_re.captures(line) {
            let prefix = &caps[1];
            let start = &caps[2];
            let end = &caps[3];
            let rest = &caps[4];

            if let (Some(start_ms), Some(end_ms)) =
                (parse_timestamp_ass(start), parse_timestamp_ass(end))
            {
                let new_start = (start_ms + shift_ms).max(0);
                let new_end = (end_ms + shift_ms).max(0);
                result.push_str(&format!(
                    "{}{},{},{}\n",
                    prefix,
                    format_timestamp_ass(new_start),
                    format_timestamp_ass(new_end),
                    rest
                ));
                continue;
            }
        }
        result.push_str(line);
        result.push('\n');
    }

    result
}

fn extract_episode_number(filename: &str) -> Option<u32> {
    // Ordered most-specific first; the fallback takes the last number to skip
    // leading title numbers and bracket hashes.
    for re in [&*SXXEXX_RE, &*EP_RE, &*EPISODE_RE] {
        if let Some(num) = re.captures(filename).and_then(|caps| caps[1].parse().ok()) {
            return Some(num);
        }
    }

    TRAILING_EP_RE
        .captures_iter(filename)
        .last()
        .and_then(|caps| caps[1].parse().ok())
}

fn find_matching_video(video_files: &[(PathBuf, u32)], episode: u32) -> Option<&PathBuf> {
    video_files
        .iter()
        .find(|(_, ep)| *ep == episode)
        .map(|(path, _)| path)
}

fn unique_shifted_path(folder: &Path, source: &Path, ext: &str) -> PathBuf {
    let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let base = stem.strip_prefix("shifted_").unwrap_or(stem);
    let mut candidate = folder.join(format!("shifted_{base}.{ext}"));

    let mut counter = 1;
    while candidate.exists() && candidate != source {
        candidate = folder.join(format!("shifted_{counter}_{base}.{ext}"));
        counter += 1;
    }

    candidate
}

fn file_name(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

fn paint(code: &str, text: &str) -> String {
    if std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

enum Outcome {
    InPlace,
    Renamed(PathBuf),
    ShiftedOnly { target: PathBuf, reason: String },
    TargetExists { target: PathBuf, occupied: PathBuf },
}

#[derive(Default)]
struct Report {
    processed: usize,
    renamed: usize,
    shifted_only: usize,
    already_named: usize,
    warnings: Vec<(String, String)>,
    failures: Vec<(String, String)>,
}

impl Report {
    fn warn(&mut self, file: &str, reason: String) {
        self.warnings.push((file.to_string(), reason));
    }

    fn fail(&mut self, file: &str, reason: String) {
        self.failures.push((file.to_string(), reason));
    }

    fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    fn print_summary(&self) {
        println!(
            "\nSummary: {} processed, {} renamed, {} shifted only, {} already named, {} failed",
            self.processed,
            self.renamed,
            self.shifted_only,
            self.already_named,
            self.failures.len()
        );

        if !self.warnings.is_empty() {
            println!("\n{}", paint("33", "⚠ Warnings:"));
            for (file, reason) in &self.warnings {
                println!("{}", paint("33", &format!("  - {file}: {reason}")));
            }
        }

        if !self.failures.is_empty() {
            println!("\n{}", paint("31", "✗ Failed:"));
            for (file, reason) in &self.failures {
                println!("{}", paint("31", &format!("  - {file}: {reason}")));
            }
        }
    }
}

fn process_folder(folder_path: &Path, shift_ms: i64) -> std::io::Result<Report> {
    if !folder_path.exists() || !folder_path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("'{}' is not a valid directory", folder_path.display()),
        ));
    }

    println!("Scanning folder: {}", folder_path.display());
    println!(
        "Time shift: {} seconds ({} ms)\n",
        shift_ms as f64 / 1000.0,
        shift_ms
    );

    let entries = fs::read_dir(folder_path)?;

    let mut video_files = Vec::new();
    let mut subtitle_files = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        let Some(ext) = path.extension() else {
            continue;
        };
        let ext_str = ext.to_str().unwrap_or("").to_lowercase();
        let filename = file_name(&path);
        let episode = extract_episode_number(filename);

        match ext_str.as_str() {
            "mkv" | "mp4" | "avi" => {
                if let Some(episode) = episode {
                    video_files.push((path, episode));
                }
            }
            "srt" | "ass" => subtitle_files.push((path, episode, ext_str)),
            _ => {}
        }
    }

    println!("Found {} video files", video_files.len());
    println!("Found {} subtitle files\n", subtitle_files.len());

    let mut report = Report::default();

    for (sub_path, episode, ext) in subtitle_files {
        report.processed += 1;
        let sub_name = file_name(&sub_path).to_string();
        println!("Processing: {sub_name}");

        let content = match fs::read_to_string(&sub_path) {
            Ok(content) => content,
            Err(e) => {
                let reason = format!("failed to read file: {e}");
                report.fail(&sub_name, reason.clone());
                println!("  {}", paint("31", &format!("✗ {reason}")));
                continue;
            }
        };

        let shifted_content = match ext.as_str() {
            "srt" => shift_srt(&content, shift_ms),
            "ass" => shift_ass(&content, shift_ms),
            _ => content,
        };

        let sub_stem = sub_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let already_named = video_files
            .iter()
            .any(|(path, _)| path.file_stem().and_then(|s| s.to_str()) == Some(sub_stem));

        let outcome = if already_named {
            Outcome::InPlace
        } else if let Some(video_path) =
            episode.and_then(|ep| find_matching_video(&video_files, ep))
        {
            let video_stem = video_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let candidate = folder_path.join(format!("{video_stem}.{ext}"));
            if candidate == sub_path {
                Outcome::InPlace
            } else if !candidate.exists() {
                Outcome::Renamed(candidate)
            } else {
                Outcome::TargetExists {
                    target: unique_shifted_path(folder_path, &sub_path, &ext),
                    occupied: candidate,
                }
            }
        } else if let Some(episode) = episode {
            Outcome::ShiftedOnly {
                target: unique_shifted_path(folder_path, &sub_path, &ext),
                reason: format!("no matching video for episode {episode}"),
            }
        } else {
            Outcome::ShiftedOnly {
                target: unique_shifted_path(folder_path, &sub_path, &ext),
                reason: "no episode detected".to_string(),
            }
        };

        let destination = match &outcome {
            Outcome::InPlace => None,
            Outcome::Renamed(path)
            | Outcome::ShiftedOnly { target: path, .. }
            | Outcome::TargetExists { target: path, .. } => Some(path.as_path()),
        };
        let target_path = destination.unwrap_or(&sub_path);

        if target_path == sub_path {
            if shift_ms != 0
                && let Err(e) = fs::write(&sub_path, &shifted_content)
            {
                let reason = format!("failed to write file: {e}");
                report.fail(&sub_name, reason.clone());
                println!("  {}", paint("31", &format!("✗ {reason}")));
                continue;
            }
        } else {
            if let Err(e) = fs::write(target_path, &shifted_content) {
                let reason = format!("failed to write {}: {e}", file_name(target_path));
                report.fail(&sub_name, reason.clone());
                println!("  {}", paint("31", &format!("✗ {reason}")));
                continue;
            }

            if let Err(e) = fs::remove_file(&sub_path) {
                let reason = format!(
                    "wrote {} but failed to remove original: {e}",
                    file_name(target_path)
                );
                report.fail(&sub_name, reason.clone());
                println!("  {}", paint("31", &format!("✗ {reason}")));
                continue;
            }
        }

        match outcome {
            Outcome::InPlace => {
                report.already_named += 1;
                let message = if shift_ms == 0 {
                    format!("✓ Already correctly named: {sub_name}")
                } else {
                    format!("✓ Shifted in place: {sub_name}")
                };
                println!("  {}", paint("32", &message));
            }
            Outcome::Renamed(target) => {
                report.renamed += 1;
                let target_name = file_name(&target);
                println!(
                    "  {}",
                    paint("32", &format!("✓ Shifted and renamed to: {target_name}"))
                );
            }
            Outcome::ShiftedOnly { target, reason } => {
                report.shifted_only += 1;
                let target_name = file_name(&target);
                let warning = format!("{reason} (saved as {target_name})");
                report.warn(&sub_name, warning.clone());
                println!("  {}", paint("33", &format!("⚠ {warning}")));
            }
            Outcome::TargetExists { target, occupied } => {
                report.shifted_only += 1;
                let target_name = file_name(&target);
                let occupied_name = file_name(&occupied);
                let warning =
                    format!("target {occupied_name} already exists, saved as {target_name}");
                report.warn(&sub_name, warning.clone());
                println!("  {}", paint("33", &format!("⚠ {warning}")));
            }
        }
    }

    Ok(report)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <folder_path> <shift_seconds>", args[0]);
        eprintln!("Example: {} ./subtitles -5.43", args[0]);
        eprintln!("\nThis will:");
        eprintln!("  1. Process all subtitle files (.srt, .ass) in the folder");
        eprintln!("  2. Shift timestamps by the specified amount (negative = earlier)");
        eprintln!("  3. Rename subtitles to match video files based on episode numbers");
        std::process::exit(1);
    }

    let folder_path = Path::new(&args[1]);
    let shift_seconds: f64 = args[2].parse().expect("Invalid shift value");
    let shift_ms = (shift_seconds * 1000.0) as i64;

    if !folder_path.exists() || !folder_path.is_dir() {
        eprintln!(
            "Error: '{}' is not a valid directory",
            folder_path.display()
        );
        std::process::exit(1);
    }

    match process_folder(folder_path, shift_ms) {
        Ok(report) => {
            report.print_summary();
            if report.has_failures() {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_temp_dir(tag: &str) -> PathBuf {
        let id = TEST_DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "subtitle_tool_{}_{}_{}",
            tag,
            std::process::id(),
            id
        ));
        fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    #[test]
    fn srt_parse_format_roundtrip() {
        for ts in [
            "00:00:00,000",
            "00:01:02,003",
            "01:02:03,456",
            "10:20:30,999",
        ] {
            let ms = parse_timestamp_srt(ts).expect("should parse");
            assert_eq!(format_timestamp_srt(ms), ts);
        }
    }

    #[test]
    fn srt_parse_rejects_malformed() {
        for ts in [
            "",
            "00:01:02",
            "00:01:02.003",
            "not a timestamp",
            "00:xx:00,000",
        ] {
            assert_eq!(parse_timestamp_srt(ts), None, "input: {ts}");
        }
    }

    #[test]
    fn ass_parse_format_roundtrip() {
        for ts in ["0:00:00.00", "0:01:02.03", "1:02:03.45"] {
            let ms = parse_timestamp_ass(ts).expect("should parse");
            assert_eq!(format_timestamp_ass(ms), ts);
        }
    }

    #[test]
    fn ass_parse_rejects_malformed() {
        for ts in ["", "0:01:02", "0:01:02,03", "not a timestamp"] {
            assert_eq!(parse_timestamp_ass(ts), None, "input: {ts}");
        }
    }

    #[test]
    fn shift_srt_positive() {
        let input = "1\n00:00:01,000 --> 00:00:02,000\nHello\n";
        let out = shift_srt(input, 1500);
        assert!(out.contains("00:00:02,500 --> 00:00:03,500"), "got:\n{out}");
        assert!(out.contains("Hello"));
    }

    #[test]
    fn shift_srt_negative() {
        let input = "1\n00:00:05,000 --> 00:00:06,000\nHello\n";
        let out = shift_srt(input, -2000);
        assert!(out.contains("00:00:03,000 --> 00:00:04,000"), "got:\n{out}");
    }

    #[test]
    fn shift_srt_clamps_negative_to_zero() {
        let input = "1\n00:00:01,000 --> 00:00:02,000\nHello\n";
        let out = shift_srt(input, -5000);
        assert!(out.contains("00:00:00,000 --> 00:00:00,000"), "got:\n{out}");
    }

    #[test]
    fn shift_srt_malformed_lines_pass_through() {
        let input = "1\n00:00:xx,000 --> 00:00:02,000\nbogus --> line\nHello\n";
        let out = shift_srt(input, 1000);
        assert!(out.contains("00:00:xx,000 --> 00:00:02,000"), "got:\n{out}");
        assert!(out.contains("bogus --> line"), "got:\n{out}");
        assert!(out.contains("Hello"));
    }

    #[test]
    fn shift_ass_positive() {
        let input = "[Events]\nDialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,Hello\n";
        let out = shift_ass(input, 1500);
        assert!(
            out.contains("Dialogue: 0,0:00:02.50,0:00:03.50,Default,,0,0,0,,Hello"),
            "got:\n{out}"
        );
    }

    #[test]
    fn shift_ass_negative() {
        let input = "Dialogue: 0,0:00:05.00,0:00:06.00,Default,,0,0,0,,Hello\n";
        let out = shift_ass(input, -2000);
        assert!(out.contains("0:00:03.00,0:00:04.00"), "got:\n{out}");
    }

    #[test]
    fn shift_ass_clamps_negative_to_zero() {
        let input = "Dialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,Hello\n";
        let out = shift_ass(input, -5000);
        assert!(out.contains("0:00:00.00,0:00:00.00"), "got:\n{out}");
    }

    #[test]
    fn shift_ass_non_dialogue_lines_pass_through() {
        let input =
            "[Script Info]\nTitle: Test\nDialogue: 0,bad-start,bad-end,Default,,0,0,0,,Hi\n";
        let out = shift_ass(input, 1000);
        assert!(out.contains("[Script Info]"), "got:\n{out}");
        assert!(out.contains("Title: Test"), "got:\n{out}");
        assert!(
            out.contains("Dialogue: 0,bad-start,bad-end,Default,,0,0,0,,Hi"),
            "got:\n{out}"
        );
    }

    #[test]
    fn extract_episode_number_patterns() {
        let cases = [
            ("Show E01.mkv", Some(1)),
            ("show_e02.srt", Some(2)),
            ("Show EP12.mkv", Some(12)),
            ("show ep3.ass", Some(3)),
            ("Show episode 2.srt", Some(2)),
            ("Show episode_03.mkv", Some(3)),
            ("Show - 001.mkv", Some(1)),
            ("Show_004.mkv", Some(4)),
            ("Show S01E02.mkv", Some(2)),
            ("Show with no number.mkv", None),
            ("[SubsPlease] Show - 07 (1080p) [ABCD1234].mkv", Some(7)),
            ("[86] Eighty Six - 11 [1080p].mkv", Some(11)),
            ("Show 2019 - 05.mkv", Some(5)),
            (
                "March.Comes.in.like.a.Lion.S01E22.FiNAL.SUBFRENCH.1080p.WEB.x264-UwU.mkv",
                Some(22),
            ),
            ("Show x264-UwU.mkv", None),
        ];
        for (name, expected) in cases {
            assert_eq!(extract_episode_number(name), expected, "input: {name}");
        }

        let hashes = [
            "329E0924", "CB461B1B", "402BCBCD", "10500A45", "7B349779", "F9E75ED3", "9F64F894",
            "7E924AB9", "1C52E0D1", "7F3DE7F9", "A389329C", "0F933B5B", "4A35E64E", "31CB8137",
            "B80A49F0", "EE6E7316", "106E9C97", "2EAA6FC0", "ADB55FD4", "542A57D9", "F88F739C",
        ];
        for (index, hash) in hashes.iter().enumerate() {
            let episode = index + 1;
            let name =
                format!("[Commie] 3-gatsu no Lion - {episode:02} [BD 720p AAC] [{hash}].srt");
            assert_eq!(
                extract_episode_number(&name),
                Some(episode as u32),
                "input: {name}"
            );
        }

        assert_eq!(
            extract_episode_number("[Commie] 3-gatsu no Lion - 01 [BD 720p AAC].srt"),
            Some(1)
        );
    }

    #[test]
    fn find_matching_video_hit_and_miss() {
        let videos = vec![
            (PathBuf::from("/v/Show E01.mkv"), 1),
            (PathBuf::from("/v/Show E02.mkv"), 2),
        ];
        assert_eq!(
            find_matching_video(&videos, 2),
            Some(&PathBuf::from("/v/Show E02.mkv"))
        );
        assert_eq!(find_matching_video(&videos, 99), None);
        let empty: Vec<(PathBuf, u32)> = vec![];
        assert_eq!(find_matching_video(&empty, 1), None);
    }

    #[test]
    fn process_folder_shifts_and_renames_to_video() {
        let dir = unique_temp_dir("rename");
        fs::write(dir.join("Show E01.mkv"), b"video").unwrap();
        fs::write(
            dir.join("sub E01.srt"),
            "1\n00:00:01,000 --> 00:00:02,000\nHello\n",
        )
        .unwrap();

        process_folder(&dir, 1000).expect("process_folder should succeed");

        assert!(!dir.join("sub E01.srt").exists());
        let renamed = fs::read_to_string(dir.join("Show E01.srt")).expect("renamed file");
        assert!(
            renamed.contains("00:00:02,000 --> 00:00:03,000"),
            "got:\n{renamed}"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn process_folder_without_video_uses_shifted_prefix() {
        let dir = unique_temp_dir("shifted");
        fs::write(
            dir.join("lone E05.srt"),
            "1\n00:00:01,000 --> 00:00:02,000\nHi\n",
        )
        .unwrap();

        process_folder(&dir, 500).expect("process_folder should succeed");

        assert!(!dir.join("lone E05.srt").exists());
        let out = fs::read_to_string(dir.join("shifted_lone E05.srt")).expect("shifted file");
        assert!(out.contains("00:00:01,500 --> 00:00:02,500"), "got:\n{out}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn process_folder_rerun_does_not_delete_renamed_subtitle() {
        let dir = unique_temp_dir("rerun");
        fs::write(dir.join("Show E01.mkv"), b"video").unwrap();
        fs::write(
            dir.join("sub E01.srt"),
            "1\n00:00:01,000 --> 00:00:02,000\nHello\n",
        )
        .unwrap();

        process_folder(&dir, 0).expect("first run should succeed");
        assert!(dir.join("Show E01.srt").exists());

        let report = process_folder(&dir, 0).expect("second run should succeed");
        let content = fs::read_to_string(dir.join("Show E01.srt")).expect("renamed file");
        assert!(content.contains("Hello"), "got:\n{content}");
        assert_eq!(report.already_named, 1);
        assert!(report.warnings.is_empty());
        assert!(report.failures.is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn process_folder_duplicate_episode_does_not_overwrite() {
        let dir = unique_temp_dir("duplicate");
        fs::write(dir.join("Show E01.mkv"), b"video").unwrap();
        fs::write(
            dir.join("first E01.srt"),
            "1\n00:00:01,000 --> 00:00:02,000\nFirst\n",
        )
        .unwrap();
        fs::write(
            dir.join("second E01.srt"),
            "1\n00:00:01,000 --> 00:00:02,000\nSecond\n",
        )
        .unwrap();

        let report = process_folder(&dir, 1000).expect("process_folder should succeed");

        assert!(dir.join("Show E01.srt").exists());
        assert!(
            report
                .warnings
                .iter()
                .any(|(_, reason)| reason.contains("already exists")),
            "expected target-exists warning, got: {:?}",
            report.warnings
        );
        let mut srt_paths = Vec::new();
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) == Some("srt") {
                srt_paths.push(path);
            }
        }
        assert_eq!(srt_paths.len(), 2, "expected two subtitle files");
        assert!(
            srt_paths.iter().any(|path| path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .starts_with("shifted_")),
            "expected one shifted file, got: {srt_paths:?}"
        );

        let combined = srt_paths
            .iter()
            .map(|path| fs::read_to_string(path).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(combined.contains("First"), "got:\n{combined}");
        assert!(combined.contains("Second"), "got:\n{combined}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn process_folder_no_episode_uses_shifted_prefix() {
        let dir = unique_temp_dir("noepisode");
        fs::write(
            dir.join("random.srt"),
            "1\n00:00:01,000 --> 00:00:02,000\nHi\n",
        )
        .unwrap();

        process_folder(&dir, 500).expect("process_folder should succeed");

        assert!(!dir.join("random.srt").exists());
        let out = fs::read_to_string(dir.join("shifted_random.srt")).expect("shifted file");
        assert!(out.contains("00:00:01,500 --> 00:00:02,500"), "got:\n{out}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn process_folder_shifted_prefix_does_not_stack() {
        let dir = unique_temp_dir("nodouble");
        fs::write(
            dir.join("shifted_lone E05.srt"),
            "1\n00:00:01,000 --> 00:00:02,000\nHi\n",
        )
        .unwrap();

        process_folder(&dir, 1000).expect("first run should succeed");
        process_folder(&dir, 1000).expect("second run should succeed");

        assert!(dir.join("shifted_lone E05.srt").exists());
        for entry in fs::read_dir(&dir).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().to_string();
            assert!(
                !name.contains("shifted_shifted_"),
                "unexpected stacked prefix: {name}"
            );
        }

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn process_folder_reports_invalid_utf8_and_continues() {
        let dir = unique_temp_dir("invalid");
        fs::write(dir.join("Show E02.mkv"), b"video").unwrap();
        fs::write(
            dir.join("ok E02.srt"),
            "1\n00:00:01,000 --> 00:00:02,000\nHello\n",
        )
        .unwrap();
        fs::write(dir.join("broken E01.srt"), [0xff, 0xfe, 0xfd]).unwrap();

        let report = process_folder(&dir, 0).expect("process_folder should succeed");

        assert_eq!(report.processed, 2);
        assert_eq!(report.renamed, 1);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].0, "broken E01.srt");
        assert!(
            report.failures[0].1.contains("failed to read"),
            "got: {:?}",
            report.failures
        );
        assert!(dir.join("Show E02.srt").exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn process_folder_warns_on_no_matching_video() {
        let dir = unique_temp_dir("nomatchwarn");
        fs::write(
            dir.join("lone E05.srt"),
            "1\n00:00:01,000 --> 00:00:02,000\nHi\n",
        )
        .unwrap();

        let report = process_folder(&dir, 500).expect("process_folder should succeed");

        assert_eq!(report.shifted_only, 1);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].0, "lone E05.srt");
        assert!(
            report.warnings[0]
                .1
                .contains("no matching video for episode 5"),
            "got: {:?}",
            report.warnings
        );
        assert!(
            report.warnings[0].1.contains("shifted_lone E05.srt"),
            "got: {:?}",
            report.warnings
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn process_folder_warns_on_no_episode() {
        let dir = unique_temp_dir("noepisodewarn");
        fs::write(
            dir.join("random.srt"),
            "1\n00:00:01,000 --> 00:00:02,000\nHi\n",
        )
        .unwrap();

        let report = process_folder(&dir, 500).expect("process_folder should succeed");

        assert_eq!(report.warnings.len(), 1);
        assert!(
            report.warnings[0].1.contains("no episode detected"),
            "got: {:?}",
            report.warnings
        );
        assert!(dir.join("shifted_random.srt").exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn process_folder_rejects_invalid_dir() {
        let missing = std::env::temp_dir().join(format!(
            "subtitle_tool_missing_{}_{}",
            std::process::id(),
            TEST_DIR_COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        assert!(process_folder(&missing, 1000).is_err());
    }
}
