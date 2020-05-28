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
- 2 independent filters with individual oscillator routing
- Wavetable scanning
- User wavetables
- Up to 16 modulation assignments
- 2 LFOs per voice plus 2 global LFOs
- 3 ADSR envelopes per voice, with adjustable slope
- Delay (mono or ping pong)
- 36 sets of MIDI controller assignments

For a detailed description, have a look at the [manual in the doc folder](doc/manual.md).

## Compiling, running and troubleshooting

Yazz should run on both MacOS and Linux. Assuming you have the Rust toolchain
installed, a simple `cargo run --release` should download all dependencies,
compile everything and run the synth.

Make sure to run the release version, otherwise the audio engine might have
performance problems (it's not optimized yet).

For Linux, the dev-package for ALSA needs to be installed (usually
libasound2-dev or alsa-lib-devel, see https://github.com/RustAudio/cpal for
more infos).

Yazz connects to MIDI port 0 per default. If you get a MIDI port error on
startup, or if Yazz doesn't react to MIDI messages, try connecting to a
different port with `cargo run --release -- -m 1`.

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
