use super::ParamId;
use super::ItemSelection;
use super::{Parameter, ParameterValue, SynthParam, ValueRange, MenuItem};
use super::MarkerManager;
use super::MidiLearn;
use super::SoundPatch;
use super::{StateMachine, SmEvent, SmResult};

use log::info;
use termion::event::Key;

use std::cell::RefCell;
use std::fmt::{self, Debug};
use std::rc::Rc;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SelectorState {
    Function,
    FunctionIndex,
    Param,
    Value,
    MidiLearn,
    AddMarker,
    GotoMarker,
    ValueFunction,
    ValueFunctionIndex,
    ValueParam,
}

impl fmt::Display for SelectorState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// Defines the order in which items are displayed on the command line.
pub fn next(current: SelectorState) -> SelectorState {
    use SelectorState::*;
    match current {
        Function => FunctionIndex,
        FunctionIndex => Param,
        Param => Value,
        Value => MidiLearn,
        MidiLearn => AddMarker,
        AddMarker => GotoMarker,
        GotoMarker => ValueFunction,
        ValueFunction => ValueFunctionIndex,
        ValueFunctionIndex => ValueParam,
        ValueParam => ValueParam,
    }
}

/// Return code from functions handling user input.
#[derive(Debug, PartialEq)]
pub enum RetCode {
    KeyConsumed,   // Key has been used, but value is not updated yet
    ValueUpdated,  // Key has been used and value has changed to a valid value
    ValueComplete, // Value has changed and will not be updated again
    KeyMissmatch,  // Key has not been used
    Cancel,        // Cancel current operation and go to previous state
    Reset,         // Reset parameter selection to initial state (function selection)
}

/// Type of event that can be pased to the Selector state machine.
pub enum SelectorEvent {
    Key(termion::event::Key), // A key has been pressed
    ControlChange(u64, u64),  // The input controller has been updated
    ValueChange(SynthParam),  // A sound parameter has changed outside of the selector (by MIDI controller)
}

/// Handles user input to change synthesizer values.
///
/// The ParamSelector uses input events (key strokes, MIDI controller events)
/// to change synthesizer parameters. It shows the current state of the
/// parameter change in the command line (top line of the synth TUI).
///
/// When a parameter has changed, the ParamSelector signals this back to the
/// TUI, which in turn sends the changed value to the synth engine.
#[derive(Debug)]
pub struct ParamSelector {
    value_changed: bool,
    pub state: SelectorState,
    pub func_selection: ItemSelection,
    pub param_selection: ItemSelection,
    pub value_func_selection: ItemSelection,
    pub value_param_selection: ItemSelection,
    pub value: ParameterValue,
    pub ml: MidiLearn,
    pub wavetable_list: Vec<(usize, String)>,
    sound: Option<Rc<RefCell<SoundPatch>>>,
    pending_key: Option<Key>,
    history: Vec<ParamId>,
    rev_history: Vec<ParamId>,
    marker_manager: MarkerManager,
    changed_values: Vec<SynthParam>,
}

impl ParamSelector {
    pub fn new(function_list: &'static [MenuItem], mod_list: &'static [MenuItem]) -> ParamSelector {
        let mut func_selection = ItemSelection{
            item_list: function_list,
            item_index: 0,
            value: ParameterValue::Int(1),
            temp_string: String::new()};
        func_selection.reset();
        let mut param_selection = ItemSelection{item_list: function_list[0].next,
            item_index: 0,
            value: ParameterValue::Int(1),
            temp_string: String::new()};
        param_selection.reset();
        let mut value_func_selection = ItemSelection{
            item_list: mod_list,
            item_index: 0,
            value: ParameterValue::Int(1),
            temp_string: String::new()};
        value_func_selection.reset();
        let mut value_param_selection = ItemSelection{item_list: mod_list[0].next,
            item_index: 0,
            value: ParameterValue::Int(1),
            temp_string: String::new()};
        value_param_selection.reset();
        let mut wavetable_list: Vec<(usize, String)> = vec!{};
        wavetable_list.push((0, "Basic".to_string()));
        wavetable_list.push((1, "PWM Square".to_string()));
        ParamSelector{value_changed: false,
                      state: SelectorState::Function,
                      func_selection: func_selection,
                      param_selection: param_selection,
                      value_func_selection: value_func_selection,
                      value_param_selection: value_param_selection,
                      value: ParameterValue::Int(0),
                      ml: MidiLearn::new(),
                      wavetable_list: wavetable_list,
                      sound: Option::None,
                      pending_key: Option::None,
                      history: vec!{},
                      rev_history: vec!{},
                      marker_manager: MarkerManager::new(),
                      changed_values: vec!{},
        }
    }

    pub fn reset(&mut self) -> SmResult<ParamSelector, SelectorEvent> {
        self.func_selection.reset();
        self.param_selection.reset();
        SmResult::ChangeState(ParamSelector::state_function)
    }

    /** Received a keyboard event from the terminal.
     *
     * Return 
     * - true if value has changed and can be sent to the synth engine
     * - false if value has not changed
     */
    pub fn handle_user_input(&mut self,
                             sm: &mut StateMachine<ParamSelector, SelectorEvent>,
                             c: termion::event::Key,
                             sound: Rc<RefCell<SoundPatch>>) -> bool {
        info!("handle_user_input {:?} in state {:?}", c, self.state);
        let result = self.handle_input_event(&SmEvent::Event(SelectorEvent::Key(c)), sm, sound);

        // If we received a key that wasn't used but caused a state change, we
        // need to pass it in again as a new event for it to be handled.
        if let Option::Some(key) = self.pending_key {
            self.pending_key = Option::None;
            sm.handle_event(self, &SmEvent::Event(SelectorEvent::Key(key)));
        }

        result
    }

    /** Received a controller event.
     *
     * This event is used as a direct value control for the UI menu line.
     *
     * Return 
     * - true if value has changed and can be sent to the synth engine
     * - false if value has not changed
     */
    pub fn handle_control_input(&mut self,
                                sm: &mut StateMachine<ParamSelector, SelectorEvent>,
                                controller: u64,
                                value: u64,
                                sound: Rc<RefCell<SoundPatch>>) -> bool {
        info!("handle_control_input {:?} in state {:?}", value, self.state);
        self.handle_input_event(&SmEvent::Event(SelectorEvent::ControlChange(controller, value)), sm, sound)
    }

    fn handle_input_event(&mut self,
                              event: &SmEvent<SelectorEvent>,
                              sm: &mut StateMachine<ParamSelector, SelectorEvent>,
                              sound: Rc<RefCell<SoundPatch>>) -> bool {
        self.sound = Option::Some(Rc::clone(&sound));
        self.value_changed = false;
        sm.handle_event(self, event);
        self.value_changed
    }

