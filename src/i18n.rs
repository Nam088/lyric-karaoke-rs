//! Internationalization (i18n) configured via external JSON files.

use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Clone, Debug, Deserialize)]
pub struct HeaderLocale {
    pub play: String,
    pub seek: String,
    pub track: String,
    pub list: String,
    #[serde(default = "default_folders_label")]
    pub folders: String,
    pub spectrum: String,
    pub theme: String,
    pub note: String,
    pub lang: String,
    pub quit: String,
}

fn default_folders_label() -> String {
    "[F] Folder".to_string()
}

#[derive(Clone, Debug, Deserialize)]
pub struct PlaylistLocale {
    pub title: String,
    #[serde(default = "default_folders_title")]
    pub folders_title: String,
    pub close: String,
    pub footer_hints: String,
    #[serde(default = "default_folder_footer_hints")]
    pub folder_footer_hints: String,
    pub live_badge: String,
    #[serde(default = "default_tracks_tab")]
    pub tracks_tab: String,
    #[serde(default = "default_folders_tab")]
    pub folders_tab: String,
    #[serde(default = "default_add_folder_prompt")]
    pub add_folder_prompt: String,
    #[serde(default = "default_add_folder_hints")]
    pub add_folder_hints: String,
    #[serde(default = "default_folder_empty")]
    pub folder_empty: String,
    #[serde(default = "default_folder_count")]
    pub folder_count: String,
}

fn default_folders_title() -> String {
    "QUẢN LÝ THƯ MỤC NHẠC".to_string()
}
fn default_folder_footer_hints() -> String {
    "[↑/↓] Chọn  •  [A] Thêm  •  [D] Xóa  •  [R] Quét lại  •  [T/Esc] Về DS".to_string()
}
fn default_tracks_tab() -> String {
    "Bài hát".to_string()
}
fn default_folders_tab() -> String {
    "Thư mục".to_string()
}
fn default_add_folder_prompt() -> String {
    "Nhập đường dẫn thư mục:".to_string()
}
fn default_add_folder_hints() -> String {
    "[Enter] Xác nhận lưu vào SQLite  •  [Esc] Hủy".to_string()
}
fn default_folder_empty() -> String {
    "Chưa có thư mục nào.".to_string()
}
fn default_folder_count() -> String {
    "bài hát".to_string()
}

#[derive(Clone, Debug, Deserialize)]
pub struct PlayerLocale {
    pub no_lyrics: String,
    pub no_lyrics_hints: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StatusLocale {
    pub live: String,
    pub paused: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LocaleConfig {
    pub code: String,
    pub name: String,
    pub header: HeaderLocale,
    pub playlist: PlaylistLocale,
    pub player: PlayerLocale,
    pub status: StatusLocale,
}

static VI_LOCALE: LazyLock<LocaleConfig> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../locales/vi.json"))
        .expect("invalid locales/vi.json file")
});

static EN_LOCALE: LazyLock<LocaleConfig> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../locales/en.json"))
        .expect("invalid locales/en.json file")
});

/// Supported languages. Cycled with the `I` key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Vietnamese,
    English,
}

impl Language {
    #[allow(dead_code)]
    pub const ALL: [Language; 2] = [Language::Vietnamese, Language::English];

    /// Cycle to the next language.
    pub fn next(self) -> Self {
        match self {
            Language::Vietnamese => Language::English,
            Language::English => Language::Vietnamese,
        }
    }

    /// Access the parsed JSON locale configuration.
    pub fn config(self) -> &'static LocaleConfig {
        match self {
            Language::Vietnamese => &VI_LOCALE,
            Language::English => &EN_LOCALE,
        }
    }

    /// Two-letter language code (e.g. "vi", "en").
    pub fn code(self) -> &'static str {
        &self.config().code
    }

    /// Full human-readable language name.
    #[allow(dead_code)]
    pub fn name(self) -> &'static str {
        &self.config().name
    }

    /// Full keybindings guide rendered in the top header.
    pub fn keybinds_guide(self, spectrum_name: &str, theme_name: &str) -> String {
        let h = &self.config().header;
        let c = self.code().to_uppercase();
        format!(
            "{}  {}  {}  {}  {}  [S] {}: {}  [C] {}: {}  [N] {}  [I] {}: {}  {}",
            h.play,
            h.seek,
            h.track,
            h.list,
            h.folders,
            h.spectrum,
            spectrum_name,
            h.theme,
            theme_name,
            h.note,
            h.lang,
            c,
            h.quit,
        )
    }

    /// Modal header title for the playlist.
    #[allow(dead_code)]
    pub fn playlist_title(self, count: usize) -> String {
        format!(" {} ({})", self.config().playlist.title, count)
    }

    /// Modal header title for the folders manager.
    #[allow(dead_code)]
    pub fn folders_title(self, count: usize) -> String {
        format!(" {} ({})", self.config().playlist.folders_title, count)
    }

    /// Tracks tab label.
    pub fn tracks_tab(self) -> &'static str {
        &self.config().playlist.tracks_tab
    }

    /// Folders tab label.
    pub fn folders_tab(self) -> &'static str {
        &self.config().playlist.folders_tab
    }

    /// Close button label in the playlist modal.
    pub fn playlist_close_btn(self) -> &'static str {
        &self.config().playlist.close
    }

    /// Footer keyboard hint bar inside the playlist modal.
    pub fn playlist_footer_hints(self) -> &'static str {
        &self.config().playlist.footer_hints
    }

    /// Footer keyboard hint bar inside the folder manager modal.
    pub fn folder_footer_hints(self) -> &'static str {
        &self.config().playlist.folder_footer_hints
    }

    /// Add folder prompt.
    pub fn add_folder_prompt(self) -> &'static str {
        &self.config().playlist.add_folder_prompt
    }

    /// Add folder hints.
    pub fn add_folder_hints(self) -> &'static str {
        &self.config().playlist.add_folder_hints
    }

    /// Empty folder notice.
    pub fn folder_empty(self) -> &'static str {
        &self.config().playlist.folder_empty
    }

    /// Folder song count suffix.
    pub fn folder_count(self) -> &'static str {
        &self.config().playlist.folder_count
    }

    /// Live badge text inside playlist modal.
    pub fn live_badge(self) -> &'static str {
        &self.config().playlist.live_badge
    }

    /// Placeholder message when a track has no lyrics file.
    pub fn no_lyrics_text(self) -> &'static str {
        &self.config().player.no_lyrics
    }

    /// Sub-hint when a track has no lyrics file.
    #[allow(dead_code)]
    pub fn no_lyrics_hints(self) -> &'static str {
        &self.config().player.no_lyrics_hints
    }

    /// Live badge text.
    pub fn live_label(self) -> &'static str {
        &self.config().status.live
    }

    /// Paused badge text.
    pub fn paused_label(self) -> &'static str {
        &self.config().status.paused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_cycling() {
        assert_eq!(Language::Vietnamese.next(), Language::English);
        assert_eq!(Language::English.next(), Language::Vietnamese);
    }

    #[test]
    fn language_json_loaded_correctly() {
        assert_eq!(Language::Vietnamese.code(), "vi");
        assert_eq!(Language::English.code(), "en");
        assert_eq!(Language::Vietnamese.name(), "Tiếng Việt");
        assert_eq!(Language::English.name(), "English");
        assert!(Language::Vietnamese.playlist_title(3).contains("DANH SÁCH"));
        assert!(Language::English.playlist_title(3).contains("PLAYLIST"));
        assert!(Language::Vietnamese.no_lyrics_text().contains("không có lời"));
    }
}
