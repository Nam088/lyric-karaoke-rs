# Port of lyric-react-ink to Rust

Written 2026-08-16. Records why the port was done this way and what changed
along the way.

## Why port at all

The React + Ink build only ran on macOS, and nothing in it said so. Its audio
layer shelled out to a bundled ffmpeg binary:

- playback was `ffmpeg -ss <offset> -f audiotoolbox`, and `audiotoolbox` is a
  macOS only output
- pause was `SIGSTOP` and resume was `SIGCONT`, neither of which Windows has
- seeking killed the process and spawned a new one, which cost enough that it
  needed a 400ms debounce to stay usable
- duration came from a regular expression over ffmpeg's stderr

So "make it run on Windows" and "stop shipping an 80MB binary to play an mp3"
turned out to be the same job.

## Stack

| Concern | TypeScript | Rust |
| --- | --- | --- |
| Interface | ink 6, React 19 | iocraft 0.8 |
| Animation | ink-motion | colour interpolation over the playback clock |
| Playback | ffmpeg-static | rodio 0.22 (cpal + Symphonia) |
| Analysis | ffmpeg `filter_complex` | rustfft 6.4 |
| Async | Node event loop | smol 2.0 |

iocraft was chosen because it is the closest thing Rust has to Ink: the same
component model, the same hooks, flexbox through taffy rather than Yoga. The
port is close to line for line in the UI layer.

`View` maps to `Box`, `Text` to `Text`, `use_state` / `use_effect` / `use_memo`
to their React namesakes, `use_terminal_events` to `useInput`,
`use_terminal_size` to `process.stdout.columns`, and
`element!(App).fullscreen()` to `render(<App/>)`.

## Decisions

### One clock

The screen is a pure function of `audio.position_ms()`. Nothing else keeps
time. Pause, seek and resize all fall out of that for free, and the drift
between the words and the music is gone because there is nothing left to drift
against.

This removed `useKaraokePlayer` entirely, along with `IndependentHeaderClock`
and `IndependentFooterTimeline` and their reaching into private fields
(`audioRef.current['_isPlaying']`).

### Seeking is immediate

`Player::try_seek` returns straight away, so `SEEK_DEBOUNCE_MS` and the state
machine around it (`targetSeekTimeRef`, `seekTimeoutRef`, `wasPlayingRef`,
roughly forty lines of `app.tsx`) are gone.

### The spectrum divides the way an analyser does

Bands follow IEC 61260: fractional octave, on standard centre frequencies
derived from 1kHz. A 1/N octave band is a fixed fraction of an octave wide, so
its bandwidth grows in proportion to its centre frequency.

That is not decoration. Pink noise carries equal energy per octave, so it reads
flat across bands like these, and music, which is roughly pink, reads roughly
flat too. Buckets of equal width in hertz need a tilt bolted on to look right;
these do not.

A band level is the **sum of the power** in it, not its loudest bin. The
distinction is the whole point. A band an octave up is twice as wide, so it
collects twice the power from a signal of equal density. Reading the peak bin
reports the same number for both, and an analyser built that way has to invent
a tilt to make music look level. The first port did exactly that, with an
arbitrary `TILT_DB = 11.0`; both the peak reading and the tilt are gone.

Two details the measurement depends on:

- **Fractional bin overlap at the band edges.** Rounding edges to whole bins
  quantises the narrow low frequency bands badly enough to flatten the three
  decibel per octave slope white noise is supposed to show. Measured 1.76dB
  before, 3.01dB after.
- **The window's noise power bandwidth.** A Hann window spreads one tone over
  1.5 bins, so a plain sum counts it one and a half times. Divided back out, a
  full scale sine reads 0 dBFS in its band.

The transform runs at 4096 points, a 93ms window and 10.8Hz per bin. At 2048
the bottom two octaves fell inside a single bin.

Which fraction is used adapts to the terminal: the standard fraction whose band
count comes closest to the column count, from {1, 3, 6, 12, 24}. The analysis
stays on standard centres whatever the width is; only the number of them
changes, and the display interpolates between bands rather than the analysis
bending to fit the screen.

Four tests pin this down: pink noise reads flat, white noise rises 3.01dB per
octave, a full scale tone reads 0 dBFS, and a tone on a band boundary is shared
between the neighbours rather than lost.

### The rest of the spectrum is measured, not invented

`Spectrum.tsx` had only three filtered bands, interpolated across the width, so
every column moved together. It compensated with two invented terms, and said
so in a comment: *"The user noted all columns looked the same."*

```js
const staticJaggedness = 0.65 + 0.35 * Math.abs(Math.sin(i * 13.73 + 4.1));
const turbulence = 0.2 * Math.sin((currentTime / 60) + i * 0.85);
```

A real transform on the audio that is playing produces its own detail, so both
are gone. What replaced them:

- **Braille canvas.** 2x4 dots per cell, so four rows is a 16 pixel surface at
  double horizontal resolution.
- **IEC 61260 fractional octave bands**, 40Hz to 16kHz, summing power.
- **Automatic gain**, fast attack and slow release on the recent loudest
  column.
- **Harmonic product spectrum** for the note readout, hidden by default and
  brought up with `N`.
- **Peak markers** that hold and fall.

### Nothing pulses

An earlier version brightened the border on every detected beat. Measured over
fourteen seconds of real playback it drew the border in **thirty different
shades**, and the shade it spent the most time in was the brightest one, not
the resting one. That is not beat detection, that is a detector saturated by
the presence of music.