    /** Select the function group to edit (Oscillator, Envelope, ...) */
    pub fn state_function(self: &mut ParamSelector,
                      e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_function Enter");
                self.state = SelectorState::Function;
                self.func_selection.value = ParameterValue::Int(1); // TODO: Remember last index based on function
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        match self.func_selection.select_item(*c) {
                            RetCode::KeyConsumed   => SmResult::EventHandled, // Selection updated
                            RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore key that doesn't match a selection
                            RetCode::ValueUpdated  => SmResult::EventHandled, // Selection updated
                            RetCode::Cancel        => SmResult::EventHandled,
                            RetCode::ValueComplete => {
                                // Function selected
                                self.param_selection.set_list_from(&self.func_selection, 0);

                                // Check if there are more then one instances of the selected function
                                let val_range = self.func_selection.get_val_range();
                                if let ValueRange::Int(_, num_instances) = val_range {
                                    if *num_instances == 1 {
                                        // Single instance, skip function_index state
                                        self.func_selection.value = ParameterValue::Int(1);
                                        self.query_current_value();
                                        SmResult::ChangeState(ParamSelector::state_parameter)
                                    } else {
                                        SmResult::ChangeState(ParamSelector::state_function_index)
                                    }
                                } else {
                                    panic!();
                                }
                            },
                            RetCode::Reset         => self.reset(),
                        }
                    }
                    SelectorEvent::ControlChange(_, _) => SmResult::EventHandled,
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    /** Select which item in the function group to edit (Oscillator 1, 2, 3, ...) */
    fn state_function_index(self: &mut ParamSelector,
                            e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_function_index Enter");
                self.state = SelectorState::FunctionIndex;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                self.func_selection.temp_string.clear();
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        match self.func_selection.handle_input(*c, self.wavetable_list.len() - 1) {
                            RetCode::KeyConsumed   => SmResult::EventHandled, // Key has been used, but value hasn't changed
                            RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore unmatched keys
                            RetCode::ValueUpdated  => SmResult::EventHandled, // Selection not complete yet
                            RetCode::ValueComplete => {
                                // Parameter has been selected, fetch current value from sound
                                self.query_current_value();
                                SmResult::ChangeState(ParamSelector::state_parameter)
                            },
                            RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_function), // Abort function index selection
                            RetCode::Reset         => self.reset(),
                        }
                    }
                    SelectorEvent::ControlChange(_, _) => SmResult::EventHandled,
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    /** Select the parameter of the function to edit (Waveshape, Frequency, ...) */
    fn state_parameter(self: &mut ParamSelector,
                       e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_parameter Enter");
                self.state = SelectorState::Param;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        let result = self.handle_navigation_keys(c);
                        match result {
                            SmResult::Error => (), // Continue processing the key
                            _ => return result     // Key was handled
                        }
                        match self.param_selection.select_item(*c) {
                            RetCode::KeyConsumed   => SmResult::EventHandled, // Value has changed, but not complete yet
                            RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore invalid key
                            RetCode::ValueUpdated  => {                       // Pararmeter selection updated
                                self.query_current_value();
                                SmResult::EventHandled
                            },
                            RetCode::ValueComplete => {                       // Prepare to read the value
                                self.query_current_value();
                                self.history_add(self.get_param_id());
                                self.change_to_value_state()
                                
                            },
                            RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_function_index), // Cancel parameter selection
                            RetCode::Reset         => self.reset(),
                        }
                    }
                    SelectorEvent::ControlChange(_, _) => SmResult::ChangeState(ParamSelector::state_value),
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    /** Select the parameter value */
    fn state_value(self: &mut ParamSelector,
                   e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_value Enter");
                self.state = SelectorState::Value;
                self.param_selection.temp_string.clear();
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        let result = self.handle_navigation_keys(c);
                        match result {
                            SmResult::Error => (), // Continue processing the key
                            _ => return result     // Key was handled
                        }
                        let result = self.handle_state_value_keys(c);
                        match result {
                            SmResult::Error => (), // Continue processing the key
                            _ => return result     // Key was handled
                        }
                        if let Key::Ctrl('l') = c {
                            return SmResult::ChangeState(ParamSelector::state_midi_learn);
                        }
                        match self.param_selection.handle_input(*c, self.wavetable_list.len() - 1) {
                            RetCode::KeyConsumed   => SmResult::EventHandled,
                            RetCode::KeyMissmatch  => {
                                // Key can't be used for value, so it probably is the short cut for a
                                // different parameter. Switch to parameter state and try again.
                                self.pending_key = Option::Some(*c);
                                SmResult::ChangeState(ParamSelector::state_parameter)
                            },
                            RetCode::ValueUpdated  => {
                                // Value has changed to a valid value, update synth
                                self.value_changed = true;
                                SmResult::EventHandled
                            },
                            RetCode::ValueComplete => {
                                // Value has changed and will not be updated again
                                self.value_changed = true;
                                SmResult::ChangeState(ParamSelector::state_parameter)
                            }
                            RetCode::Cancel => SmResult::ChangeState(ParamSelector::state_parameter), // Stop updating the value, back to parameter selection
                            RetCode::Reset => self.reset(),
                        }
                    }
                    SelectorEvent::ControlChange(_, value) => {
                        self.param_selection.set_control_value(*value);
                        self.value_changed = true;
                        SmResult::EventHandled
                    }
                    SelectorEvent::ValueChange(p) => {
                        let selected_param = self.get_param_id();
                        if p.equals(&selected_param) {
                            self.param_selection.update_value(p.value);
                        }
                        SmResult::EventHandled
                    }
                }
            }
        }
    }

    /** Select the function group to edit (Oscillator, Envelope, ...) */
    fn state_value_function(self: &mut ParamSelector,
                            e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_value_function Enter");
                self.state = SelectorState::ValueFunction;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                // TODO: This is copied here from state_value_function_index.
                //       Clean this mess up.
                if self.value_changed {
                    match self.param_selection.value {
                        ParameterValue::Function(ref mut id) => {
                            info!("Saving function value");
                            id.function = self.value_func_selection.item_list[self.value_func_selection.item_index].item;
                            id.function_id = if let ParameterValue::Int(x) = self.value_func_selection.value { x as usize } else { panic!() };
                        },
                        _ => panic!(),
                    }
                }
                self.value_func_selection.temp_string.clear();
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        let result = self.handle_navigation_keys(c);
                        match result {
                            SmResult::Error => (), // Continue processing the key
                            _ => return result     // Key was handled
                        }
                        match self.value_func_selection.select_item(*c) {
                            RetCode::KeyConsumed   => SmResult::EventHandled, // Selection updated
                            RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore key that doesn't match a selection
                            RetCode::ValueUpdated  => SmResult::EventHandled, // Selection updated
                            RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_parameter), // Stop updating the value, back to parameter selection
                            RetCode::ValueComplete => { // Function selected
                                // Check if there are more then one instances of the selected function
                                let val_range = self.value_func_selection.get_val_range();
                                if let ValueRange::Int(_, num_instances) = val_range {
                                    if *num_instances == 1 {
                                        // Only single instance of this
                                        // function available, so no need to go
                                        // to state_value_function_index.
                                        self.value_func_selection.value = ParameterValue::Int(1);
                                        match self.param_selection.value {
                                            ParameterValue::Function(_) => {
                                                // Value is finished
                                                self.value_changed = true;
                                                SmResult::ChangeState(ParamSelector::state_parameter)
                                            }
                                            ParameterValue::Param(_) => {
                                                // Parameter has to be selected
                                                self.value_func_selection.value = ParameterValue::Int(1);
                                                self.value_param_selection.set_list_from(&self.value_func_selection, 0);
                                                SmResult::ChangeState(ParamSelector::state_value_parameter)
                                            }
                                            _ => panic!(),
                                        }
                                    } else {
                                        SmResult::ChangeState(ParamSelector::state_value_function_index)
                                    }
                                } else {
                                    panic!();
                                }
                            }
                            RetCode::Reset         => self.reset(),
                        }
                    }
                    SelectorEvent::ControlChange(_, _) => SmResult::ChangeState(ParamSelector::state_value_function_index),
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    /** Select which item in the function group to edit (Oscillator 1, 2, 3, ...) */
    fn state_value_function_index(self: &mut ParamSelector,
                                  e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_value_function_index Enter");
                self.state = SelectorState::ValueFunctionIndex;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                if self.value_changed {
                    match self.param_selection.value {
                        ParameterValue::Function(ref mut id) => {
                            info!("Saving function value");
                            id.function = self.value_func_selection.item_list[self.value_func_selection.item_index].item;
                            id.function_id = if let ParameterValue::Int(x) = self.value_func_selection.value { x as usize } else { panic!() };
                        },
                        _ => panic!(),
                    }
                }
                self.value_func_selection.temp_string.clear();
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        let result = self.handle_navigation_keys(c);
                        match result {
                            SmResult::Error => (), // Continue processing the key
                            _ => return result     // Key was handled
                        }
                        match self.value_func_selection.handle_input(*c, self.wavetable_list.len() - 1) {
                            RetCode::KeyConsumed   => SmResult::EventHandled, // Key has been used, but value hasn't changed
                            RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore unmatched keys
                            RetCode::ValueUpdated  => SmResult::EventHandled, // Selection not complete yet
                            RetCode::ValueComplete => {                       // Parameter has been selected
                                // For modulation source or target, we might be finished here with
                                // getting the value. Compare current state to expected target state.
                                match self.param_selection.value {
                                    ParameterValue::Function(_) => {
                                        // Value is finished
                                        self.value_changed = true;
                                        SmResult::ChangeState(ParamSelector::state_parameter)
                                    },
                                    ParameterValue::Param(_) => {
                                        self.value_param_selection.set_list_from(&self.value_func_selection, 0);

                                        // If function is same as before, set
                                        // value_param_selection.item_index to current value,
                                        // else set to valid value.
                                        self.query_current_value_param_index();

                                        SmResult::ChangeState(ParamSelector::state_value_parameter)
                                    },
                                    _ => SmResult::EventHandled,
                                }
                            },
                            RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_value_parameter), // Abort function index selection
                            RetCode::Reset         => self.reset(),
                        }
                    }
                    SelectorEvent::ControlChange(_, value) => {
                        self.value_func_selection.set_control_value(*value);
                        SmResult::EventHandled
                    }
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    /** Select the parameter of the function to edit (Waveshape, Frequency, ...) */
    fn state_value_parameter(self: &mut ParamSelector,
                             e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_value_parameter Enter");
                self.state = SelectorState::ValueParam;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                if self.value_changed {
                    match self.param_selection.value {
                        ParameterValue::Param(ref mut id) => {
                            id.function = self.value_func_selection.item_list[self.value_func_selection.item_index].item;
                            id.function_id = if let ParameterValue::Int(x) = self.value_func_selection.value { x as usize } else { panic!() };
                            id.parameter = self.value_param_selection.item_list[self.value_param_selection.item_index].item;
                            info!("Saving parameter value {:?}", id);
                        },
                        _ => panic!(),
                    }
                }
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        let result = self.handle_navigation_keys(c);
                        match result {
                            SmResult::Error => (), // Continue processing the key
                            _ => return result     // Key was handled
                        }
                        match self.value_param_selection.select_item(*c) {
                            RetCode::KeyConsumed   => SmResult::EventHandled, // Value has changed, but not complete yet
                            RetCode::KeyMissmatch  => SmResult::EventHandled, // Ignore invalid key
                            RetCode::ValueUpdated  => {                       // Pararmeter selection updated
                                SmResult::EventHandled
                            },
                            RetCode::ValueComplete => {                       // Prepare to read the value
                                self.value_changed = true;
                                SmResult::ChangeState(ParamSelector::state_parameter)
                            },
                            RetCode::Cancel        => SmResult::ChangeState(ParamSelector::state_value_function_index), // Abort function index selection
                            RetCode::Reset         => self.reset(),
                        }
                    }
                    SelectorEvent::ControlChange(_, _) => {
                        SmResult::EventHandled
                    }
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    /** Assign a MIDI controller to control the current parameter. */
    fn state_midi_learn(self: &mut ParamSelector,
                        e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_midi_learn Enter");
                self.state = SelectorState::MidiLearn;
                self.ml.reset();
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        match c {
                            Key::Esc => SmResult::ChangeState(ParamSelector::state_value),
                            Key::Backspace => {
                                self.ml.clear_controller();
                                SmResult::ChangeState(ParamSelector::state_value)
                            }
                            _ => SmResult::EventHandled
                        }
                    }
                    SelectorEvent::ControlChange(ctrl, value) => {
                        if self.ml.handle_controller(*ctrl, *value) {
                            SmResult::ChangeState(ParamSelector::state_value)
                        } else {
                            SmResult::EventHandled
                        }
                    }
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    /** Assign a marker to the currently selected parameter.
     *
     * Recalling the marker while editing a different parameter will reset the
     * command line to this parameter.
     */
    fn state_add_marker(self: &mut ParamSelector,
                        e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_add_marker Enter");
                self.state = SelectorState::AddMarker;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        match c {
                            Key::Char(m) => {
                                self.marker_manager.add(*m, self.get_param_id());
                                self.change_to_value_state()
                            }
                            Key::Esc => self.change_to_value_state(),
                            _ => SmResult::EventHandled
                        }
                    }
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    /** Reset the input state to the parameter of the marker. */
    fn state_goto_marker(self: &mut ParamSelector,
                        e: &SmEvent<SelectorEvent>)
    -> SmResult<ParamSelector, SelectorEvent> {
        match e {
            SmEvent::EnterState => {
                info!("state_goto_marker Enter");
                self.state = SelectorState::GotoMarker;
                SmResult::EventHandled
            }
            SmEvent::ExitState => {
                SmResult::EventHandled
            }
            SmEvent::Event(selector_event) => {
                match selector_event {
                    SelectorEvent::Key(c) => {
                        match c {
                            Key::Char(c) => {
                                match self.marker_manager.get(*c) {
                                    Some(param_id) => self.apply_param_id(&param_id),
                                    None => ()
                                };
                                self.change_to_value_state()
                            }
                            Key::Esc => self.change_to_value_state(),
                            _ => SmResult::EventHandled
                        }
                    }
                    _ => SmResult::EventHandled,
                }
            }
        }
    }

    /** Change to the value state that matches the selected parameter. */
    fn change_to_value_state(&self) -> SmResult<ParamSelector, SelectorEvent> {
        match self.param_selection.value {
            ParameterValue::Function(_) => SmResult::ChangeState(ParamSelector::state_value_function),
            ParameterValue::Param(_) => SmResult::ChangeState(ParamSelector::state_value_function),
            ParameterValue::NoValue => panic!(),
            _ => SmResult::ChangeState(ParamSelector::state_value),
        }
    }

    /** Handle some shortcut keys for easier navigation.
     *
     * - PageUp/ PageDown change the function ID
     * - '['/- ']' change the selected parameter
     * - '<'/ '>' go back/ forwards in the edit history
     * - '\"'/ '\'' set and recall markers for parameters
     */
    fn handle_navigation_keys(&mut self, c: &termion::event::Key)
            -> SmResult<ParamSelector, SelectorEvent> {
        match *c {
            Key::PageUp => {
                self.increase_function_id();
                return SmResult::EventHandled;
            }
            Key::PageDown => {
                self.decrease_function_id();
                return SmResult::EventHandled;
            }
            Key::Char(c) => {
                let param_changed = match c {
                    ']' => {
                        self.increase_param_id();
                        true
                    }
                    '[' => {
                        self.decrease_param_id();
                        true
                    }
                    '<' => {
                        self.history_backwards();
                        true
                    }
                    '>' => {
                        self.history_forwards();
                        true
                    }
                    '"' => {
                        return SmResult::ChangeState(ParamSelector::state_add_marker);
                    }
                    '\'' => {
                        return SmResult::ChangeState(ParamSelector::state_goto_marker);
                    }
                    _ => false
                };
                if param_changed {
                    // The input state needs to match the value type of the new
                    // parameter.
                    match self.param_selection.value {
                        ParameterValue::Function(_) |
                        ParameterValue::Param(_) => {
                            if self.state == SelectorState::Value {
                                return SmResult::ChangeState(ParamSelector::state_value_function);
                            }
                        }
                        _ => {
                            if self.state != SelectorState::Value
                            && self.state != SelectorState::Param {
                                return SmResult::ChangeState(ParamSelector::state_value);
                            }
                        }
                    }
                    return SmResult::EventHandled;
                }
            }
            _ => ()
        }
        SmResult::Error // Not really an error, but event not handled
    }

    fn increase_function_id(&mut self) {
        self.func_selection.handle_input(Key::Up, self.wavetable_list.len() - 1);
        self.query_current_value();
        self.history_add(self.get_param_id());
    }

    fn decrease_function_id(&mut self) {
        self.func_selection.handle_input(Key::Down, self.wavetable_list.len() - 1);
        self.query_current_value();
        self.history_add(self.get_param_id());
    }

    fn increase_param_id(&mut self) {
        self.param_selection.select_item(Key::Up);
        self.query_current_value();
        self.history_add(self.get_param_id());
    }

    fn decrease_param_id(&mut self) {
        self.param_selection.select_item(Key::Down);
        self.query_current_value();
        self.history_add(self.get_param_id());
    }

    fn history_add(&mut self, p: ParamId) {
        self.history.push(p);
        self.rev_history.clear();
    }

    fn history_backwards(&mut self) {
        if self.history.len() < 2 {
            return; // Nothing to go back to
        }
        let current_param_id = self.history.pop();
        if let Some(p) = current_param_id {
            self.rev_history.push(p);
        } else {
            panic!();
        }
        let prev_param_id = self.history.last();
        let p = if let Some(p) = prev_param_id { *p } else { panic!() };
        self.apply_param_id(&p);
    }

    fn history_forwards(&mut self) {
        if self.rev_history.len() < 1 {
            return; // Nothing to go forward to
        }
        let current_param_id = self.rev_history.pop();
        if let Some(p) = current_param_id {
            self.history.push(p);
        } else {
            panic!();
        }
        let prev_param_id = self.history.last();
        let p = if let Some(p) = prev_param_id { *p } else { panic!() };
        self.apply_param_id(&p);
    }

    /** Handle some extra shortcuts for state_value.
     *
     * - Ctrl-L switches to MIDI learn mode
     * - '/' creates a new modulator for the current parameter
     * - '?' searches active modulators for the current parameter
     */
    fn handle_state_value_keys(&mut self, c: &termion::event::Key)
            -> SmResult<ParamSelector, SelectorEvent> {
        match *c {
            Key::Char(c) => {
                let param_changed = match c {
                    '/' => self.create_modulator(),
                    '?' => self.search_modulator(),
                    _ => false
                };
                if param_changed {
                    // The input state needs to match the value type of the new
                    // parameter.
                    match self.param_selection.value {
                        ParameterValue::Function(_) |
                        ParameterValue::Param(_) => {
                            if self.state == SelectorState::Value {
                                return SmResult::ChangeState(ParamSelector::state_value_function);
                            }
                        }
                        _ => {
                            if self.state != SelectorState::Value
                            && self.state != SelectorState::Param {
                                return SmResult::ChangeState(ParamSelector::state_value);
                            }
                        }
                    }
                    return SmResult::EventHandled;
                }
            }
            _ => ()
        }
        SmResult::Error // Not really an error, but event not handled
    }

    fn create_modulator(&mut self) -> bool {
        // Find a modulator that is inactive
        let sound = if let Some(sound) = &self.sound { sound } else { panic!() };
        let mut found = false;
        let mut param_id = self.get_param_id();
        let mut index: usize = 0;
        {
            let modulators = &sound.borrow().data.modul;
            let len = modulators.len();
            for i in 0..len {
                if modulators[i].active == false {
                    index = i;
                    found = true;
                    break;
                }
            }
        }
        if !found { return false; }

        // Add changed parameters to the change stack. They will be applied by
        // Tui after this function finishes.
        let mut sp = SynthParam::new(Parameter::Modulation, index + 1, Parameter::Target, ParameterValue::Param(param_id));
        self.add_changed_value(&sp);
        sp.parameter = Parameter::Active;
        sp.value = ParameterValue::Int(1);
        self.add_changed_value(&sp);
        sp.parameter = Parameter::Amount;
        sp.value = ParameterValue::Float(1.0);
        self.add_changed_value(&sp);
        self.value_changed = true;

        // Prepare param_id to match current param. This is the position that
        // the command line goes to after applying the values.
        param_id.function = Parameter::Modulation;
        param_id.function_id = index + 1;
        param_id.parameter = Parameter::Source;
        self.apply_param_id(&param_id);
        true
    }

    fn add_changed_value(&mut self, parameter: &SynthParam) {
        self.changed_values.push(*parameter);
    }

    pub fn get_changed_value(&mut self) -> Option<SynthParam> {
        self.changed_values.pop()
    }

    fn search_modulator(&mut self) -> bool {
        true
    }

    fn apply_param_id(&mut self, param_id: &ParamId) {
        self.func_selection.item_index = MenuItem::get_item_index(param_id.function, &self.func_selection.item_list);
        self.func_selection.value = ParameterValue::Int(param_id.function_id as i64);
        self.param_selection.set_list_from(&self.func_selection, 0);
        let item_index = MenuItem::get_item_index(param_id.parameter, &self.param_selection.item_list);
        self.param_selection.item_index = item_index;
        self.query_current_value();
    }

    fn get_param_id_internal(fs: &ItemSelection, ps: &ItemSelection) -> ParamId {
        let function = &fs.item_list[fs.item_index];
        let function_id = if let ParameterValue::Int(x) = &fs.value { *x as usize } else { panic!() };
        let parameter = &ps.item_list[ps.item_index];
        ParamId::new(function.item, function_id, parameter.item)
    }

    /** Return a ParamId matching the currently selected parameter.
     *
     * A ParamId does not include the value of the parameter.
     */
    pub fn get_param_id(&self) -> ParamId {
        ParamSelector::get_param_id_internal(&self.func_selection, &self.param_selection)
    }

    /** Return a SynthParam matching the currently selected parameter.
     *
     * A SynthParam does include the current value of the selected parameter.
     */
    pub fn get_synth_param(&self) -> SynthParam {
        let function = &self.func_selection.item_list[self.func_selection.item_index];
        let function_id = if let ParameterValue::Int(x) = &self.func_selection.value { *x as usize } else { panic!() };
        let parameter = &self.param_selection.item_list[self.param_selection.item_index];
        SynthParam::new(function.item, function_id, parameter.item, self.param_selection.value)
    }

    /** Get the value range for currently selected parameter. */
    pub fn get_value_range(&self) -> &'static ValueRange {
        self.param_selection.get_val_range()
    }

    /** Select the parameter chosen by input, set value to current sound data. */
    fn query_current_value(&mut self) {
        let param = self.get_param_id();
        info!("query_current_value {:?}", param);

        // The value in the selected parameter needs to point to the right type.
        let sound = if let Some(sound) = &self.sound { sound } else { panic!() };
        let value = sound.borrow().data.get_value(&param);
        info!("Sound has value {:?}", value);
        match value {
            // Let the two ItemSelection structs point to the currently active
            // values.
            ParameterValue::Function(id) => {
                let fs = &mut self.value_func_selection;
                fs.set_list_from(&self.param_selection, 0);
                fs.item_index = MenuItem::get_item_index(id.function, fs.item_list);
                fs.value = ParameterValue::Int(id.function_id as i64);
            }
            ParameterValue::Param(id) => {
                let fs = &mut self.value_func_selection;
                fs.set_list_from(&self.param_selection, 0);
                fs.item_index = MenuItem::get_item_index(id.function, fs.item_list);
                fs.value = ParameterValue::Int(id.function_id as i64);
                let ps = &mut self.value_param_selection;
                ps.set_list_from(fs, 0);
                ps.item_index = MenuItem::get_item_index(id.parameter, ps.item_list);
            }
            ParameterValue::NoValue => panic!(),
            _ => ()
        }
        self.param_selection.value = value;
    }

    /** Select the parameter chosen by input, set value to current sound data. */
    fn query_current_value_param_index(&mut self) {
        // Get current sound entry
        // Compare function
        // If equal, set same param Index, else set to 0

        let param = self.get_param_id();
        info!("query_current_value_param_index {:?}", param);

        // The value in the selected parameter needs to point to the right type.
        let sound = if let Some(sound) = &self.sound { sound } else { panic!() };
        let value = sound.borrow().data.get_value(&param);
        info!("Sound has value {:?}", value);

        match value {
            ParameterValue::Param(id) => {
                let fs = &self.value_func_selection;
                let ps = &mut self.value_param_selection;
                let function = &fs.item_list[fs.item_index];
                if function.item == id.function {
                    info!("Same function, keep parameter index");
                    ps.item_index = MenuItem::get_item_index(id.parameter, ps.item_list);
                } else {
                    info!("Function differs, set parameter index to safe value");
                    ps.item_index = 1;
                }
            }
            _ => ()
        }
    }

    /** A value has changed outside of the selector, update the local value. */
    pub fn value_has_changed(&mut self, sm: &mut StateMachine<ParamSelector, SelectorEvent>, param: SynthParam) {
        sm.handle_event(self, &SmEvent::Event(SelectorEvent::ValueChange(param)));
    }

    /** Get a dynamic list for the given parameter.
     *
     * Some parameter lists change during the runtime of the program. This
     * function is responsible for returning a matching list to the caller.
     * Example: The wavetable list can change after scanning a directory.
     */
    pub fn get_dynamic_list<'a>(&'a mut self, param: Parameter) -> &'a mut Vec<(usize, String)> {
        match param {
            Parameter::Wavetable => return &mut self.wavetable_list,
            _ => panic!()
        }
    }

    /** Same as get_dynamic_list, but unmutable reference. */
    pub fn get_dynamic_list_no_mut<'a>(&'a self, param: Parameter) -> &'a Vec<(usize, String)> {
        match param {
            Parameter::Wavetable => return &self.wavetable_list,
            _ => panic!()
        }
    }
}

