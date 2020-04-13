/*
- Controller input is absolute or relative
- Relative translates to +/ - keys
- Map is from controller number to parameter ID
- This is unrelated to selector
- Must set sound parameter directly, similar to parameter UI
- Must set both synth sound and tui sound (see tui::send_event())

API:
- Reset map
- Add a mapping (controller number to synth parameter, abs or rel)
- Translate a controller value into a parameter
    In: Controller number, value
    Out: Option(Synth parameter, controller value(abs or rel))
*/

struct CtrlMap {

}
