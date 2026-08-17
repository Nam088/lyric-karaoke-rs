# 🎧 Sound Player (Rust)

A powerful, high-performance terminal music player and visualizer written in Rust. Features smooth per-grapheme lyric animations, SQLite music library management, dynamic folder scanning, audio spectrum visualizer, pitch detection, and zero-delay clock-derived animations.

```text
╭────────────────────────────────────────────────────────────────────────────╮
│     🎧 Sound Player                               ● LIVE │ 01:25.285     │
│     ──────────────────────────────────────────────────────────────────     │
│                       Đừng về trễ nha em yêu dấu ơi!                       │
│                             ♫  ♪  ♫  ♪  ♫  ♪  ♫                            │
│                              Đừng về trễ nha!                              │
│                    ♬  Đừng về trễ nha em yêu dấu hỡi!  ♬                   │
│                         Đừng trở về vào ngày mai..                         │
│                Khi cô đơn bủa vây yêu thương trong nuối tiếc               │
│     ──────────────────────────────────────────────────────────────────     │
│             SƠN TÙNG M-TP      •      ĐỪNG VỀ TRỄ (RNB VERSION)            │
│     ⠀⣀⣀⣀⣀⣀⣀⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⠀⢀⣀⠀⠀⠀⠀⠀⠀⢀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀     │
│     ⡲⡪⡪⡪⡪⡲⡲⡲⣚⢝⢝⡲⡲⣀⣀⣀⣘⢝⢝⢝⡪⠵⡪⡪⠴⣂⠀⡲⣀⡂⡪⣢⣢⢀⢘⠅⠀⢀⠄⠀⣢⡀⠄⠀     │
│     ⡪⡪⡪⡪⡪⡪⡪⡪⣒⢕⢕⡪⡪⡪⡪⡪⣒⢕⢕⢕⡪⠭⡪⡪⠭⡪⣚⡪⡪⣚⡪⣒⣒⠬⢔⢥⠴⠬⢅⡲⣒⣢⠵⡰     │
│        01:23   ━━━━━━━━━━━━━━●───────────────────────────────   04:27       │
│                             |◄      ▌▌      ►|                             │
╰────────────────────────────────────────────────────────────────────────────╯
```

---

## ✨ Features

* **Real-time Lyric Synchronization**: Smooth per-grapheme singing animations with breathing glow, anticipation hints, wave ripples, and afterglow decay.
* **Smart Instrumental Gap Detection**: Automatically detects long instrumental breaks and displays dancing musical notations (`♫  ♪  ♫`).
* **SQLite Library & Dynamic Folder Scanning**:
  * Persistent music folders configured in SQLite (`library.db`).
  * Automatic directory scanning for all standard audio formats (`.mp3`, `.wav`, `.flac`, `.ogg`, `.m4a`).
  * Automatic discovery of companion lyric files (`.json`, `.lrc`).
  * Instant folder rescan via hotkey `[R]` or UI button.
* **7 Curated Color Themes**: Live cycling with the `C` key (`Emerald`, `Cyberpunk`, `Ocean`, `Sunset`, `Sakura`, `Mono`, `Light`).
* **Audio Visualizer**: Real-time IEC 61260 Fractional Octave Bands (40Hz - 16kHz) with 4 rendering modes (`Curve`, `Mirror`, `Line`, `Bars`, `Off`).
* **Real-time Pitch & Cents Offset Readout**: Fundamental pitch detection with musical note and cents deviation.
* **Multi-language Support (i18n)**: Instant language switching with `I` key (`English`, `Tiếng Việt`).
* **Interactive Transport Controls**: Full mouse and keyboard seeking, line skipping, and track selection.
* **Full Unicode & Vietnamese Diacritics Support**: NFC/NFD decomposition safe text rendering.

---

## 🚀 Installation & Getting Started

### 1. Prerequisites

* **Rust toolchain** (Cargo & rustc 2024 edition or newer):
  * **macOS / Linux:**
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
  * **Windows:** Download installer from [rustup.rs](https://rustup.rs/)

* **System Dependencies:**
  * **macOS:** No additional dependencies needed (uses native CoreAudio).
  * **Windows:** No additional dependencies needed.
  * **Linux:** Install ALSA development libraries:
    ```bash
    # Debian / Ubuntu / Mint:
    sudo apt-get update && sudo apt-get install -y libasound2-dev pkg-config
    # Fedora / RHEL:
    sudo dnf install alsa-lib-devel
    # Arch Linux:
    sudo pacman -S alsa-lib
    ```

---

### 2. Build & Run

```bash
# Clone the repository
git clone https://github.com/Nam088/sound-player.git
cd sound-player

# Place your songs and lyrics into data/ directory:
#   data/my_song.mp3
#   data/my_song.json (optional)

# Run the player:
cargo run --release
```

#### Play a specific directory directly:
```bash
cargo run --release -- /path/to/my/music/folder
```

---

## 🎮 Keyboard & Mouse Controls

### Keyboard Shortcuts:
| Key | Action |
| :--- | :--- |
| `Space` | Play / Pause playback |
| `←` / `→` | Seek backward / forward 5 seconds (±5s) |
| `[` / `P` | Previous Track in playlist |
| `]` / `O` | Next Track in playlist |
| `L` | Open / Close Playlist Modal Dialog |
| `F` | Open Music Folders Manager Dialog directly |
| `R` | **Rescan & Reload**: Scan all configured music folders for new songs |
| `1` / `2` or `T` / `F` | Switch Modal Tab between **[1] Tracks** and **[2] Folders** |
| `A` *(in Folders Tab)* | **Add Music Folder**: Enter a path and hit `Enter` to save to SQLite |
| `D` *(in Folders Tab)* | **Delete Folder**: Remove selected folder (with `Y`/`N` confirmation) |
| `↑` / `↓` or `J` / `K` | Navigate tracks / folders in modal |
| `←` / `→` or `PgUp` / `PgDn`| Pagination / Jump pages in modal |
| `Enter` | Play selected track / Confirm folder addition |
| `I` | Cycle UI Language (`English (EN)` ↔ `Tiếng Việt (VI)`) |
| `C` | Cycle Color Theme Preset |
| `S` | Cycle Visualizer Style (`Curve` → `Mirror` → `Line` → `Bars` → `Off`) |
| `N` | Toggle Pitch & Cents Readout in header |
| `H` | Toggle Keybindings Guide in header |
| `Q` / `Esc` | Close Modal / Cancel Input / Quit Application |

### Mouse Controls (Action Bar):
* **`|◀◀` (Previous Track)**: Jump to previous song.
* **`|◀` (Previous Line)**: Seek to the beginning of the current lyric line.
* **`▌▌` / `►` (Play / Pause)**: Toggle playback.
* **`▶|` (Next Line)**: Jump to the next lyric line.
* **`▶▶|` (Next Track)**: Jump to the next song in playlist.
* **`☰` (Playlist)**: Open interactive playlist modal.
* **Click on Lyric Line or Timeline Bar**: Instant interactive seek.

---

## 🌐 Localization (i18n)

UI language strings are configured via JSON files in the [`locales/`](locales/) directory:
* [`locales/en.json`](locales/en.json): English translations.
* [`locales/vi.json`](locales/vi.json): Vietnamese translations.

Press **`I`** during playback to switch languages on the fly!

---

## 🧪 Testing

```bash
# Run all unit tests:
cargo test

# Run code style & linter checks:
cargo clippy --all-targets
```

---

## 📄 License

This project is licensed under the MIT License.