// ----------------------------------------------
//                  Unit tests
// ----------------------------------------------

#[cfg(test)]
mod tests {

use super::{ParamSelector, SelectorEvent, SelectorState};

use super::super::FunctionId;
use super::super::Float;
use super::super::{Parameter, ParameterValue, ParamId, FUNCTIONS, MOD_SOURCES};
use super::super::SoundPatch;
use super::super::StateMachine;

use log::info;
use termion::event::Key;

use std::cell::RefCell;
use std::rc::Rc;

struct TestContext {
    ps: ParamSelector,
    sound: Rc<RefCell<SoundPatch>>,
    sm: StateMachine<ParamSelector, SelectorEvent>,
}

#[derive(Debug)]
enum TestInput {
    Chars(String),
    Key(Key),
    ControlChange(u64, u64),
}

use flexi_logger::{Logger, opt_format};
static mut LOGGER_INITIALIZED: bool = false;

impl TestContext {

    fn new() -> TestContext {
        // Setup logging if required
        unsafe {
            if LOGGER_INITIALIZED == false {
                Logger::with_env_or_str("myprog=debug, mylib=warn")
                                        .log_to_file()
                                        .directory("log_files")
                                        .format(opt_format)
                                        .start()
                                        .unwrap();
                LOGGER_INITIALIZED = true;
            }
        }

        let ps = ParamSelector::new(&FUNCTIONS, &MOD_SOURCES);
        let sound = Rc::new(RefCell::new(SoundPatch::new()));
        let sm = StateMachine::new(ParamSelector::state_function);
        TestContext{ps, sound, sm}
    }

