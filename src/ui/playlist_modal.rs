//! Interactive Playlist Modal Dialog without emojis.

use std::sync::{Arc, Mutex};

use iocraft::prelude::*;

use super::Session;
use crate::color::Theme;

pub fn render<FSelect, FClose>(
    session: Arc<Session>,
    cursor: usize,
    width: usize,
    theme: &Theme,
    on_select: FSelect,
    on_close: FClose,
) -> AnyElement<'static>
where
    FSelect: FnMut(usize) + Send + 'static,
    FClose: FnMut() + Send + 'static,
{
    let playlist = session.playlist.read().unwrap();
    let current_idx = *session.track_index.read().unwrap();
    let is_playing = session.audio.is_playing();

    let on_select = Arc::new(Mutex::new(on_select));
    let on_close = Arc::new(Mutex::new(on_close));

    let items: Vec<AnyElement<'static>> = playlist
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
                            Text(color: theme.live, weight: Weight::Bold, content: " [LIVE] ")
                        }))
                    }
                }
            }
            .into()
        })
        .collect();

    let on_cls = on_close.clone();
    element! {
        View(
            flex_direction: FlexDirection::Column,
            border_style: BorderStyle::Double,
            border_color: theme.highlight,
            padding_top: 1,
            padding_bottom: 1,
            padding_left: 2,
            padding_right: 2,
            width: (width.min(84)) as u32,
            align_items: AlignItems::Center,
        ) {
            View(
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                width: 100pct,
                margin_bottom: 1,
            ) {
                Text(
                    color: theme.highlight,
                    weight: Weight::Bold,
                    content: format!(" DANH SÁCH BÀI HÁT ({})", playlist.len()),
                )
                Button(handler: move |_| {
                    if let Ok(mut f) = on_cls.lock() {
                        f();
                    }
                }) {
                    Text(color: theme.remaining, weight: Weight::Bold, content: "[x] Đóng ")
                }
            }

            View(flex_direction: FlexDirection::Column, width: 100pct) {
                #(items)
            }

            View(
                margin_top: 1,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                width: 100pct,
            ) {
                Text(
                    color: theme.keybinds_dim,
                    content: "[↑/↓] Chọn  •  [Enter] Phát  •  [Esc/L] Đóng",
                )
            }
        }
    }
    .into()
}