It also cost three quarters of the output: removing it took the frame from
4,790 bytes to 1,568. The border is one colour now, and the onset detector
went with it rather than sit unused.

### The tap

`SpectrumTap` wraps the rodio `Source`, forwards every sample untouched, and
writes a mono copy to a ring buffer. That is what makes the spectrum live
rather than a scan of the file, and it removes the "Analyzing Audio Waves..."
screen the old build showed at startup.

### The timeline is a plain bar

Elapsed run, marker at the playhead, remaining run. A progress bar exists to be
read at a glance, and at two rows of braille the loudness envelope reads as
texture rather than as a position.

The envelope is still there behind `config::WAVEFORM_TIMELINE`, off by default.
It is scanned on a background thread and drawn as a flat line until it lands,
so the layout never reflows either way.

Three things had to be right before it showed any shape:

1. **RMS, not peak.** A quarter second of a modern master hits full scale
   almost everywhere.
2. **Percentile stretch, not a fixed decibel window.** A compressed track lives
   in a handful of decibels; any window wide enough for a live recording
   flattens it. Stretching between the track's own 5th and 95th percentile
   adapts to whatever it was mastered like.
3. **Average when resampling, not maximum.** Taking the peak of a dozen
   already normalised slots puts every column back at the ceiling.

## Bugs found in the original

- `Spectrum.tsx:26` called `React.useRef` after a conditional `return null` on
  line 25. On the first render `spectrumData.mid.length` is 0, so the hook was
  skipped and the hook order changed once the data arrived.
- `LyricLine.tsx:175` advanced the karaoke fill by `word.data.length`, which
  counts UTF-16 code units. macOS stores text decomposed, so "ế" is three
  units for one visible character and the highlight ran ahead of the voice.
  The port counts grapheme clusters.
- `useKaraokePlayer.ts` computed `isInsideWord`, `getNextBoundary` and
  `timelineRef` and listed them in dependency arrays, but nothing read them.
  The scheduling scheme its doc comment described had been abandoned.
- `constants.ts:73` `PATHS.AUDIO_FILE` was never used; `app.tsx` resolved the
  path itself.
- `app.tsx` read the lyrics file and probed ffmpeg during module import, and on
  failure logged to the console and continued with an empty array, so a wrong
  path produced a blank screen rather than an error.

## Bugs found while porting

- **The panel overflowed and rendered blank.** At full size it wants 23 rows,
  more than a standard 80x24 terminal. `ui/layout.rs` now measures and drops
  chrome in order of importance, and there is a test asserting the measurement
  matches what actually gets drawn.
- **Children competing for the last column.** This one bit three times, and it
  is worth stating as a rule rather than a bug: when two full width children
  both want the whole content area, one of them loses, gets squeezed to a
  single column, and wraps once per character. A two row footer became seventy
  rows and the panel became 213; later the same thing pushed four rows of
  spectrum into sixty odd, and because the panel is centred vertically the
  extra height rode up over the title.

  The first two times were patched locally, with percentage widths and then by
  reverting a fixed pixel width. Neither addressed the cause. The panel now
  keeps **one spare column** inside its content area so nothing has to compete,
  and there are tests that render the real spectrum, in every style, inside a
  panel and assert the height.

  `TextWrap::NoWrap` looked like the obvious answer and was tried. It makes
  things worse: a graphic that refuses to shrink simply pushes the loss onto
  its sibling, and the horizontal rule started wrapping instead.
- **`Decoder::try_from(BufReader<File>)` returns no duration.** Symphonia
  buffers internally and the wrapper hid the file length, so the envelope scan
  gave up after a millisecond. Bare `File` everywhere now.
- **Interleaved stereo fed straight to the FFT** reads as a signal at twice the
  sample rate and smears every frequency. The tap downmixes first.
- **Unnormalised FFT magnitudes** saturated every column. A full scale sine
  through a Hann window peaks at N/4.
- **Naive harmonic product spectrum missed harmonics.** If the fundamental is
  not bin centred, its `h`th harmonic sits up to `h/2` bins from `b * h`; since
  the terms are multiplied, one miss zeroed the candidate. It now takes the
  maximum over a widening neighbourhood.

### The active line marker

Three things were wrong with it, two inherited and one of mine.

The TypeScript build showed the marker on every line within one step of the
centre and dimmed the neighbours toward `#010201` to hide them. That only
disappears on a terminal whose background really is black; on any other theme
three lines wore the marker at once. It now appears on the line being sung and
nowhere else.

An instrumental break no longer gets one either. That line is already a row of
musical notes, and `♯  ♫ ♪ ♫ ♪ ♫  ♯` reads as noise.

The line also sat off centre, because every word was emitted as
`format!("{} ", w.data)` including the last one. The active line was therefore
one column wider than the same words on any neighbouring line, so the text
drifted and the right hand marker sat further out than the left. Separators now
go between words rather than after them, and a test asserts the rendered spans
join back to exactly `sentence.text()`.

What remains is a single column: centring content of odd width inside an even
width panel always leaves an odd number of spaces to split. The extra one
consistently goes on the left.

## Kept

`resolveAudioPath`'s fuzzy matching. macOS stores filenames decomposed, so a
Vietnamese title typed composed will not match on disk. It is the one piece of
cleverness in the original that earns its place.