    /* Return 
     * - true if a value has changed and can be sent to the synth engine
     * - false if no value has
     */
    fn do_handle_input(&mut self, input: &TestInput) -> bool {
        let mut result = false;
        match input {
            TestInput::Chars(chars) => {
                for c in chars.chars() {
                    let k = Key::Char(c);
                    result = ParamSelector::handle_user_input(&mut self.ps, &mut self.sm, k, self.sound.clone());
                    self.update_sound_if(result);
                }
            }
            TestInput::Key(k) => {
                result = ParamSelector::handle_user_input(&mut self.ps, &mut self.sm, *k, self.sound.clone());
                self.update_sound_if(result);
            }
            TestInput::ControlChange(controller, value) => {
                result = ParamSelector::handle_control_input(&mut self.ps, &mut self.sm, *controller, *value, self.sound.clone());
                self.update_sound_if(result);
            }
        }
        result
    }

    fn update_sound_if(&mut self, value_changed: bool) {
        if value_changed {
            let param = self.ps.get_synth_param();
            self.sound.borrow_mut().data.set_parameter(&param);
        }
    }

    /* Process a single string of input characters.
     *
     * Return 
     * - true if a value has changed and can be sent to the synth engine
     * - false if no value has
     */
    fn handle_input(&mut self, input: TestInput) -> bool {
        self.do_handle_input(&input)
    }

