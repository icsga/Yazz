# Yazz - Yet Another Subtractive Synth

## Introduction

Thanks for trying out Yazz. This project is still in development, so not
everything works as expected, or at all. I'm happy about any feedback.

**IMPORTANT: The filter is currently unstable, so certain settings will create
noise at maximum volume. Please be careful when making filter changes. Don't
damage your ears.**

## Architecture

Yazz has a fixed signal flow with the following components:

* 32 voices
* 3 wavetable-based oscillators per voice
* 2 (well, currently only one) filters with LP-/ HP- and BP-Modes (well, currently
  only LP) per voice
* 3 ADSR envelopes per voice
* 2 LFOs per voice
* 2 global LFOs
* Delay

## Loading and saving sounds

This functionality is still under construction.

Currently the sound filename is hardwired to "Yazz_FactoryBank.ysn". To load
the file, press F1.

To change the current program, send MIDI program change commands.

**WARNING: All changes made to a sound will be lost when changing the current
program without saving.**

To save a sound, press F2. Currently, only saving the complete sound bank is
supported. Copying a sound, saving single sounds etc. are still on the TODO
list. There is also no safety dialog yet to prevent accidentally overwriting
sounds. Making manual backups might be a good idea if you made a cool sound and
want to keep it.

## Operating modes

Yazz has two distinct operating modes: Edit mode and Play mode. The mode is
switched with the TAB key. The main difference is that Edit mode captures one
dedicated MIDI controller (modulation wheel per default) as data input to
change parameter values, while play mode allows easy switching between MIDI
controller sets.

## Edit Mode: Editing parameters

Yazz is controlled entirely with the (computer-)keyboard and/ or MIDI
controllers. The top line of the UI is the **command line** that is used
for changing parameters.

Every parameter consists of four parts:

* A **function**: The function group, e.g. Oscillator, Envelope etc,
* A **function ID**, in case there is more than one instance of the current
  function, e.g. Oscillator 3, Envelope 1,
* A **parameter**: A parameter of the selected function (e.g. Oscillator Level,
  Envelope Attack),
* A **value**: The actual value of the selected parameter. This can itself be
  another parameter, e.g. for modulation source and target.

To make entering values easier, a MIDI controller can be assigned as value
input control. Currently this is hardwired to the modulation wheel.

### Function selection

When starting to edit a parameter, the function must be selected. The UI shows
a drop-down list of available functions. Every function has a shortcut
associated with it, e.g. 'o' for oscillator, 'l' for LFO. The function can
be selected by:

* Entering the shortcut key,
* Using the CursorUp/ CursorDown keys and pressing Enter or CursorRight.

### Function ID selection

The function ID can be selected by entering the number, using the cursor keys
or using +/ -. If only a single instance of the function is available (e.g.
Delay), the input will immediately progress to the parameter selection.

### Parameter selection

The parameter selection uses the same mechanism as the function selection

### Value selection

The value can be changed by:

* Typing in the target value and pressing Enter, e.g. "3.14" for a float value,
* Using '+'/ '-' to increment/ decrement the value,
* Using the input controller (currently the modulation wheel) to set the value.
* Using a MIDI controller assigned to this parameter to set the value.

After adjusting the value, pressing Enter will return to the Parameter
selection, pressing Escape will return to the function selection.

### Keyboard shortcuts

While setting the value in the command line, there are a number of additional
keyboard shortcuts for faster navigation:

* **PageUp/ PageDown** will change the function ID of the current parameter.
  That way it is easy to adjust the same parameter for all 3 oscillators, or to
  search for the modulation slot that is assigned to a particular target.
* **[/ ]** will step through the parameter list of the selected function.
* **</ >** will move backwards/ forwards through the history of selected
  parameters.
* **"<MarkerId>** adds a marker at the selected parameter. MarkerId can be any
  valid ASCII character.
* **'<MarkerId>** goes to the selected marker if it has been defined.

## Assigning MIDI controllers

Yazz supports assigning MIDI controllers to most sound parameters. To assign a
controller:

* Select the target parameter in the command line (e.g. "o1l" to select
  oscillator 1 level.
* Enter MIDI learn mode by pressing "Ctrl-l". The command line will show the text
  **MIDI learn: Send controller data**.
* Send values with the MIDI controller. Yazz needs at least two distinct values
  to be able to distinguish between absolute and relative controller types.

After having detected the controller, the command line switches back to value
input mode.

To cancel MIDI learn mode without assigning a controller, press Escape.

Controller assignments are global settings, not sound specific. They are saved
automatically after every controller assignment change.

## Modulation ##

Yazz has a flexible modulation matrix, which allows using most signal outputs
as modulation values for sound parameters. There are two different types of
modulation sources and targets:

* Global modulation sources:
    * Channel aftertouch
    * Global LFOs
    * Pitch wheel
* Local modulation sources:
    * Key velocity
    * Per-voice signals (oscillator, envelope and LFO outputs)
* Global modulation targets:
    * Patch volume
    * Delay parameters
* Local modulation targets:
    * Most voice parameters (not all included yet)

To assign a modulator, select one the 20 available Modulation function slots.
Both the source and the target parameters can be entered the same way as
selecting a synth parameter. Modulation source requires only Function and
Function ID, while Modulation Target also requires the Parameter to assign to.

Any modulator can be adjusted in intensity and can be turned on/ off.

## Play Mode: Select controller set
