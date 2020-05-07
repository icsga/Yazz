# Yazz - Yet Another Subtractive Synth

This is a subtractive synth written in Rust. It comes with a simple terminal
UI that allows all parameters to be edited by key sequences and/ or MIDI
controllers.

This is still very much work in progress. The basic sound engine works, but
some features are still missing.

## Features

- 3 wavetable oscillators per voice, 32 voice polyphony
- Up to 7 instances per oscillator with frequency spreading
- Wavetable scanning
- Oscillator sync
- User wavetables
- Up to 16 modulation assignments
- 2 LFOs per voice plus 2 global LFOs
- 3 ADSR envelopes per voice, with adjustable slope
- Delay

For a detailed description, have a look at the manual in the doc folder.

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
