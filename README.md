# SubSync

A fast command-line tool for batch processing subtitles: synchronize timing and automatically rename subtitle files to match your video files.

## Features

- ⚡ **Batch Processing** - Process entire folders of subtitles at once
- 🎯 **Precise Timing** - Shift timestamps with millisecond precision
- 🔄 **Auto-Rename** - Automatically matches and renames subtitles to video filenames to automatically add the subtitle when playing the video
- 📝 **Multi-Format** - Supports both .srt and .ass subtitle formats
- 🎬 **Smart Matching** - Extracts episode numbers from various naming conventions

## Installation

### Prerequisites
- Rust toolchain (install from [rustup.rs](https://rustup.rs/))

### Build from source

```bash
# Clone or download the source
git clone git@github.com:ClemPera/SubSync.git
cd subsync

# Build release version
cargo build --release

# The binary will be at target/release/subsync
```

Alternatively, compile directly:
```bash
rustc --edition 2021 -O subsync.rs -o subsync
```
*Note: Direct compilation requires the `regex` crate to be available in your system.*

## Usage

```bash
subsync <folder_path> <shift_seconds>
```

### Arguments

- `<folder_path>` - Path to folder containing video and subtitle files
- `<shift_seconds>` - Time to shift subtitles (supports decimals, negative values shift earlier)

### Examples

Shift subtitles 5.43 seconds earlier:
```bash
subsync ./episodes -5.43
```

Shift subtitles 2 seconds later:
```bash
subsync /path/to/anime 2.0
```

## How It Works

1. **Scans** the specified folder for video files (.mkv, .mp4, .avi) and subtitle files (.srt, .ass)
2. **Extracts** episode numbers from filenames using intelligent pattern matching
3. **Shifts** all subtitle timestamps by your specified amount
4. **Renames** subtitles to match corresponding video files
5. **Outputs** new subtitle files ready to use

### Example

**Before:**
- Video: `Dragon.Ball.Z.Kai.E01.MULTi.1080p.BluRay.x265-KHAYA.mkv`
- Subtitle: `[Anime Time] Dragon Ball Z Kai - 001.jpn.ass`

**After:**
- Video: `Dragon.Ball.Z.Kai.E01.MULTi.1080p.BluRay.x265-KHAYA.mkv`
- Subtitle: `Dragon.Ball.Z.Kai.E01.MULTi.1080p.BluRay.x265-KHAYA.ass` ✨

Your video player will now automatically load the subtitles!

**Note:** If no matching video file is found for a subtitle, it will still be processed and saved with a `shifted_` prefix.

## Supported Formats

### Video Files
- .mkv
- .mp4
- .avi

### Subtitle Files
- .srt (SubRip)
- .ass (Advanced SubStation Alpha)

## Episode Number Detection

SubSync extracts episode numbers from common naming patterns, case-insensitively:

- `S01E02`, `S1E2`
- `E01`, `E001`, `EP12`
- `episode 2`, `episode_03`
- trailing `- 02`, `_004`

Bracket hashes and CRC tags such as `[329E0924]` or `[7F3DE7F9]` are ignored and never mistaken for an episode number.

## Safe by default

- Rerunning on the same folder is safe: a subtitle already named after its video is shifted in place and never deleted.
- Existing subtitle files are never overwritten. If the target name is taken, the subtitle is saved under a unique `shifted_` name instead.
- Subtitle files with no detectable episode are still shifted and saved with a `shifted_` prefix.

## Output and failures

Every run ends with a summary of renamed, shifted-only, already-named, and failed files. Subtitles that could not be renamed are listed with their reason, and read or write errors are listed separately.

Warnings such as an unmatched episode still exit with status 0. Hard processing errors exit with status 1.

## Timing

- Positive values (e.g., `2.5`) shift subtitles later
- Negative values (e.g., `-5.43`) shift subtitles earlier
- Supports millisecond precision
- Prevents negative timestamps (clamps to 0)

## Disclaimer

- This tool processes subtitle files only and does NOT modify, transcode, or re-encode video files in any way. The video files remain completely untouched with their original codecs intact.

- SubSync is designed for personal use to help synchronize subtitle timing for language learning and accessibility purposes. Users are responsible for ensuring they have legal rights to use the subtitle and video files they process.

- This tool was AI-generated (vibe-coded). While functional, users should review the code and test with backup files before use on important data.