    /* Process an array of strings of input characters.
     *
     * Used for entering a mix of ASCII and control characters, which have
     * different representations in the Key enum.
     *
     * Return 
     * - true if a value has changed and can be sent to the synth engine
     * - false if no value has
     */
    fn handle_inputs(&mut self, input: &[TestInput]) -> bool {
        let mut result = false;
        for i in input {
            result = self.do_handle_input(i);
        }
        result
    }

    /* Compare the selected function to the expected one.
     *
     * \return true if the functions match, false otherwise
     */
    fn verify_function(&self, func: Parameter) -> bool {
        let fs = &self.ps.func_selection;
        let function = fs.item_list[fs.item_index].item;
        if function != func {
            info!("Expected function {}, found {}", func, function);
            false
        } else {
            true
        }
    }

    /* Compare the selected function ID to the expected one.
     *
     * \return true if the function IDs match, false otherwise
     */
    fn verify_function_id(&self, func_id: usize) -> bool {
        let fs = &self.ps.func_selection;
        let function_id = if let ParameterValue::Int(x) = &fs.value { *x as usize } else { panic!() };
        if function_id != func_id {
            info!("Expected function ID {}, found {}", func_id, function_id);
            false
        } else {
            true
        }
    }

    /* Compare the selected parameter to the expected one.
     *
     * \return true if the parameters match, false otherwise
     */
    fn verify_parameter(&self, param: Parameter) -> bool {
        let ps = &self.ps.param_selection;
        let parameter = ps.item_list[ps.item_index].item;
        if parameter != param {
            info!("Expected parameter {}, found {}", param, parameter);
            false
        } else {
            true
        }
    }

