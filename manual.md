# Yazz - Yet Another Subtractive Synth

## Architecture

Yazz has a fixed signal flow with the following components:

* 32 voices
* 3 wavetable-based oscillators per voice
* 2 (well, currently only one) filters with LP-/ HP- and BP-Modes (well, currently
  only LP) per voice
* 2 ADSR envelopes per voice
* 2 LFOs per voice
* 2 global LFOs
* Delay

## Loading and saving sounds

**This functionality is still under construction.**

Currently the sound filename is hardwired to "Yazz_FactoryBank.ysn". To load
the file, press F1.

To change the current program, send MIDI program change commands. **WARNING: All
changes made to a sound will be lost when changing the current program.**

To save a sound, press F2. Currently only saving the complete sound bank is
supported. Copying a sound, saving single sounds etc. are still on the TODO
list.

## Editing parameters

Yazz is controlled entirely with the (computer-)keyboard and/ or MIDI
controllers. The top line of the UI is the **command line** that is used
for changing parameters.

Every parameter consists of four parts:

* A **function**: The general function group, e.g. Oscillator, Envelope etc.
* A **function ID**, in case there are more than one instances of the current
  function, e.g. Oscillator 3, Envelope 1
* A **parameter**: A parameter of the selected function (e.g. Oscillator Level,
  Envelope Attack)
* A **value**: The actual value of the selected parameter. This can itself be
  another parameter, e.g. for modulation source and target.

### Function selection

When starting a parameter editing, the function must be selected. The UI shows
a drop-down list of available functions. Every function has a shortcut
associated with it, e.g. 'o' for oscillator, 'l' for LFO. The function can
be selected by:

* Entering the shortcut key
* Selecting a function with the CursorUp/ CursorDown keys and pressing enter
  or CursorRight

### Function ID selection

If only a single instance of the function is available (e.g. Delay), the input
will immediately progress to the parameter selection. Otherwise the ID can
be selected by entering the number, using the cursor keys or using +/ -.

### Parameter selection

The parameter selection uses the same mechanism as the function selection

### Value selection

The value can be changed by:

* Typing in the target value and pressing enter, e.g. "3.14".
* Using +/ - to increment/ decrement the value
* Using the assigned input controller (currently the modulation wheel) to set
  the value

After having completed the value, pressing Enter will return to the Parameter
selection. Pressing Escape will return to the function selection.

## Assigning MIDI controllers

Yazz supports assigning MIDI controllers to all values (except modulation
assignments). To assign a controller, enter MIDI learn mode by:

* Select the parameter to map to in the command line (e.g. "o1l" to select
  oscillator 1 level.
* Enter MIDI learn mode by pressing "l". The command line will show the text
  **MIDI learn: Send controller values**. TODO: Verify text
* Send values with the MIDI controller. Yazz needs at least two values to be
  able to distinguish absolute and relative controller types.

After having detected the controller, the command line switches back to value
input mode. To cancel MIDI learn mode, press Escape.
