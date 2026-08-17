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
| `L` | Mở / Đóng Hộp Thoại Danh Sách Phát (Playlist Modal) |
| `F` | Mở Trực Tiếp Hộp Thoại Quản Lý Thư Mục (Folder Manager) |
| `1` / `2` hoặc `T` / `F` | Chuyển Tab giữa **[1] Bài Hát (Tracks)** và **[2] Thư Mục (Folders)** |
| `A` *(trong tab Folder)* | **Thêm Thư Mục Nhạc Mới**: Nhập đường dẫn thư mục và nhấn `Enter` để lưu vào SQLite |
| `D` *(trong tab Folder)* | **Xóa Thư Mục**: Xóa thư mục đang chọn khỏi SQLite |
| `R` *(trong tab Folder)* | **Quét Lại (Rescan)**: Quét lại tất cả thư mục để cập nhật bài hát mới nhất |
| `↑` / `↓` hoặc `J` / `K` | Di chuyển chọn bài hát / chọn thư mục trong Modal |
| `Enter` | Phát bài hát đang chọn / Xác nhận thêm thư mục |
| `I` | Đổi Ngôn Ngữ i18n (`Tiếng Việt (VI)` ➔ `English (EN)`) |
| `C` | Đổi Bảng Màu Theme Preset (`Emerald` ➔ `Cyberpunk` ➔ `Ocean` ➔ `Sunset` ➔ `Sakura` ➔ `Mono` ➔ `Light`) |
| `S` | Đổi kiểu Visualizer Spectrum (`Curve` ➔ `Mirror` ➔ `Line` ➔ `Bars` ➔ `Off`) |
| `N` | Bật / Tắt hiển thị Nốt nhạc (Pitch Detection & Cents offset) |
| `H` | Bật / Tắt thanh hướng dẫn phím tắt ở header |
| `Q` / `Esc` | Đóng Modal / Hủy nhập / Thoát ứng dụng |

### Bằng chuột trên thanh Action Bar:
* **`|◀◀` (Previous Track)**: Chuyển về bài hát trước đó trong Playlist.
* **`|◀` (Previous Line)**: Tua về đầu câu hát / bài hát.
* **`▌▌` / `►` (Play / Pause)**: Phát hoặc tạm dừng bài hát.
* **`▶|` (Next Line)**: Nhảy nhanh sang câu hát tiếp theo.
* **`▶▶|` (Next Track)**: Chuyển sang bài hát tiếp theo trong Playlist.
* **`☰` (Playlist Modal)**: Bật/tắt cửa sổ danh sách phát để click chọn bài trực quan.
* **Click trực tiếp vào câu lyric hoặc thanh Timeline**: Tua nhanh đến đúng thời điểm mong muốn.

---

## 🌐 Đa Ngôn Ngữ (i18n Support qua JSON)

Toàn bộ ngôn ngữ giao diện được cấu hình hoàn toàn độc lập qua các file JSON tại thư mục [`locales/`](locales/):
* [`locales/vi.json`](locales/vi.json): Cấu hình tiếng Việt.
* [`locales/en.json`](locales/en.json): Cấu hình tiếng Anh.

Bấm phím **`I`** trong khi đang chạy để chuyển đổi ngôn ngữ tức thì!

---

## 🗄️ Quản Lý Bài Hát Bằng SQLite & Tự Động Quét Thư Mục (Folder Auto-Scan)

Ứng dụng tích hợp sẵn cơ sở dữ liệu **SQLite (`library.db`)** và engine tự động quét mọi file nhạc trong thư mục:

### 🌟 Tính năng nổi bật:
1. **Không bắt buộc phải có file Lyric**: Bạn có thể thả bất kỳ file nhạc nào (`.mp3`, `.wav`, `.ogg`, `.flac`, `.m4a`) vào thư mục `data/`. Nếu không có file `.json` hoặc `.lrc`, ứng dụng **vẫn phát nhạc bình thường** và hiển thị hiệu ứng sóng âm Spectrum + Waveform Envelope.
2. **Tự động bóc tách Tên bài hát & Ca sĩ**: Tự động phân tích từ tên file (ví dụ: `Son Tung M-TP - Dung Ve Tre.mp3` ➔ Artist: *Son Tung M-TP*, Title: *Dung Ve Tre*).
3. **Tự động liên kết Lyric**: Nếu có file lyric cùng tên (`song.json` hoặc `song.lrc`), player sẽ tự động nạp lời.
4. **Hỗ trợ phát bất kỳ thư mục nào qua CLI**:
   ```bash
   cargo run -- /path/to/my-music
   ```

---

## 📑 Danh Sách Phát (Playlist Support)

Ứng dụng hỗ trợ phát nhiều bài hát liên tục thông qua SQLite Database hoặc danh sách trong [`data/playlist.json`](data/playlist.json):

* **Tự động chuyển bài (Auto-advance)**: Khi bài hát kết thúc, player sẽ tự động chuyển sang bài tiếp theo trong danh sách.
* **Tự động bóc tách từ & căn chỉnh lyric**: Dùng tool `audio-aligner/batch_add_song.py` để tự động nạp lời chuẩn:
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