    /* Compare the current value to the expected one.
     *
     * \return true if the values match, false otherwise
     */
    fn verify_value(&self, value: ParameterValue) -> bool {
        let ps = &self.ps.param_selection;
        info!("Checking value {:?}", ps.value);
        match value {
            ParameterValue::Int(expected) => {
                let actual = if let ParameterValue::Int(j) = ps.value { j } else { panic!() };
                if expected != actual {
                    info!("Expected value {}, actual {}", expected, actual);
                    false
                } else {
                    true
                }
            }
            ParameterValue::Float(expected) => {
                let actual = if let ParameterValue::Float(j) = ps.value { j } else { panic!() };
                if expected != actual {
                    info!("Expected value {}, actual {}", expected, actual);
                    false
                } else {
                    true
                }
            },
            ParameterValue::Choice(expected) => {
                let actual = if let ParameterValue::Choice(j) = ps.value { j } else { panic!() };
                if expected != actual {
                    info!("Expected value {}, actual {}", expected, actual);
                    false
                } else {
                    true
                }
            },
            ParameterValue::Function(expected) => {
                let actual = if let ParameterValue::Function(j) = ps.value { j } else { panic!() };
                if expected.function != actual.function
                || expected.function_id != actual.function_id {
                    info!("Expected value {:?}, actual {:?}", expected, actual);
                    false
                } else {
                    true
                }
            }
            ParameterValue::Param(expected) => {
                let actual = if let ParameterValue::Param(j) = ps.value { j } else { panic!() };
                if expected.function != actual.function
                || expected.function_id != actual.function_id {
                    info!("Expected value {:?}, actual {:?}", expected, actual);
                    false
                } else {
                    true
                }
            }
            _ => panic!(),
        }
    }

    /* Compare the current selection to the expected one.
     *
     * \return true if all values match, false otherwise
     */
    fn verify_selection(&self,
                        func: Parameter,
                        func_id: usize,
                        param: Parameter,
                        value: ParameterValue) -> bool {
        self.verify_function(func) &&
        self.verify_function_id(func_id) &&
        self.verify_parameter(param) &&
        self.verify_value(value)
    }
}

// -----------------------
// Test function selection
// -----------------------

const DEFAULT_LEVEL: Float = 50.0;
const DEFAULT_ATTACK: Float = 15.0;
const DEFAULT_DECAY: Float = 15.0;
const DEFAULT_SUSTAIN: Float = 1.0;
const DEFAULT_RELEASE: Float = 15.0;
const DEFAULT_FACTOR: i64 = 1;

// ----------------
// Basic navigation
// ----------------

#[test]
fn direct_shortcuts_select_parameter() {
    let mut context = TestContext::new();

    // Initial state: Osc 1 something
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));
    assert_eq!(context.ps.state, SelectorState::Function);

    // Change to envelope 2 Sustain selection
    assert_eq!(context.handle_input(TestInput::Chars("e".to_string())), false);
    assert!(context.verify_function(Parameter::Envelope));
    assert_eq!(context.ps.state, SelectorState::FunctionIndex);
    assert!(context.verify_function_id(1));

    assert_eq!(context.handle_input(TestInput::Chars("2".to_string())), false);
    assert!(context.verify_function_id(2));
    assert_eq!(context.ps.state, SelectorState::Param);

    assert_eq!(context.handle_input(TestInput::Chars("s".to_string())), false);
    assert!(context.verify_parameter(Parameter::Sustain));
    assert_eq!(context.ps.state, SelectorState::Value);
}

#[test]
fn invalid_shortcut_doesnt_change_function() {
    let mut context = TestContext::new();
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));
    assert_eq!(context.handle_input(TestInput::Chars("@".to_string())), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));
    assert_eq!(context.ps.state, SelectorState::Function);
}

#[test]
fn function_id_can_be_entered_directly() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("o2v".to_string()));
    assert!(context.verify_selection(Parameter::Oscillator, 2, Parameter::Voices, ParameterValue::Int(1)));

    // Modulators have an ID range > 10, but < 20. Entering a '2' should
    // directly switch to the parameter selection, since adding another digit
    // would produce an invalid ID.
    let c: &[TestInput] = &[TestInput::Key(Key::Esc), TestInput::Chars("m2a".to_string())];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Modulation, 2, Parameter::Amount, ParameterValue::Float(0.0)));

    // Entering a '1' should wait for a possible second digit.
    let c: &[TestInput] = &[TestInput::Key(Key::Esc), TestInput::Chars("m12a".to_string())];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Modulation, 12, Parameter::Amount, ParameterValue::Float(0.0)));
}

#[test]
fn tempstring_for_function_id_is_cleared() {
    let mut context = TestContext::new();

    // Enter a double-digit value to fill the tempstring.
    let c: &[TestInput] = &[TestInput::Key(Key::Esc), TestInput::Chars("m12a".to_string())];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Modulation, 12, Parameter::Amount, ParameterValue::Float(0.0)));

    // On next selection, the tempstring should be cleared, so a single digit
    // value will work.
    let c: &[TestInput] = &[TestInput::Key(Key::Esc), TestInput::Chars("m2a".to_string())];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Modulation, 2, Parameter::Amount, ParameterValue::Float(0.0)));
}

#[test]
fn state_function_id_is_skipped_for_len_1() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("d".to_string())); // Only single delay, skip function ID input
    assert_eq!(context.ps.state, SelectorState::Param);
}

#[test]
fn multi_digit_index() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("m".to_string()));
    assert!(context.verify_function(Parameter::Modulation));
    assert!(context.verify_function_id(1));
    context.handle_input(TestInput::Chars("12".to_string()));
    assert!(context.verify_function_id(12));
    assert_eq!(context.ps.state, SelectorState::Param);
}

#[test]
fn param_selection_reads_current_value() {
    let mut context = TestContext::new();
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));

    // Change to level selection, reads current value from sound data
    assert_eq!(context.handle_input(TestInput::Chars("o1l".to_string())), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(DEFAULT_LEVEL)));
}

#[test]
fn alpha_key_in_state_value_selects_parameter() {
    let mut context = TestContext::new();

    context.handle_input(TestInput::Chars("o1l3v".to_string()));

    // Verify that we left level input and are now in voices
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Voices, ParameterValue::Int(1)));

    // Go back to level and verify the correct value was set
    let c: &[TestInput] = &[TestInput::Key(Key::Esc), TestInput::Chars("o1l".to_string())];
    context.handle_inputs(c);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(3.0)));
}

#[test]
fn cursor_navigation() {
    let mut context = TestContext::new();
    assert_eq!(context.ps.state, SelectorState::Function);

    // Forwards
    assert_eq!(context.handle_input(TestInput::Key(Key::Up)), false);
    assert!(context.verify_function(Parameter::Envelope));
    assert_eq!(context.handle_input(TestInput::Key(Key::Right)), false);
    assert_eq!(context.ps.state, SelectorState::FunctionIndex);
    assert!(context.verify_function_id(1));
    assert_eq!(context.handle_input(TestInput::Key(Key::Up)), false);
    assert!(context.verify_function_id(2));
    assert_eq!(context.handle_input(TestInput::Key(Key::Right)), false);
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Attack));
    assert_eq!(context.handle_input(TestInput::Key(Key::Up)), false);
    assert!(context.verify_parameter(Parameter::Decay));
    assert_eq!(context.handle_input(TestInput::Key(Key::Right)), false);
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_value(ParameterValue::Float(DEFAULT_DECAY)));

    // Backwards
    assert_eq!(context.handle_input(TestInput::Key(Key::Left)), false);
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Decay));
    assert_eq!(context.handle_input(TestInput::Key(Key::Down)), false);
    assert!(context.verify_parameter(Parameter::Attack));
    assert_eq!(context.handle_input(TestInput::Key(Key::Left)), false);
    assert_eq!(context.ps.state, SelectorState::FunctionIndex);
    assert!(context.verify_function_id(2));
    assert_eq!(context.handle_input(TestInput::Key(Key::Down)), false);
    assert!(context.verify_function_id(1));
    assert_eq!(context.handle_input(TestInput::Key(Key::Left)), false);
    assert_eq!(context.ps.state, SelectorState::Function);
    assert!(context.verify_function(Parameter::Envelope));
    assert_eq!(context.handle_input(TestInput::Key(Key::Down)), false);
    assert!(context.verify_function(Parameter::Oscillator));
}

