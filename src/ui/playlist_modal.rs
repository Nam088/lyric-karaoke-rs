//! Interactive Playlist & Music Folders Manager Modal Dialog with dynamic Pagination, Table Layout, and Delete Confirmation.

use std::sync::{Arc, Mutex};

use iocraft::prelude::*;

use super::Session;
use crate::color::Theme;
use crate::i18n::Language;

pub const PAGE_SIZE: usize = 6;

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
pub fn render<
    FSelect,
    FClose,
    FTabChange,
    FAddFolder,
    FRescan,
    FRequestDeleteFolder,
    FConfirmDeleteFolder,
    FCancelDeleteFolder,
    FSelectFolder,
    FPrevPage,
    FNextPage,
>(
    session: Arc<Session>,
    tab: ModalTab,
    cursor: usize,
    folder_cursor: usize,
    is_adding_folder: bool,
    confirm_delete_index: Option<usize>,
    folder_input: &str,
    width: usize,
    lang: Language,
    theme: &Theme,
    on_select: FSelect,
    on_close: FClose,
    on_tab_change: FTabChange,
    on_add_folder: FAddFolder,
    on_rescan: FRescan,
    on_request_delete_folder: FRequestDeleteFolder,
    on_confirm_delete_folder: FConfirmDeleteFolder,
    on_cancel_delete_folder: FCancelDeleteFolder,
    on_select_folder: FSelectFolder,
    on_prev_page: FPrevPage,
    on_next_page: FNextPage,
) -> AnyElement<'static>
where
    FSelect: FnMut(usize) + Send + 'static,
    FClose: FnMut() + Send + 'static,
    FTabChange: FnMut(ModalTab) + Send + 'static,
    FAddFolder: FnMut() + Send + 'static,
    FRescan: FnMut() + Send + 'static,
    FRequestDeleteFolder: FnMut(usize) + Send + 'static,
    FConfirmDeleteFolder: FnMut(usize) + Send + 'static,
    FCancelDeleteFolder: FnMut() + Send + 'static,
    FSelectFolder: FnMut(usize) + Send + 'static,
    FPrevPage: FnMut() + Send + 'static,
    FNextPage: FnMut() + Send + 'static,
{
    let playlist = session.playlist.read().unwrap();
    let current_idx = *session.track_index.read().unwrap();
    let is_playing = session.audio.is_playing();

    let on_select = Arc::new(Mutex::new(on_select));
    let on_close = Arc::new(Mutex::new(on_close));
    let on_tab_change = Arc::new(Mutex::new(on_tab_change));
    let on_add_folder = Arc::new(Mutex::new(on_add_folder));
    let on_rescan = Arc::new(Mutex::new(on_rescan));
    let on_request_delete_folder = Arc::new(Mutex::new(on_request_delete_folder));
    let on_confirm_delete_folder = Arc::new(Mutex::new(on_confirm_delete_folder));
    let on_cancel_delete_folder = Arc::new(Mutex::new(on_cancel_delete_folder));
    let on_select_folder = Arc::new(Mutex::new(on_select_folder));
    let on_prev_page = Arc::new(Mutex::new(on_prev_page));
    let on_next_page = Arc::new(Mutex::new(on_next_page));

    let (bg_r, bg_g, bg_b) = theme.dark_base;
    let modal_bg = Color::Rgb {
        r: bg_r,
        g: bg_g,
        b: bg_b,
    };

    let col_no_w = 4;
    let col_title_w = 32;
    let col_artist_w = 22;

    // ── Pagination Calculation ──
    let total_tracks = playlist.tracks.len();
    let track_total_pages = total_tracks.max(1).div_ceil(PAGE_SIZE);
    let track_page = if total_tracks == 0 { 0 } else { (cursor / PAGE_SIZE).min(track_total_pages - 1) };
    let track_start = track_page * PAGE_SIZE;
    let track_end = (track_start + PAGE_SIZE).min(total_tracks);

    // ── Tab 1: Paginated Tracks Table Button Rows (Clean, no box clutter) ──
    let track_rows: Vec<AnyElement<'static>> = if total_tracks == 0 {
        vec![element! {
            Text(color: theme.paused, content: " (Danh sách bài hát trống)")
        }
        .into()]
    } else {
        playlist.tracks[track_start..track_end]
            .iter()
            .enumerate()
            .map(|(offset, track)| {
                let idx = track_start + offset;
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
            .collect()
    };

    // ── Tab 2: Configured Folders Table Button Rows ──
    let folders = session.list_music_folders();
    let col_f_no_w = 4;
    let col_f_path_w = 44;
    let col_f_count_w = 12;

    let total_folders = folders.len();
    let folder_total_pages = total_folders.max(1).div_ceil(PAGE_SIZE);
    let folder_page = if total_folders == 0 { 0 } else { (folder_cursor / PAGE_SIZE).min(folder_total_pages - 1) };
    let folder_start = folder_page * PAGE_SIZE;
    let folder_end = (folder_start + PAGE_SIZE).min(total_folders);

    let folder_rows: Vec<AnyElement<'static>> = if total_folders == 0 {
        vec![element! {
            Text(color: theme.paused, content: format!(" {}", lang.folder_empty()))
        }
        .into()]
    } else {
        folders[folder_start..folder_end]
            .iter()
            .enumerate()
            .map(|(offset, f_path)| {
                let idx = folder_start + offset;
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

                let on_req_del = on_request_delete_folder.clone();
                let on_sel_f = on_select_folder.clone();

                element! {
                    View(
                        flex_direction: FlexDirection::Row,
                        padding_left: 1,
                        padding_right: 1,
                        width: 100pct,
                        justify_content: JustifyContent::SpaceBetween,
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
                            if let Ok(mut f) = on_req_del.lock() {
                                f(idx);
                            }
                        }) {
                            Text(color: theme.paused, weight: Weight::Bold, content: format!(" {} ", lang.btn_delete()))
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
    let on_prev_btn = on_prev_page.clone();
    let on_next_btn = on_next_page.clone();

    let tab_tracks_color = if tab == ModalTab::Tracks { theme.highlight } else { theme.remaining };
    let tab_folders_color = if tab == ModalTab::Folders { theme.highlight } else { theme.remaining };

    let (active_page, total_p, item_total_str) = match tab {
        ModalTab::Tracks => (track_page + 1, track_total_pages, format!("{} {}", total_tracks, lang.folder_count())),
        ModalTab::Folders => (folder_page + 1, folder_total_pages, format!("{} {}", total_folders, lang.folders_tab())),
    };

    let is_confirming = confirm_delete_index.is_some();
    let is_busy = is_adding_folder || is_confirming;

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
            // Header Bar with Clean Flat Tab Buttons and Close Button
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

                    Text(color: theme.remaining, content: "│")

                    Button(handler: move |_| {
                        if let Ok(mut f) = on_tab_f.lock() {
                            f(ModalTab::Folders);
                        }
                    }) {
                        Text(
                            color: tab_folders_color,
                            weight: if tab == ModalTab::Folders { Weight::Bold } else { Weight::Normal },
                            content: format!(" [2] {} ({}) ", lang.folders_tab(), folders.len()),
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

            // Quick Toolbar for Folders Tab
            #(if tab == ModalTab::Folders && !is_busy {
                Some(element! {
                    View(
                        flex_direction: FlexDirection::Row,
                        width: 100pct,
                        margin_bottom: 1,
                        padding_left: 1,
                    ) {
                        Button(handler: move |_| {
                            if let Ok(mut f) = on_add_btn.lock() {
                                f();
                            }
                        }) {
                            Text(color: theme.highlight, weight: Weight::Bold, content: format!("{}   ", lang.btn_add_folder()))
                        }
                        Button(handler: move |_| {
                            if let Ok(mut f) = on_rescan_btn.lock() {
                                f();
                            }
                        }) {
                            Text(color: theme.lyric_singing, weight: Weight::Bold, content: lang.btn_rescan().to_string())
                        }
                    }
                })
            } else {
                None
            })

            // Table Column Headers
            #(if !is_busy {
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
            #(if !is_busy {
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
                        if let Some(del_idx) = confirm_delete_index {
                            let folder_path_str = folders.get(del_idx)
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| String::from("---"));
                            let on_conf = on_confirm_delete_folder.clone();
                            let on_canc = on_cancel_delete_folder.clone();
                            vec![
                                element! {
                                    View(
                                        flex_direction: FlexDirection::Column,
                                        border_style: BorderStyle::Single,
                                        border_color: theme.paused,
                                        padding: 1,
                                        margin_bottom: 1,
                                        width: 100pct,
                                        align_items: AlignItems::Center,
                                    ) {
                                        Text(
                                            color: theme.paused,
                                            weight: Weight::Bold,
                                            content: lang.confirm_delete_title().to_string(),
                                        )
                                        Text(
                                            color: theme.highlight,
                                            weight: Weight::Bold,
                                            content: format!("  {}", folder_path_str),
                                        )
                                        View(flex_direction: FlexDirection::Row, margin_top: 1, margin_bottom: 1) {
                                            Button(handler: move |_| {
                                                if let Ok(mut f) = on_conf.lock() {
                                                    f(del_idx);
                                                }
                                            }) {
                                                Text(color: theme.paused, weight: Weight::Bold, content: format!("{}   ", lang.btn_confirm_yes()))
                                            }
                                            Button(handler: move |_| {
                                                if let Ok(mut f) = on_canc.lock() {
                                                    f();
                                                }
                                            }) {
                                                Text(color: theme.highlight, weight: Weight::Bold, content: lang.btn_confirm_no().to_string())
                                            }
                                        }
                                        Text(
                                            color: theme.keybinds_dim,
                                            content: lang.confirm_delete_hints().to_string(),
                                        )
                                    }
                                }.into()
                            ]
                        } else if is_adding_folder {
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
                                            content: lang.add_folder_prompt().to_string(),
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

            // Pagination Controls Bar (Clean & Flat)
            #(if !is_busy && total_p > 1 {
                Some(element! {
                    View(
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        width: 100pct,
                        margin_top: 1,
                        padding_left: 1,
                        padding_right: 1,
                    ) {
                        Button(handler: move |_| {
                            if let Ok(mut f) = on_prev_btn.lock() {
                                f();
                            }
                        }) {
                            Text(color: theme.highlight, weight: Weight::Bold, content: format!("{} ", lang.btn_prev_page()))
                        }

                        Text(
                            color: theme.elapsed,
                            weight: Weight::Normal,
                            content: format!("{} {} / {} ({})", lang.page_info(), active_page, total_p, item_total_str),
                        )

                        Button(handler: move |_| {
                            if let Ok(mut f) = on_next_btn.lock() {
                                f();
                            }
                        }) {
                            Text(color: theme.highlight, weight: Weight::Bold, content: format!(" {}", lang.btn_next_page()))
                        }
                    }
                })
            } else {
                None
            })

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
                            if is_confirming {
                                lang.confirm_delete_hints().to_string()
                            } else if is_adding_folder {
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
