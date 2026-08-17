//! Interactive Playlist & Music Folders Manager Modal Dialog formatted as an interactive Table with button cells.

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

fn pad_truncate(s: &str, target_width: usize) -> String {
    let count = unicode_width::UnicodeWidthStr::width(s);
    if count > target_width {
        let mut res = String::new();
        let mut cur_w = 0;
        for c in s.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
            if cur_w + cw + 2 > target_width {
                break;
            }
            res.push(c);
            cur_w += cw;
        }
        res.push_str("..");
        cur_w += 2;
        if cur_w < target_width {
            res.push_str(&" ".repeat(target_width - cur_w));
        }
        res
    } else {
        let mut res = s.to_string();
        res.push_str(&" ".repeat(target_width - count));
        res
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render<FSelect, FClose, FTabChange, FAddFolder, FRescan, FDeleteFolder, FSelectFolder>(
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
    on_add_folder: FAddFolder,
    on_rescan: FRescan,
    on_delete_folder: FDeleteFolder,
    on_select_folder: FSelectFolder,
) -> AnyElement<'static>
where
    FSelect: FnMut(usize) + Send + 'static,
    FClose: FnMut() + Send + 'static,
    FTabChange: FnMut(ModalTab) + Send + 'static,
    FAddFolder: FnMut() + Send + 'static,
    FRescan: FnMut() + Send + 'static,
    FDeleteFolder: FnMut(usize) + Send + 'static,
    FSelectFolder: FnMut(usize) + Send + 'static,
{
    let playlist = session.playlist.read().unwrap();
    let current_idx = *session.track_index.read().unwrap();
    let is_playing = session.audio.is_playing();

    let on_select = Arc::new(Mutex::new(on_select));
    let on_close = Arc::new(Mutex::new(on_close));
    let on_tab_change = Arc::new(Mutex::new(on_tab_change));
    let on_add_folder = Arc::new(Mutex::new(on_add_folder));
    let on_rescan = Arc::new(Mutex::new(on_rescan));
    let on_delete_folder = Arc::new(Mutex::new(on_delete_folder));
    let on_select_folder = Arc::new(Mutex::new(on_select_folder));

    let (bg_r, bg_g, bg_b) = theme.dark_base;
    let modal_bg = Color::Rgb {
        r: bg_r,
        g: bg_g,
        b: bg_b,
    };

    let col_no_w = 4;
    let col_title_w = 32;
    let col_artist_w = 22;

    // ── Tab 1: Tracks Table Button Rows ──
    let track_rows: Vec<AnyElement<'static>> = playlist
        .tracks
        .iter()
        .enumerate()
        .map(|(idx, track)| {
            let is_current = idx == current_idx;
            let is_selected = idx == cursor;

            let prefix = if is_current {
                if is_playing { "▶" } else { "■" }
            } else if is_selected {
                "→"
            } else {
                " "
            };

            let num_str = format!("{}{:02}", prefix, idx + 1);
            let no_col = pad_truncate(&num_str, col_no_w);
            let title_col = pad_truncate(&track.title, col_title_w);
            let artist_str = if track.artist.is_empty() { "Unknown Artist" } else { &track.artist };
            let artist_col = pad_truncate(artist_str, col_artist_w);

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
                        border_style: if is_selected { BorderStyle::Single } else { BorderStyle::None },
                        border_color: if is_selected { theme.highlight } else { Color::Reset },
                    ) {
                        View(flex_direction: FlexDirection::Row) {
                            Text(color: if is_current { theme.live } else { theme.elapsed }, weight: weight, content: no_col)
                            Text(color: text_color, weight: weight, content: format!(" {}", title_col))
                            Text(color: theme.remaining, content: format!(" {}", artist_col))
                        }
                        View(flex_direction: FlexDirection::Row) {
                            #(if is_current {
                                Some(element! {
                                    Text(color: theme.live, weight: Weight::Bold, content: format!(" {} ", lang.live_badge()))
                                })
                            } else if !track.lyrics.is_empty() {
                                Some(element! {
                                    Text(color: theme.remaining, content: " [♫] ".to_string())
                                })
                            } else {
                                Some(element! {
                                    Text(color: theme.keybinds_dim, content: " [-] ".to_string())
                                })
                            })
                        }
                    }
                }
            }
            .into()
        })
        .collect();

    // ── Tab 2: Configured Folders Table Button Rows ──
    let folders = session.list_music_folders();
    let col_f_no_w = 4;
    let col_f_path_w = 44;
    let col_f_count_w = 12;

    let folder_rows: Vec<AnyElement<'static>> = if folders.is_empty() {
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
                let prefix = if is_selected { "→" } else { " " };
                let num_str = format!("{}{:02}", prefix, idx + 1);
                let no_col = pad_truncate(&num_str, col_f_no_w);
                let path_str = f_path.display().to_string();
                let path_col = pad_truncate(&path_str, col_f_path_w);

                let song_count = crate::db::scan_folder_for_tracks(f_path)
                    .map(|t| t.len())
                    .unwrap_or(0);
                let count_str = format!("{} {}", song_count, lang.folder_count());
                let count_col = pad_truncate(&count_str, col_f_count_w);

                let (text_color, weight) = if is_selected {
                    (theme.highlight, Weight::Bold)
                } else {
                    (theme.lyric_future, Weight::Normal)
                };

                let on_del = on_delete_folder.clone();
                let on_sel_f = on_select_folder.clone();

                element! {
                    View(
                        flex_direction: FlexDirection::Row,
                        padding_left: 1,
                        padding_right: 1,
                        width: 100pct,
                        justify_content: JustifyContent::SpaceBetween,
                        border_style: if is_selected { BorderStyle::Single } else { BorderStyle::None },
                        border_color: if is_selected { theme.highlight } else { Color::Reset },
                    ) {
                        Button(handler: move |_| {
                            if let Ok(mut f) = on_sel_f.lock() {
                                f(idx);
                            }
                        }) {
                            View(flex_direction: FlexDirection::Row) {
                                Text(color: theme.elapsed, weight: weight, content: no_col)
                                Text(color: text_color, weight: weight, content: format!(" {}", path_col))
                                Text(color: theme.remaining, content: format!(" {}", count_col))
                            }
                        }
                        Button(handler: move |_| {
                            if let Ok(mut f) = on_del.lock() {
                                f(idx);
                            }
                        }) {
                            View(
                                border_style: BorderStyle::Single,
                                border_color: theme.paused,
                                padding_left: 1,
                                padding_right: 1,
                            ) {
                                Text(color: theme.paused, weight: Weight::Bold, content: lang.btn_delete().to_string())
                            }
                        }
                    }
                }
                .into()
            })
            .collect()
    };

    let on_cls = on_close.clone();
    let on_tab_t = on_tab_change.clone();
    let on_tab_f = on_tab_change.clone();
    let on_add_btn = on_add_folder.clone();
    let on_rescan_btn = on_rescan.clone();

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
            width: (width.min(94)) as u32,
            align_items: AlignItems::Center,
        ) {
            // Header Bar with Tab Buttons and Close Button
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
                        View(
                            border_style: BorderStyle::Single,
                            border_color: tab_tracks_color,
                            padding_left: 1,
                            padding_right: 1,
                            margin_right: 1,
                        ) {
                            Text(
                                color: tab_tracks_color,
                                weight: if tab == ModalTab::Tracks { Weight::Bold } else { Weight::Normal },
                                content: format!("𝄢 [1] {} ({})", lang.tracks_tab(), playlist.len()),
                            )
                        }
                    }

                    Button(handler: move |_| {
                        if let Ok(mut f) = on_tab_f.lock() {
                            f(ModalTab::Folders);
                        }
                    }) {
                        View(
                            border_style: BorderStyle::Single,
                            border_color: tab_folders_color,
                            padding_left: 1,
                            padding_right: 1,
                        ) {
                            Text(
                                color: tab_folders_color,
                                weight: if tab == ModalTab::Folders { Weight::Bold } else { Weight::Normal },
                                content: format!("📂 [2] {} (SQL: {})", lang.folders_tab(), folders.len()),
                            )
                        }
                    }
                }

                Button(handler: move |_| {
                    if let Ok(mut f) = on_cls.lock() {
                        f();
                    }
                }) {
                    View(
                        border_style: BorderStyle::Single,
                        border_color: theme.remaining,
                        padding_left: 1,
                        padding_right: 1,
                    ) {
                        Text(color: theme.remaining, weight: Weight::Bold, content: lang.playlist_close_btn().to_string())
                    }
                }
            }

            // Quick Toolbar for Folders Tab
            #(if tab == ModalTab::Folders && !is_adding_folder {
                Some(element! {
                    View(
                        flex_direction: FlexDirection::Row,
                        width: 100pct,
                        margin_bottom: 1,
                    ) {
                        Button(handler: move |_| {
                            if let Ok(mut f) = on_add_btn.lock() {
                                f();
                            }
                        }) {
                            View(
                                border_style: BorderStyle::Single,
                                border_color: theme.highlight,
                                padding_left: 1,
                                padding_right: 1,
                                margin_right: 1,
                            ) {
                                Text(color: theme.highlight, weight: Weight::Bold, content: lang.btn_add_folder().to_string())
                            }
                        }
                        Button(handler: move |_| {
                            if let Ok(mut f) = on_rescan_btn.lock() {
                                f();
                            }
                        }) {
                            View(
                                border_style: BorderStyle::Single,
                                border_color: theme.lyric_singing,
                                padding_left: 1,
                                padding_right: 1,
                            ) {
                                Text(color: theme.lyric_singing, weight: Weight::Bold, content: lang.btn_rescan().to_string())
                            }
                        }
                    }
                })
            } else {
                None
            })

            // Table Column Headers
            #(if !is_adding_folder {
                Some(element! {
                    View(
                        flex_direction: FlexDirection::Row,
                        padding_left: 1,
                        padding_right: 1,
                        width: 100pct,
                        justify_content: JustifyContent::SpaceBetween,
                    ) {
                        View(flex_direction: FlexDirection::Row) {
                            Text(color: theme.elapsed, weight: Weight::Bold, content: pad_truncate(lang.th_no(), if tab == ModalTab::Tracks { col_no_w } else { col_f_no_w }))
                            Text(
                                color: theme.elapsed,
                                weight: Weight::Bold,
                                content: format!(" {}", pad_truncate(if tab == ModalTab::Tracks { lang.th_title() } else { lang.th_folder() }, if tab == ModalTab::Tracks { col_title_w } else { col_f_path_w }))
                            )
                            Text(
                                color: theme.elapsed,
                                weight: Weight::Bold,
                                content: format!(" {}", pad_truncate(if tab == ModalTab::Tracks { lang.th_artist() } else { lang.th_count() }, if tab == ModalTab::Tracks { col_artist_w } else { col_f_count_w }))
                            )
                        }
                        Text(color: theme.elapsed, weight: Weight::Bold, content: if tab == ModalTab::Tracks { lang.th_status() } else { lang.th_action() })
                    }
                })
            } else {
                None
            })

            // Table Divider Rule
            #(if !is_adding_folder {
                Some(element! {
                    View(width: 100pct, margin_bottom: 1) {
                        Text(color: theme.remaining, content: "─".repeat(86))
                    }
                })
            } else {
                None
            })

            // Table Body
            View(flex_direction: FlexDirection::Column, width: 100pct) {
                #(match tab {
                    ModalTab::Tracks => track_rows,
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
                            folder_rows
                        }
                    }
                })
            }

            // Footer keyboard hints
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