#[test]
fn escape_resets_to_valid_state() {
    // 1. Function state
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Key(Key::Up), TestInput::Key(Key::Esc)];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));

    // 2. Function ID state
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o".to_string()), TestInput::Key(Key::Esc)];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));

    // 3. Parameter state
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1".to_string()), TestInput::Key(Key::Esc)];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));

    // 4. Value state
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1f".to_string()), TestInput::Key(Key::Esc)];
    assert_eq!(context.handle_inputs(c), false);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));
}

// -------------------
// Entering int values
// -------------------

#[test]
fn cursor_up_increments_int_value() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("o1v".to_string()));
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Voices, ParameterValue::Int(1)));
    assert_eq!(context.handle_input(TestInput::Key(Key::Up)), true);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Voices, ParameterValue::Int(2)));
}

#[test]
fn cursor_down_decrements_int_value() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1v".to_string()), TestInput::Key(Key::Up)];
    assert_eq!(context.handle_inputs(c), true);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Voices, ParameterValue::Int(2)));
    assert_eq!(context.handle_input(TestInput::Key(Key::Down)), true);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Voices, ParameterValue::Int(1)));
}

#[test]
fn clear_int_tempstring_between_values() {

    // 1. After completing a value
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("o1t23".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Tune, ParameterValue::Int(23)));
    context.handle_input(TestInput::Chars("t21".to_string()));
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Tune, ParameterValue::Int(21)));

    // 2. After cancelling input
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1t1".to_string()), TestInput::Key(Key::Backspace)];
    assert!(!context.handle_inputs(c));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Tune, ParameterValue::Int(1)));
    context.handle_input(TestInput::Chars("t23".to_string()));
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Tune, ParameterValue::Int(23)));
}

// ---------------------
// Entering float values
// ---------------------

#[test]
fn cursor_up_increments_float_value() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1l".to_string()), TestInput::Key(Key::Up)];
    assert_eq!(context.handle_inputs(c), true);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(DEFAULT_LEVEL + 1.0)));
}

#[test]
fn cursor_up_stops_at_max_for_floats() {
    let mut context = TestContext::new();
    // Set level to max
    assert_eq!(context.handle_input(TestInput::Chars("o1l100".to_string())), true);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(100.0)));

    // Enter value state again, try to increase level
    let c: &[TestInput] = &[TestInput::Key(Key::Esc), TestInput::Chars("o1l".to_string()), TestInput::Key(Key::Up)];
    assert_eq!(context.handle_inputs(c), true);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(100.0)));
}

#[test]
fn cursor_down_decrements_float_value() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("o1l".to_string()));
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(DEFAULT_LEVEL)));
    assert_eq!(context.handle_input(TestInput::Key(Key::Down)), true);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(DEFAULT_LEVEL - 1.0)));
}

#[test]
fn multi_digit_float_value() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("o1l12.3456\n".to_string()));
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(12.3456)));
}

#[test]
fn float_string_input() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o3l".to_string()),
                            TestInput::Chars("13.5\n".to_string())];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_selection(Parameter::Oscillator, 3, Parameter::Level, ParameterValue::Float(13.5)));

    let c: &[TestInput] = &[TestInput::Chars("l".to_string()),
                            TestInput::Chars("26.47".to_string()),
                            TestInput::Key(Key::Backspace),
                            TestInput::Chars("\n".to_string()),
    ];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_selection(Parameter::Oscillator, 3, Parameter::Level, ParameterValue::Float(26.4)));
}

// --------------------
// Parameters as values
// --------------------

#[test]
fn modulator_source_selection() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("m\nsl1".to_string()));
    let value = FunctionId{function: Parameter::Lfo, function_id: 1};
    assert!(context.verify_selection(Parameter::Modulation, 1, Parameter::Source, ParameterValue::Function(value)));
    assert_eq!(context.ps.state, SelectorState::Param);
}

#[test]
fn modulator_target_selection() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("m".to_string()),
                            TestInput::Key(Key::Right),
                            TestInput::Chars("te2a".to_string())];
    context.handle_inputs(c);
    let value = ParamId{function: Parameter::Envelope, function_id: 2, parameter: Parameter::Attack};
    assert!(context.verify_selection(Parameter::Modulation, 1, Parameter::Target, ParameterValue::Param(value)));
    assert_eq!(context.ps.state, SelectorState::Param);
}

#[test]
fn modulator_target_sel_with_cursor() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("m".to_string()),
                            TestInput::Key(Key::Right), // Mod 1
                            TestInput::Key(Key::Up), // Target
                            TestInput::Key(Key::Right), // Target
                            TestInput::Key(Key::Right), // Oscillator
                            TestInput::Key(Key::Right), // Oscillator 1
                            ];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::ValueParam);
}

#[test]
fn leave_subsel_with_cursor() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("m".to_string()),
                            TestInput::Key(Key::Right), // Mod 1
                            TestInput::Key(Key::Right), // Source
                            ];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::ValueFunction);
    context.handle_input(TestInput::Key(Key::Left)); // Back to parameter
    assert_eq!(context.ps.state, SelectorState::Param);
}

// ----------------
// Controller input
// ----------------

#[test]
fn controller_updates_selected_value() {
    let mut context = TestContext::new();
    let c: &[TestInput] = &[TestInput::Chars("o1l".to_string())];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::Value);
    context.handle_input(TestInput::ControlChange(1, 0));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0)));
    context.handle_input(TestInput::ControlChange(1, 127));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(100.0)));
}

#[test]
fn controller_is_ignored_in_other_states() {
    let mut context = TestContext::new();

    // Ignored in state_function
    context.handle_input(TestInput::ControlChange(1, 127));
    assert_eq!(context.ps.state, SelectorState::Function);

    // Ignored in state_function_id
    context.handle_input(TestInput::Chars("o".to_string()));
    context.handle_input(TestInput::ControlChange(1, 127));
    assert_eq!(context.ps.state, SelectorState::FunctionIndex);

    // In state_param, it switches to state_value
    context.handle_input(TestInput::Chars("1".to_string()));
    context.handle_input(TestInput::ControlChange(1, 127));
    assert_eq!(context.ps.state, SelectorState::Value);
}

// -------------
// Shortcut keys
// -------------

#[test]
fn right_bracket_in_state_param_selects_nextparameter() {
    let mut context = TestContext::new();

    context.handle_input(TestInput::Chars("e3".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Attack));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Decay));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Sustain));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Release));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Factor));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Delay));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Loop));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Loop));
}

