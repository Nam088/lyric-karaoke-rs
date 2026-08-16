# Lyric Karaoke (Rust)

Terminal karaoke player. A port of the React + Ink build, rewritten so it runs
on Windows as well as macOS and needs no external tools to build or run.

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

## Running it

```bash
# Put an audio file and its lyrics in data/
#   data/lr.json
#   data/<song>.mp3
# Then set SONG_NAME and SONG_FILE in src/config.rs and:
cargo run --release
```

`--frame` renders a single pass to stdout instead of taking over the terminal,
which is handy for checking the layout.

### Keys

| Key | Action |
| --- | --- |
| `Space` | play or pause |
| `←` `→` | seek five seconds |
| `S` | cycle the spectrum: curve, mirror, line, bars, off |
| `N` | show or hide the detected note |
| `H` | show the key list in the header |
| `Q` / `Esc` | quit |

## What it needs

Nothing beyond a Rust toolchain. No ffmpeg, no Node, no `node_modules`, no
system audio libraries on macOS or Windows. `cargo build` and you have a single
binary you can copy to another machine.

Linux additionally wants the ALSA development headers (`libasound2-dev` on
Debian and Ubuntu) because that is what cpal links against there.

## How it is put together

```
src/
├── main.rs          load everything, then hand off to the UI
├── config.rs        colours, timings, symbols, layout
├── lyrics.rs        lr.json, gap injection, which line is active
├── color.rs         blending
├── braille.rs       the 2x4 dot drawing surface
├── audio/
│   ├── mod.rs       play, pause, seek, position, duration
│   └── tap.rs       pass through node that copies audio for analysis
├── analysis/
│   ├── mod.rs       FFT, band levels, automatic gain, beat detection
│   ├── bands.rs     IEC 61260 fractional octave bands
│   ├── pitch.rs     which note is sounding
│   └── envelope.rs  background loudness scan, for the optional waveform
└── ui/
    ├── mod.rs       the root component and the render loop
    ├── layout.rs    fitting the panel into the terminal
    ├── header.rs    title, state, clock, note
    ├── lyric_line.rs one line, with the karaoke fill
    ├── spectrum.rs  the spectrum curve
    └── footer.rs    ticker, progress bar, transport
```

Built on [iocraft](https://github.com/ccbrown/iocraft) for the interface,
[rodio](https://github.com/RustAudio/rodio) for playback and
[rustfft](https://crates.io/crates/rustfft) for the analysis.

## One clock

Every frame reads `audio.position_ms()` and derives the whole screen from it.
Nothing keeps its own timer.

The TypeScript build ran three, none of which was the audio: `useKaraokePlayer`
counted with `performance.now()`, `AudioPlayer.currentPositionMs` counted with
`Date.now()` and described itself as approximate, and the header clock and the
timeline each polled on their own interval. They drifted from each other and
from the song.

## The visualiser

Four things, all measured from the signal that is playing:

- **A braille canvas.** Each cell is a 2x4 dot matrix, so a four row strip is a
  16 pixel tall surface at double horizontal resolution. Enough to draw a curve
  rather than a staircase.
- **IEC 61260 fractional octave bands**, 40Hz to 16kHz, on standard centre
  frequencies, summing the power in each. Bands like these are a fixed fraction
  of an octave wide, so pink noise reads flat and music, being roughly pink,
  reads roughly flat too. No tilt has to be invented to make it look right.
- **Automatic gain**, tracking the recent loudest column. A quiet verse and a
  loud chorus both fill the frame.

Four ways to draw it, cycled with `S`. The measurement is identical for all of
them; only the picture changes.

```
curve                              mirror
⢝⢝⢝⢝⢝⢝⢕⢄⠀⠀⠀⣀⡠⠤⣀⠀⠀⠀⠀⠀⠀⠀    ⢝⢝⢝⢝⢝⢝⣢⢄⣀⣀⣀⣀⣠⢤⣀⡀⠀⠀⠀⠀⠀⠀
⢕⢕⢕⢕⢕⢕⠭⠵⢍⣉⣩⠴⡲⡲⣢⢕⢄⠀⠀⠀⠀⠀    ⢕⢕⢕⢕⢕⢕⣒⠭⡪⡪⡪⡪⣒⢕⡪⣚⠵⣢⢤⢤⢤⢤
⢕⢕⢕⢕⢕⢕⠭⠭⠭⡪⣒⠭⡪⡪⣒⠭⠵⢍⠢⠤⠤⢒    ⡪⡪⡪⡪⡪⡪⠭⣒⢕⢕⢕⢕⠭⡪⢕⢭⡲⠝⠚⠚⠚⠚
⢕⢕⢕⢕⢕⢕⠭⠭⠭⡪⣒⠭⡪⡪⣒⠭⠭⠭⢝⡲⣚⠭    ⣪⣪⣪⣪⣪⣪⠝⠊⠉⠉⠉⠉⠙⠚⠉⠁⠀⠀⠀⠀⠀⠀

line                               bars
⠉⠉⠉⠉⠉⠉⢕⢄⠀⠀⠀⣀⡠⠤⣀⠀⠀⠀⠀⠀⠀⠀    ██████▃
⠀⠀⠀⠀⠀⠀⠀⠑⢍⣉⡩⠔⠒⠒⠢⢕⢄⠀⠀⠀⠀⠀    ███████▆▃▂▃▅▆▇▅▃
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠑⢍⠢⠤⠤⢒    ████████████████▇▃   ▁▄▇
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠒⠊⠁    ███████████████████▆▇███
```

`bars` is the only one that does not use braille, in case a terminal renders it
poorly. One more press of `S` switches the spectrum off, and the rows it was
using go back to the lyrics.

It starts off. The lyrics are the point of the app and the spectrum is
decoration, so the rows go to the words unless you ask for them back.
`config::SHOW_SPECTRUM` changes which end of the cycle the app starts on.

The timeline underneath is a plain progress bar. The song's loudness envelope
is available instead via `config::WAVEFORM_TIMELINE`, though at two rows of
braille it reads as texture rather than as a position you can judge at a
glance.

## Testing

```bash
cargo test
cargo clippy --all-targets
```

91 tests. The band analysis is checked against the standard: pink noise reads
flat, white noise rises 3.01dB per octave, a full scale tone reads 0 dBFS, and
a tone on a band boundary is shared between neighbours rather than lost. The
rest cover the braille encoding, grapheme aware text fill, the progress bar
arithmetic, the layout, and the lyric line markers.
