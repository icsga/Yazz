Yazz - Yet Another Subtractive Synth

This is a subtractive synth written in Rust. It comes with a simple terminal
UI that allows all parameters to be edited by key sequences and/ or MIDI
controllers.

Features are:
- 3 oscillators per voice, 32 voice polyphony
- Up to 7 instances per oscillator with frequency spreading
- Waveform morphing
- Oscillator sync
- PWM
- Up to 20 modulation assignments from x sources to y targets
- 2 LFOs per voice plus 2 global LFOs
- 2 ADSR envelopes per voice, with adjustable slope
- Delay

This is still very much work in progress. The basic sound engine works, but
some features are still missing.

Near future enhancements:
- 2 resonant filters
- Chorus
- Multitap delay
- Additional key tuning tables for alternate tunings
- Additional oscillators (Wavetable, PM, FM)
- MIDI controller assignment
- Editing via MIDI note commands

Far away future enhancements:
- GUI
- VST plugin
