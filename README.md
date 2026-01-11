# SubSync

A fast command-line tool for batch processing subtitles: synchronize timing and automatically rename subtitle files to match your video files.

## Features

- ⚡ **Batch Processing** - Process entire folders of subtitles at once
- 🎯 **Precise Timing** - Shift timestamps with millisecond precision
- 🔄 **Auto-Rename** - Automatically matches and renames subtitles to video filenames
- 📝 **Multi-Format** - Supports both .srt and .ass subtitle formats
- 🎬 **Smart Matching** - Extracts episode numbers from various naming conventions

## Installation

### Prerequisites
- Rust toolchain (install from [rustup.rs](https://rustup.rs/))

### Build from source

```bash
# Clone or download the source
git clone <your-repo-url>
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

SubSync intelligently extracts episode numbers from various naming patterns:

- `E01`, `E001`
- `ep01`, `episode01`
- `- 01`, `- 001`
- Case-insensitive matching

## Timing

- Positive values (e.g., `2.5`) shift subtitles later
- Negative values (e.g., `-5.43`) shift subtitles earlier
- Supports millisecond precision
- Prevents negative timestamps (clamps to 0)

## Disclaimer

- This tool processes subtitle files only and does NOT modify, transcode, or re-encode video files in any way. The video files remain completely untouched with their original codecs intact.

- SubSync is designed for personal use to help synchronize subtitle timing for language learning and accessibility purposes. Users are responsible for ensuring they have legal rights to use the subtitle and video files they process.

- This tool was AI-generated (vibe-coded). While functional, users should review the code and test with backup files before use on important data.

