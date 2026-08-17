use std::sync::{Arc, Mutex};

use iocraft::prelude::*;

use super::Session;
use crate::color::Theme;
use crate::i18n::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ModalTab {
    #[default]
    Tracks,
    Folders,
}

#[allow(clippy::too_many_arguments)]
pub fn render<FSelect, FClose, FTabChange>(
    session: Arc<Session>,
    tab: ModalTab,
    cursor: usize,
    folder_cursor: usize,
    is_adding_folder: bool,
    folder_input: &str,
    width: usize,
    lang: Language,
    theme: &Theme,
    on_select: FSelect,
    on_close: FClose,
    on_tab_change: FTabChange,
) -> AnyElement<'static>
where
    FSelect: FnMut(usize) + Send + 'static,
    FClose: FnMut() + Send + 'static,
    FTabChange: FnMut(ModalTab) + Send + 'static,
{
    let playlist = session.playlist.read().unwrap();
    let current_idx = *session.track_index.read().unwrap();
    let is_playing = session.audio.is_playing();

    let on_select = Arc::new(Mutex::new(on_select));
    let on_close = Arc::new(Mutex::new(on_close));
    let on_tab_change = Arc::new(Mutex::new(on_tab_change));

    let (bg_r, bg_g, bg_b) = theme.dark_base;
    let modal_bg = Color::Rgb {
        r: bg_r,
        g: bg_g,
        b: bg_b,
    };

    // ── Tab 1: Tracks List ──
    let track_items: Vec<AnyElement<'static>> = playlist
        .tracks
        .iter()
        .enumerate()
        .map(|(idx, track)| {
            let is_current = idx == current_idx;
            let is_selected = idx == cursor;

            let prefix = if is_current {
                if is_playing { "▶ " } else { "■ " }
            } else if is_selected {
                "→ "
            } else {
                "  "
            };

            let num = format!("[{:02}] ", idx + 1);
            let title = &track.title;
            let artist = if track.artist.is_empty() || track.artist == "Unknown Artist" {
                String::new()
            } else {
                format!(" - {}", track.artist)
            };

            let (text_color, weight) = if is_current {
                (theme.highlight, Weight::Bold)
            } else if is_selected {
                (theme.lyric_singing, Weight::Bold)
            } else {
                (theme.lyric_future, Weight::Normal)
            };

            let s = session.clone();
            let on_sel = on_select.clone();
            element! {
                Button(handler: move |_| {
                    let _ = s.switch_track(idx);
                    if let Ok(mut f) = on_sel.lock() {
                        f(idx);
                    }
                }) {
                    View(
                        flex_direction: FlexDirection::Row,
                        padding_left: 1,
                        padding_right: 1,
                        width: 100pct,
                        justify_content: JustifyContent::SpaceBetween,
                    ) {
                        View(flex_direction: FlexDirection::Row) {
                            Text(color: text_color, weight: weight, content: prefix)
                            Text(color: theme.elapsed, weight: Weight::Normal, content: num)
                            Text(color: text_color, weight: weight, content: title.to_string())
                            Text(color: theme.remaining, content: artist)
                        }
                        #(is_current.then(|| element! {
                            Text(color: theme.live, weight: Weight::Bold, content: format!(" {} ", lang.live_badge()))
                        }))
                    }
                }
            }
            .into()
        })
        .collect();

    // ── Tab 2: Configured Folders in SQLite ──
    let folders = session.list_music_folders();
    let folder_items: Vec<AnyElement<'static>> = if folders.is_empty() {
        vec![element! {
            Text(color: theme.paused, content: format!(" {}", lang.folder_empty()))
        }
        .into()]
    } else {
        folders
            .iter()
            .enumerate()
            .map(|(idx, f_path)| {
                let is_selected = idx == folder_cursor;
                let prefix = if is_selected { "→ " } else { "  " };
                let num = format!("[{:02}] ", idx + 1);
                let (text_color, weight) = if is_selected {
                    (theme.highlight, Weight::Bold)
                } else {
                    (theme.lyric_future, Weight::Normal)
                };

                let song_count = crate::db::scan_folder_for_tracks(f_path)
                    .map(|t| t.len())
                    .unwrap_or(0);

                element! {
                    View(
                        flex_direction: FlexDirection::Row,
                        padding_left: 1,
                        padding_right: 1,
                        width: 100pct,
                        justify_content: JustifyContent::SpaceBetween,
                    ) {
                        View(flex_direction: FlexDirection::Row) {
                            Text(color: text_color, weight: weight, content: prefix)
                            Text(color: theme.elapsed, weight: Weight::Normal, content: num)
                            Text(color: text_color, weight: weight, content: f_path.display().to_string())
                        }
                        Text(color: theme.remaining, content: format!("({} {})", song_count, lang.folder_count()))
                    }
                }
                .into()
            })
            .collect()
    };

    let on_cls = on_close.clone();
    let on_tab_t = on_tab_change.clone();
    let on_tab_f = on_tab_change.clone();

    let tab_tracks_color = if tab == ModalTab::Tracks { theme.highlight } else { theme.remaining };
    let tab_folders_color = if tab == ModalTab::Folders { theme.highlight } else { theme.remaining };

    element! {
        View(
            flex_direction: FlexDirection::Column,
            border_style: BorderStyle::Double,
            border_color: theme.highlight,
            background_color: modal_bg,
            padding_top: 1,
            padding_bottom: 1,
            padding_left: 2,
            padding_right: 2,
            width: (width.min(88)) as u32,
            align_items: AlignItems::Center,
        ) {
            // Header Bar with Tabs and Close Button
            View(
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                width: 100pct,
                margin_bottom: 1,
            ) {
                View(flex_direction: FlexDirection::Row) {
                    Button(handler: move |_| {
                        if let Ok(mut f) = on_tab_t.lock() {
                            f(ModalTab::Tracks);
                        }
                    }) {
                        Text(
                            color: tab_tracks_color,
                            weight: if tab == ModalTab::Tracks { Weight::Bold } else { Weight::Normal },
                            content: format!(" [1] {} ({}) ", lang.tracks_tab(), playlist.len()),
                        )
                    }
                    Text(color: theme.remaining, content: "|")
                    Button(handler: move |_| {
                        if let Ok(mut f) = on_tab_f.lock() {
                            f(ModalTab::Folders);
                        }
                    }) {
                        Text(
                            color: tab_folders_color,
                            weight: if tab == ModalTab::Folders { Weight::Bold } else { Weight::Normal },
                            content: format!(" [2] {} (SQL: {}) ", lang.folders_tab(), folders.len()),
                        )
                    }
                }

                Button(handler: move |_| {
                    if let Ok(mut f) = on_cls.lock() {
                        f();
                    }
                }) {
                    Text(color: theme.remaining, weight: Weight::Bold, content: format!("{} ", lang.playlist_close_btn()))
                }
            }

            // Body
            View(flex_direction: FlexDirection::Column, width: 100pct) {
                #(match tab {
                    ModalTab::Tracks => track_items,
                    ModalTab::Folders => {
                        if is_adding_folder {
                            vec![
                                element! {
                                    View(
                                        flex_direction: FlexDirection::Column,
                                        border_style: BorderStyle::Single,
                                        border_color: theme.highlight,
                                        padding: 1,
                                        margin_bottom: 1,
                                        width: 100pct,
                                    ) {
                                        Text(
                                            color: theme.highlight,
                                            weight: Weight::Bold,
                                            content: format!("📂 {}", lang.add_folder_prompt()),
                                        )
                                        View(flex_direction: FlexDirection::Row, margin_top: 1) {
                                            Text(color: theme.lyric_singing, weight: Weight::Bold, content: "> ")
                                            Text(color: theme.highlight, weight: Weight::Bold, content: format!("{}_", folder_input))
                                        }
                                        Text(
                                            color: theme.keybinds_dim,
                                            content: format!("  {}", lang.add_folder_hints()),
                                        )
                                    }
                                }.into()
                            ]
                        } else {
                            folder_items
                        }
                    }
                })
            }

            // Footer hints
            View(
                margin_top: 1,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                width: 100pct,
            ) {
                Text(
                    color: theme.keybinds_dim,
                    content: match tab {
                        ModalTab::Tracks => lang.playlist_footer_hints().to_string(),
                        ModalTab::Folders => {
                            if is_adding_folder {
                                lang.add_folder_hints().to_string()
                            } else {
                                lang.folder_footer_hints().to_string()
                            }
                        }
                    },
                )
            }
        }
    }
    .into()
}
