# Yazz - Yet Another Software Synth

Yazz is a subtractive synth written in Rust. It comes with a simple terminal
UI that allows all parameters to be edited by key sequences and/ or MIDI
controllers.

![rust-screenshot.png](doc/Screenshot1.png)

The main focus of this project is on mouse-free editing: Yazz is a synth for
terminal lovers.

This is still work in progress. The sound engine works, but some features are
missing, and the parameter ranges are not perfectly balanced yet.

## Features

- 3 wavetable oscillators per voice, 32 voice polyphony
- Up to 7 instances per oscillator with frequency spreading
- Oscillator sync
- Wavetable scanning
- User wavetables
- Up to 16 modulation assignments
- 2 LFOs per voice plus 2 global LFOs
- 3 ADSR envelopes per voice, with adjustable slope
- Delay
- 36 sets of MIDI controller assignments

For a detailed description, have a look at the [manual in the doc folder](doc/manual.md).

## Compiling, running and troubleshooting

Yazz should run on both MacOS and Linux. Assuming you have the Rust toolchain
installed, a simple "cargo build --release" should download all dependencies
and compile everything.

For Linux, the dev-package for ALSA needs to be installed (usually
libasound2-dev, see https://github.com/RustAudio/cpal for more infos).

Make sure to run the release version, otherwise the audio engine might have
performance problems (it's not optimized yet):

cargo run --release

Yazz connects to MIDI device 1 per default, which is probably incorrect for
most systems. If you get a MIDI port error on startup, try connectin to
device 0 instead:

cargo run --release -- -m 0

If you get a "file not found" error on startup, please create a "data"
folder in the directory you are starting the program from.

Check the documentation for additional command line parameters.

## Known issues

- The UI isn't drawn correctly on the MacOS terminal (as of 10.14.6). It works
  fine with Tmux or iTerm2, so please try one of these if you're on MacOS.
- The filter is a bit unstable and produces loud noise in some extreme
  settings.

## Near future enhancements

- Chorus
- Multitap delay
- Additional key tuning tables for alternate tunings
- Additional oscillators (PM, FM)
- Editing via MIDI note commands

## Far away future enhancements

- Optional GUI
- Implement VST plugin interface
