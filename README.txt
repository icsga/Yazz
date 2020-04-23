Yazz - Yet Another Subtractive Synth

This is a subtractive synth written in Rust. It comes with a simple terminal
UI that allows all parameters to be edited by key sequences and/ or MIDI
controllers.

Features are:
- 3 wavetable oscillators per voice, 32 voice polyphony
- Up to 7 instances per oscillator with frequency spreading
- Wavetable scanning
- Oscillator sync
- Up to 20 modulation assignments from x sources to y targets
- 2 LFOs per voice plus 2 global LFOs
- 2 ADSR envelopes per voice, with adjustable slope
- Delay

For a detailed description, have a look at the manual in the doc folder.

This is still very much work in progress. The basic sound engine works, but
some features are still missing.

Near future enhancements:
- Resonant filter
- Chorus
- Multitap delay
- Additional key tuning tables for alternate tunings
- Additional oscillators (PM, FM)
- Editing via MIDI note commands

Far away future enhancements:
- GUI
- VST plugin
