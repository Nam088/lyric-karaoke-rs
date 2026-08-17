# Lyric Karaoke (Rust)

Terminal karaoke player written in Rust. Features smooth per-grapheme text animation, interactive mouse and keyboard transport, audio spectrum visualizer, pitch detection, and zero-delay clock-derived animations.

```
╭────────────────────────────────────────────────────────────────────────────╮
│     🎤 Karaoke                                    ● LIVE │ 01:25.285     │
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
│        01:23 ╶━━━━━━━━━━━━━━●───────────────────────────────╴ 04:27        │
│                             |◄      ▌▌      ►|                             │
╰────────────────────────────────────────────────────────────────────────────╯
```

---

## 🚀 Hướng Dẫn Cài Đặt (Installation)

### 1. Yêu cầu hệ thống (Prerequisites)

* **Rust toolchain** (Cargo & rustc 2024 edition hoặc mới hơn):
  * **macOS / Linux:**
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
  * **Windows:** Tải installer từ [rustup.rs](https://rustup.rs/)

* **Hệ điều hành:**
  * **macOS:** Không cần cài thêm thư viện hệ thống nào (sử dụng CoreAudio mặc định).
  * **Windows:** Không cần cài thêm thư viện nào.
  * **Linux:** Cài đặt ALSA development headers:
    ```bash
    # Debian / Ubuntu / Mint:
    sudo apt-get update && sudo apt-get install -y libasound2-dev pkg-config
    # Fedora / RHEL:
    sudo dnf install alsa-lib-devel
    # Arch Linux:
    sudo pacman -S alsa-lib
    ```

---

### 2. Cài đặt và Chạy (Build & Run)

```bash
# Clone repository
git clone https://github.com/your-username/lyric-karaoke-rs.git
cd lyric-karaoke-rs

# Cài đặt file nhạc và lời bài hát vào thư mục data/
#   data/a.mp3 (hoặc file .mp3 bài hát bạn muốn)
#   data/lr.json (file lyrics định dạng LRC/JSON)

# Chạy ứng dụng:
cargo run --release
```

---

## ⚙️ Cấu Hình (Configuration)

Tất cả cấu hình nằm trong file [`src/config.rs`](src/config.rs):

```rust
// Tên bài hát & File nhạc trong data/
pub const SONG_NAME: &str = "Tìm Em - Hngle, Bảo Anh";
pub const SONG_FILE: &str = "a.mp3";
pub const LYRIC_JSON: &str = "data/lr.json";

// Giao diện
pub const DEFAULT_THEME: ThemePreset = ThemePreset::Emerald; // Emerald | Cyberpunk | Ocean | Sunset | Sakura | Mono | Light
pub const SHOW_SPECTRUM: bool = false; // Bật/tắt spectrum khi khởi động
pub const SHOW_NOTE: bool = false;     // Bật/tắt hiển thị nốt nhạc phát hiện
pub const SHOW_KEYBINDS: bool = false; // Bật/tắt thanh hướng dẫn phím
pub const LINE_SPACING: usize = 0;     // 0 = các dòng lyric liền kề, 1 = cách 1 dòng trống
```

---

## 🎮 Điều Khiển (Controls)

### Bằng bàn phím:
| Phím (Key) | Chức năng (Action) |
| --- | --- |
| `Space` | Play / Pause bài hát |
| `←` / `→` | Tua lùi / Tua tới 5 giây (±5s) |
| `[` hoặc `P` | Chuyển về Bài Hát Trước (Previous Track) |
| `]` hoặc `O` | Chuyển sang Bài Hát Tiếp Theo (Next Track) |
| `C` | Đổi Bảng Màu Theme Preset (`Emerald` ➔ `Cyberpunk` ➔ `Ocean` ➔ `Sunset` ➔ `Sakura` ➔ `Mono` ➔ `Light`) |
| `S` | Đổi kiểu Visualizer Spectrum (`Curve` ➔ `Mirror` ➔ `Line` ➔ `Bars` ➔ `Off`) |
| `N` | Bật / Tắt hiển thị Nốt nhạc (Pitch Detection & Cents offset) |
| `H` | Bật / Tắt thanh hướng dẫn phím tắt ở header |
| `Q` / `Esc` | Thoát ứng dụng |

### Bằng chuột:
* **Click vào bất kỳ dòng lyric nào**: Lập tức tua đến câu hát đó.
* **Click vào nút `|◄`**: Tua về đầu câu/bài hát, hoặc lùi về bài trước.
* **Click vào nút `▌▌` / `►`**: Play / Pause.
* **Click vào nút `►|`**: Nhảy sang câu hát tiếp theo hoặc chuyển bài.
* **Click vào thanh Timeline**: Nhảy ngay đến vị trí mong muốn.
* **Click vào chữ `● LIVE` / `⏸ PAUSED`**: Play / Pause.

---

## 📑 Danh Sách Phát (Playlist Support)

Ứng dụng hỗ trợ phát nhiều bài hát liên tục thông qua danh sách trong [`data/playlist.json`](data/playlist.json):

```json
[
    {
        "id": "mua-he-nam-ay",
        "title": "Mùa Hè Năm Ấy",
        "artist": "Vũ",
        "audio": "c.mp3",
        "lyrics": "c.json"
    },
    {
        "id": "nothing-without-you",
        "title": "NOTHING WITHOUT YOU",
        "artist": "JUNE",
        "audio": "a1.mp3",
        "lyrics": "a1.json"
    }
]
```

* **Tự động chuyển bài (Auto-advance)**: Khi bài hát kết thúc, player sẽ tự động chuyển sang bài tiếp theo trong danh sách.
* **Tự động thêm bài hát mới**: Dùng tool `audio-aligner/batch_add_song.py` để tự động bóc tách từ và nạp bài mới vào playlist:
  ```bash
  cd ../audio-aligner
  ./venv/bin/python batch_add_song.py /path/to/song.mp3 --lyrics /path/to/lyric.txt --title "Tên Bài" --artist "Ca Sĩ" -l vi
  ```

---

## ✨ Tính Năng Nổi Bật (Features)

* **7 Color Theme Presets**: Chuyển đổi linh hoạt giữa các phong cách màu sắc (`Emerald`, `Cyberpunk`, `Ocean`, `Sunset`, `Sakura`, `Mono`, `Light` dành cho terminal nền trắng) chỉ với 1 phím bấm `C`.
* **Zero-delay Clock-derived Animation**:
  * **Breathing Glow**: Dòng đang hát phát sáng nhịp nhàng.
  * **Wave Ripple**: Hiệu ứng sóng nhẹ lướt qua các từ chuẩn bị hát.
  * **Twinkling Gap**: Các ký tự nốt nhạc ♫♪ lấp lánh khi đến đoạn dạo nhạc.
  * **Per-char Shimmer**: Ánh sáng quét qua từng ký tự tiêu đề `Karaoke`.
* **Interactive Seeking**: Tua nhạc mượt mà ngay cả khi bài hát đã chạy hết (tự động reload stream không crash).
* **Audio Visualizer**: Đo theo tiêu chuẩn IEC 61260 Fractional Octave Bands (40Hz - 16kHz) với 4 phong cách hiển thị khác nhau.
* **Pitch Detection**: Nhận diện cao độ nốt nhạc và độ lệch (cents) theo thời gian thực.
* **Unicode & Grapheme cluster aware**: Tương thích hoàn hảo với tiếng Việt có dấu (NFC / NFD decomposition).

---

## 🧪 Kiểm Thử (Testing)

```bash
# Chạy toàn bộ 100 unit tests:
cargo test

# Kiểm tra code style & linting:
cargo clippy --all-targets
```

License: MIT

