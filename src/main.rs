// SubSync - Subtitle Synchronization & Batch Renaming Tool
// A tool for shifting subtitle timestamps and renaming them to match video files

use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;

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
    
    format!("{}:{:02}:{:02}.{:02}", hours, minutes, seconds, centiseconds)
}

fn shift_srt(content: &str, shift_ms: i64) -> String {
    let mut result = String::new();
    
    for line in content.lines() {
        if line.contains(" --> ") {
            let parts: Vec<&str> = line.split(" --> ").collect();
            if parts.len() == 2 {
                if let (Some(start_ms), Some(end_ms)) = (parse_timestamp_srt(parts[0]), parse_timestamp_srt(parts[1])) {
                    let new_start = (start_ms + shift_ms).max(0);
                    let new_end = (end_ms + shift_ms).max(0);
                    result.push_str(&format!("{} --> {}\n", format_timestamp_srt(new_start), format_timestamp_srt(new_end)));
                    continue;
                }
            }
        }
        result.push_str(line);
        result.push('\n');
    }
    
    result
}

fn shift_ass(content: &str, shift_ms: i64) -> String {
    let dialogue_re = Regex::new(r"^(Dialogue: \d+,)(\d+:\d+:\d+\.\d+),(\d+:\d+:\d+\.\d+),(.+)$").unwrap();
    let mut result = String::new();
    
    for line in content.lines() {
        if let Some(caps) = dialogue_re.captures(line) {
            let prefix = &caps[1];
            let start = &caps[2];
            let end = &caps[3];
            let rest = &caps[4];
            
            if let (Some(start_ms), Some(end_ms)) = (parse_timestamp_ass(start), parse_timestamp_ass(end)) {
                let new_start = (start_ms + shift_ms).max(0);
                let new_end = (end_ms + shift_ms).max(0);
                result.push_str(&format!("{}{},{},{}\n", prefix, format_timestamp_ass(new_start), format_timestamp_ass(new_end), rest));
                continue;
            }
        }
        result.push_str(line);
        result.push('\n');
    }
    
    result
}

fn extract_episode_number(filename: &str) -> Option<u32> {
    // Try multiple patterns to match various naming conventions
    let patterns = vec![
        r"(?i)e(\d+)",           // E01, e01
        r"(?i)ep(\d+)",          // EP01, ep01
        r"(?i)episode[_\s]*(\d+)", // episode01, episode 01
        r"[\s\-_](\d{2,3})(?:\.|$|[\s\-_])", // - 001, _001, 001.
    ];
    
    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(filename) {
            if let Some(num) = caps.get(1).and_then(|m| m.as_str().parse().ok()) {
                return Some(num);
            }
        }
    }
    
    None
}

fn find_matching_video(video_files: &[(PathBuf, u32)], episode: u32) -> Option<&PathBuf> {
    video_files.iter()
        .find(|(_, ep)| *ep == episode)
        .map(|(path, _)| path)
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
        eprintln!("Error: '{}' is not a valid directory", folder_path.display());
        std::process::exit(1);
    }
    
    println!("Scanning folder: {}", folder_path.display());
    println!("Time shift: {} seconds ({} ms)\n", shift_seconds, shift_ms);
    
    let entries = fs::read_dir(folder_path).expect("Failed to read directory");
    
    let mut video_files = Vec::new();
    let mut subtitle_files = Vec::new();
    
    for entry in entries {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_str().unwrap_or("").to_lowercase();
            let filename = path.file_name().unwrap().to_str().unwrap_or("");
            
            if let Some(episode) = extract_episode_number(filename) {
                match ext_str.as_str() {
                    "mkv" | "mp4" | "avi" => {
                        video_files.push((path.clone(), episode));
                    }
                    "srt" | "ass" => {
                        subtitle_files.push((path.clone(), episode, ext_str));
                    }
                    _ => {}
                }
            }
        }
    }
    
    println!("Found {} video files", video_files.len());
    println!("Found {} subtitle files\n", subtitle_files.len());
    
    for (sub_path, episode, ext) in subtitle_files {
        println!("Processing: {}", sub_path.file_name().unwrap().to_str().unwrap());
        
        let content = fs::read_to_string(&sub_path).expect("Failed to read subtitle file");
        
        let shifted_content = match ext.as_str() {
            "srt" => shift_srt(&content, shift_ms),
            "ass" => shift_ass(&content, shift_ms),
            _ => content,
        };
        
        if let Some(video_path) = find_matching_video(&video_files, episode) {
            let video_stem = video_path.file_stem().unwrap().to_str().unwrap();
            let new_name = format!("{}.{}", video_stem, ext);
            let new_path = folder_path.join(&new_name);
            
            fs::write(&new_path, shifted_content).expect("Failed to write file");
            fs::remove_file(&sub_path).expect("Failed to remove original file");
            println!("  ✓ Shifted and renamed to: {}", new_name);
        } else {
            let new_name = format!("shifted_{}", sub_path.file_name().unwrap().to_str().unwrap());
            let new_path = folder_path.join(&new_name);
            
            fs::write(&new_path, shifted_content).expect("Failed to write file");
            fs::remove_file(&sub_path).expect("Failed to remove original file");
            println!("  ✓ Shifted (no matching video found): {}", new_name);
        }
    }
    
    println!("\n✓ All done!");
}