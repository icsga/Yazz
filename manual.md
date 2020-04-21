# Yazz - Yet Another Subtractive Synth

<!-- vim-markdown-toc GFM -->

* [Introduction](#introduction)
* [Architecture](#architecture)
* [Loading and saving sounds](#loading-and-saving-sounds)
* [Editing parameters](#editing-parameters)
    * [Function selection](#function-selection)
    * [Function ID selection](#function-id-selection)
    * [Parameter selection](#parameter-selection)
    * [Value selection](#value-selection)
    * [Keyboard shortcuts](#keyboard-shortcuts)
* [Assigning MIDI controllers](#assigning-midi-controllers)
* [Modulation](#modulation)

<!-- vim-markdown-toc -->

## Introduction
Thanks for trying out Yazz. This project is still under development, so not
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
* 2 ADSR envelopes per voice
* 2 LFOs per voice
* 2 global LFOs
* Delay

<a text="loading-and-saving-sounds"></a>
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

## Editing parameters

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
* Using the assigned input controller (currently the modulation wheel) to set
  the value.

After adjusting the value, pressing Enter will return to the Parameter
selection, pressing Escape will return to the function selection.

### Keyboard shortcuts

TODO: There will be additional keyboard shortcuts for a faster workflow, e.g.
for updating the function ID from the value input position, and for setting
markers to jump between parameters.

## Assigning MIDI controllers

Yazz supports assigning MIDI controllers to most sound parameters. To assign a
controller:

* Select the target parameter in the command line (e.g. "o1l" to select
  oscillator 1 level.
* Enter MIDI learn mode by pressing "l". The command line will show the text
  **MIDI learn: Send controller data**.
* Send values with the MIDI controller. Yazz needs at least two distinct values
  to be able to distinguish between absolute and relative controller types.

After having detected the controller, the command line switches back to value
input mode.

To cancel MIDI learn mode without assigning a controller, press Escape.

## Modulation ##

Yazz has a flexible modulation matrix, which allows using most signal outputs
as modulation values for sound parameters. There are two different types of
modulation sources and targets:

* Global modulation sources:
    * Channel aftertouch
    * Global LFOs
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
Function ID, while Modulation Target also requires the Parameter to assign.

In addition, any modulator can be adjusted in intensity and can be turned on/
off.

The value ranges for most of these are not finished yet, so not all assignments
have the desired effect, and some will cause the program to exit. Working on
it.