#[test]
fn left_bracket_in_state_param_selects_prevparameter() {
    let mut context = TestContext::new();

    context.handle_input(TestInput::Chars("e3f\n".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Factor));

    context.handle_input(TestInput::Chars("[".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Release));

    context.handle_input(TestInput::Chars("[".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Sustain));

    context.handle_input(TestInput::Chars("[".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Decay));

    context.handle_input(TestInput::Chars("[".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Attack));

    context.handle_input(TestInput::Chars("[".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);
    assert!(context.verify_parameter(Parameter::Attack));
}

#[test]
fn right_bracket_in_state_value_selects_nextparameter() {
    let mut context = TestContext::new();

    context.handle_input(TestInput::Chars("e3a".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 3, Parameter::Attack, ParameterValue::Float(DEFAULT_ATTACK)));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 3, Parameter::Decay, ParameterValue::Float(DEFAULT_DECAY)));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 3, Parameter::Sustain, ParameterValue::Float(DEFAULT_SUSTAIN)));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 3, Parameter::Release, ParameterValue::Float(DEFAULT_RELEASE)));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 3, Parameter::Factor, ParameterValue::Int(DEFAULT_FACTOR)));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 3, Parameter::Delay, ParameterValue::Float(0.0)));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 3, Parameter::Loop, ParameterValue::Int(0)));

    context.handle_input(TestInput::Chars("]".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 3, Parameter::Loop, ParameterValue::Int(0)));
}

#[test]
fn left_bracket_in_state_value_selects_prevparameter() {
    let mut context = TestContext::new();

    context.handle_input(TestInput::Chars("e2f".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 2, Parameter::Factor, ParameterValue::Int(DEFAULT_FACTOR)));

    context.handle_input(TestInput::Chars("[".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 2, Parameter::Release, ParameterValue::Float(DEFAULT_RELEASE)));

    context.handle_input(TestInput::Chars("[".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 2, Parameter::Sustain, ParameterValue::Float(DEFAULT_SUSTAIN)));

    context.handle_input(TestInput::Chars("[".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 2, Parameter::Decay, ParameterValue::Float(DEFAULT_DECAY)));

    context.handle_input(TestInput::Chars("[".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 2, Parameter::Attack, ParameterValue::Float(DEFAULT_ATTACK)));

    context.handle_input(TestInput::Chars("[".to_string()));
    assert!(context.verify_selection(Parameter::Envelope, 2, Parameter::Attack, ParameterValue::Float(DEFAULT_ATTACK)));
}

#[test]
fn bracket_in_state_value_updates_value_state_to_match_value_type() {
    let mut context = TestContext::new();

    // From state_value to state_value_function
    context.handle_input(TestInput::Chars("m2a".to_string()));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_parameter(Parameter::Amount));
    context.handle_input(TestInput::Chars("[".to_string()));
    assert_eq!(context.ps.state, SelectorState::ValueFunction);
    assert!(context.verify_parameter(Parameter::Target));

    // From state_value_function to state_value
    context.handle_input(TestInput::Key(Key::Esc));
    context.handle_input(TestInput::Chars("m2t".to_string()));
    assert_eq!(context.ps.state, SelectorState::ValueFunction);
    assert!(context.verify_parameter(Parameter::Target));
    context.handle_input(TestInput::Chars("]".to_string()));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_parameter(Parameter::Amount));

    // From state_value_function_id to state_value
    context.handle_input(TestInput::Key(Key::Esc));
    context.handle_input(TestInput::Chars("m2to".to_string()));
    assert_eq!(context.ps.state, SelectorState::ValueFunctionIndex);
    assert!(context.verify_parameter(Parameter::Target));
    context.handle_input(TestInput::Chars("]".to_string()));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_parameter(Parameter::Amount));

    // From state_value_parameter to state_value
    context.handle_input(TestInput::Key(Key::Esc));
    context.handle_input(TestInput::Chars("m2to1".to_string()));
    assert_eq!(context.ps.state, SelectorState::ValueParam);
    assert!(context.verify_parameter(Parameter::Target));
    context.handle_input(TestInput::Chars("]".to_string()));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_parameter(Parameter::Amount));
}

#[test]
fn page_up_down_change_function_id() {
    let mut context = TestContext::new();

    // Select Oscillator 1 Finetune
    let c: &[TestInput] = &[TestInput::Chars("o1f".to_string())];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Finetune, ParameterValue::Float(0.0)));

    // Enter PageUp
    context.handle_input(TestInput::Key(Key::PageUp));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 2, Parameter::Finetune, ParameterValue::Float(0.0)));

    // Enter PageUp again
    context.handle_input(TestInput::Key(Key::PageUp));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 3, Parameter::Finetune, ParameterValue::Float(0.0)));

    // Enter PageDown
    context.handle_input(TestInput::Key(Key::PageDown));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 2, Parameter::Finetune, ParameterValue::Float(0.0)));
}

#[test]
fn history_back_and_forward() {
    let mut context = TestContext::new();

    // Push 3 items in the history
    context.handle_input(TestInput::Chars("o1l".to_string()));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(DEFAULT_LEVEL)));

    let c: &[TestInput] = &[TestInput::Key(Key::Esc), TestInput::Chars("e2d".to_string())];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Envelope, 2, Parameter::Decay, ParameterValue::Float(DEFAULT_DECAY)));

    let c: &[TestInput] = &[TestInput::Key(Key::Esc), TestInput::Chars("m12s".to_string())];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::ValueFunction);
    let value = FunctionId{function: Parameter::Lfo, function_id: 1};
    assert!(context.verify_selection(Parameter::Modulation, 12, Parameter::Source, ParameterValue::Function(value)));

    // Walk backwards through history
    context.handle_input(TestInput::Chars("<".to_string()));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Envelope, 2, Parameter::Decay, ParameterValue::Float(DEFAULT_DECAY)));

    context.handle_input(TestInput::Chars("<".to_string()));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(DEFAULT_LEVEL)));

    // Walk forwards through history
    context.handle_input(TestInput::Chars(">".to_string()));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Envelope, 2, Parameter::Decay, ParameterValue::Float(DEFAULT_DECAY)));

    context.handle_input(TestInput::Chars(">".to_string()));
    assert_eq!(context.ps.state, SelectorState::ValueFunction);
    let value = FunctionId{function: Parameter::Lfo, function_id: 1};
    assert!(context.verify_selection(Parameter::Modulation, 12, Parameter::Source, ParameterValue::Function(value)));
}

#[test]
fn set_and_recall_marker() {
    let mut context = TestContext::new();

    context.handle_input(TestInput::Chars("o1l".to_string()));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(DEFAULT_LEVEL)));

    // Set marker a
    context.handle_input(TestInput::Chars("\"a".to_string()));

    // Go somewhere else
    let c: &[TestInput] = &[TestInput::Key(Key::Esc), TestInput::Chars("e2d".to_string())];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Envelope, 2, Parameter::Decay, ParameterValue::Float(DEFAULT_DECAY)));

    // Return to marker a
    context.handle_input(TestInput::Chars("\'a".to_string()));
    assert_eq!(context.ps.state, SelectorState::Value);
    assert!(context.verify_selection(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(DEFAULT_LEVEL)));
}

#[test]
fn going_from_long_to_shorter_function_index_by_cursor() {
    let mut context = TestContext::new();
    context.handle_input(TestInput::Chars("e3".to_string()));
    assert_eq!(context.ps.state, SelectorState::Param);

    let c: &[TestInput] = &[TestInput::Key(Key::Left),  // FunctionId
                            TestInput::Key(Key::Left),  // Function
                            TestInput::Key(Key::Up),    // LFO
                            TestInput::Key(Key::Right), // FunctionId
                            ];
    context.handle_inputs(c);
    assert_eq!(context.ps.state, SelectorState::FunctionIndex);
}

// TODO:
// - Delete controller assignment in MIDI learn mode

} // mod tests
