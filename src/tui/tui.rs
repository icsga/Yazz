use super::{CtrlMap, MappingType};
use super::{Parameter, ParameterValue, ParamId, SynthParam, ValueRange, FUNCTIONS, MOD_SOURCES};
use super::Float;
use super::MidiMessage;
use super::{SelectorEvent, SelectorState, ParamSelector, next, ItemSelection};
use super::{SoundBank, SoundPatch};
use super::{UiMessage, SynthMessage};
use super::surface::Surface;
use super::Value;
use super::{SOUND_DATA_VERSION, SYNTH_ENGINE_VERSION};
use super::StateMachine;
use super::WtInfo;

use crossbeam_channel::{Sender, Receiver};
use log::info;
use termion::{clear, color, cursor};
use termion::color::{Black, LightWhite, Rgb};
use termion::event::Key;

extern crate regex;
use regex::Regex;

use std::convert::TryInto;
use std::fs;
use std::io::{stdout, Write};
use std::path::Path;
use std::thread::spawn;
use std::time::Duration;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
enum Mode {
    Play,
    Edit
}

enum TuiState {
    Play,
    Help,
}

//type TuiEvent = termion::event::Key;

pub struct Tui {
    // Function selection
    sender: Sender<SynthMessage>,
    ui_receiver: Receiver<UiMessage>,

    // TUI handling
    selector: ParamSelector,
    selection_changed: bool,

    // Actual UI
    window: Surface,
    sync_counter: u32,
    idle: Duration, // Accumulated idle times of the engine
    busy: Duration, // Accumulated busy times of the engine
    min_idle: Duration,
    max_busy: Duration,
    show_tui: bool,

    bank: SoundBank,   // Bank with sound patches
    sound: Rc<RefCell<SoundPatch>>, // Current sound patch as loaded from disk
    selected_sound: usize,
    ctrl_map: CtrlMap, // Mapping of MIDI controller to parameter
    active_ctrl_set: usize,

    // State machine for ParamSelector
    selector_sm: StateMachine<ParamSelector, SelectorEvent>,
    mode: Mode,
    state: TuiState,
}

impl Tui {
    pub fn new(sender: Sender<SynthMessage>, ui_receiver: Receiver<UiMessage>, show_tui: bool) -> Tui {
        let mut window = Surface::new();
        let sound = Rc::new(RefCell::new(SoundPatch::new()));
        window.set_position(1, 3);
        window.update_all(&sound.borrow().data);

        let mut tui = Tui{
            sender,
            ui_receiver,
            selector: ParamSelector::new(&FUNCTIONS, &MOD_SOURCES),
            selection_changed: true,
            window,
            sync_counter: 0,
            idle: Duration::new(0, 0),
            busy: Duration::new(0, 0),
            min_idle: Duration::new(10, 0),
            max_busy: Duration::new(0, 0),
            show_tui,
            bank: SoundBank::new(SOUND_DATA_VERSION, SYNTH_ENGINE_VERSION),
            sound,
            selected_sound: 0,
            ctrl_map: CtrlMap::new(),
            active_ctrl_set: 0,
            selector_sm: StateMachine::new(ParamSelector::state_function),
            mode: Mode::Edit,
            state: TuiState::Play,
        };
        tui.bank.load_bank("Yazz_FactoryBank.ysn").unwrap();
        tui.load_wavetables();
        tui.scan_wavetables();
        tui.select_sound(0);
        tui.selector_sm.init(&mut tui.selector);
        match tui.ctrl_map.load("Yazz_ControllerMapping.ysn") {
            _ => ()
        }
        tui
    }

    fn load_wavetables(&mut self) {
        // send default wavetables to synth (already in list)
        //self.sender.send(SynthMessage::Wavetable(WtInfo{id: 0, valid: true, name: "Basic".to_string(), filename: "".to_string()})).unwrap();
        //self.sender.send(SynthMessage::Wavetable(WtInfo{id: 1, valid: true, name: "PWM Square".to_string(), filename: "".to_string()})).unwrap();

        // For all wavetable list entries:
        let list = self.selector.get_dynamic_list(Parameter::Wavetable);
        for entry in &mut self.bank.wt_list {
            // Check if entry is already in list
            let mut found_entry = false;
            for (id, name) in list.iter() {
                if *id == entry.id && *name == entry.name {
                    found_entry = true;
                }
            }
            if found_entry {
                continue;
            }
            // Check if file exists
            let filename = "data/".to_string() + &entry.filename;
            if !Path::new(&filename).exists() {
                entry.valid = false; // Invalid => Won't show up in menu, sounds get default wavetable
            }
            // Send struct to synth
            self.sender.send(SynthMessage::Wavetable(entry.clone())).unwrap();
            // Add to selector for option display
            list.push((entry.id, entry.name.clone()));
        }
        // Send list to synth

    }

    /** Start input handling thread.
     *
     * This thread receives messages from the terminal, the MIDI port, the
     * synth engine and the audio engine.
     */
    pub fn run(to_synth_sender: Sender<SynthMessage>,
               ui_receiver: Receiver<UiMessage>,
               show_tui: bool) -> std::thread::JoinHandle<()> {
        let handler = spawn(move || {
            let mut tui = Tui::new(to_synth_sender, ui_receiver, show_tui);
            loop {
                let msg = tui.ui_receiver.recv().unwrap();
                if !tui.handle_ui_message(msg) {
                    break;
                }
            }
        });
        handler
    }

    fn handle_ui_message(&mut self, msg: UiMessage) -> bool {
        match msg {
            UiMessage::Midi(m)  => self.handle_midi_event(&m),
            UiMessage::Key(m) => self.handle_key_input(m),
            UiMessage::MousePress{x: _, y: _}
            | UiMessage::MouseHold{x: _, y: _}
            | UiMessage::MouseRelease{x: _, y: _} => self.window.handle_event(&msg),
            UiMessage::SampleBuffer(m, p) => self.handle_samplebuffer(m, p),
            UiMessage::EngineSync(idle, busy) => {
                self.update_idle_time(idle, busy);
                self.handle_engine_sync();
            }
            UiMessage::Exit => {
                info!("Stopping TUI");
                self.sender.send(SynthMessage::Exit).unwrap();
                return false;
            }
        };
        true
    }

    fn handle_key_input(&mut self, key: Key) {
        // Top-level keys that work in both modes
        if !match key {
            Key::F(1) => {
                self.display_help();
                true
            }
            Key::F(2) => {
                // Copy current sound to selected sound in bank
                self.bank.set_sound(self.selected_sound, &self.sound.borrow());
                // Write bank to disk
                self.bank.save_bank("Yazz_FactoryBank.ysn").unwrap();
                true
            },
            Key::F(3) => {
                // Read bank from disk
                self.bank.load_bank("Yazz_FactoryBank.ysn").unwrap();
                self.select_sound(0);
                true
            },
            Key::F(10) => {
                // Scan data folder for new wavetable files
                self.scan_wavetables();
                true
            },
            Key::Char(c) => {
                match c {
                    '\t' => {
                        self.toggle_mode();
                        true
                    }
                    _ => false // Key not handled yet
                }
            },
            _ => false
        } {
            // Mode-specific key handling
            if let Mode::Edit = self.mode {
                self.state = TuiState::Play; // Exit help state (no need to check, just overwrite it)
                let last_selector_state = self.selector.state;
                if self.selector.handle_user_input(&mut self.selector_sm, key, self.sound.clone()) {
                    loop {
                        let p = self.selector.get_changed_value();
                        match p {
                            Some(param) => self.send_parameter(&param),
                            None => break,
                        }
                    }
                    self.send_event();
                }

                if last_selector_state == SelectorState::MidiLearn {
                    // Check if user has aborted MIDI learn state
                    self.check_midi_learn_status();
                }
            } else {
                self.handle_play_mode_input(key);
            }
        }
        self.selection_changed = true; // Trigger full UI redraw
    }

    fn toggle_mode(&mut self) {
        match self.mode {
            Mode::Play => self.mode = Mode::Edit,
            Mode::Edit => self.mode = Mode::Play,
        }
    }

    fn handle_play_mode_input(&mut self, key: Key) {
        self.state = match self.state {
            TuiState::Play => self.state_play(key),
            TuiState::Help => self.state_help(key),
        }
    }

    fn state_play(&mut self, key: Key) -> TuiState {
        match key {
            Key::Char(c) => {
                match c {
                    '0' ..= '9' => self.active_ctrl_set = c as usize - '0' as usize,
                    'a' ..= 'z' => self.active_ctrl_set = c as usize - 'a' as usize + 10,
                    '+' => self.select_sound(self.selected_sound + 1),
                    '-' => self.select_sound(self.selected_sound - 1),
                    _ => ()
                }
            }
            _ => ()
        }
        TuiState::Play
    }

    fn state_help(&mut self, _key: Key) -> TuiState {
        TuiState::Play
    }

    fn scan_wavetables(&mut self) {
        let re = Regex::new(r"(.*).wav").unwrap();
        if !Path::new("data").exists() {
            // Create data directory
            let result = fs::create_dir("data");
            match result {
                Ok(()) => info!("Created data directory"),
                Err(err) => info!("Error, can't create data directory: {}", err),
            }
            return;
        }
        for entry in fs::read_dir("data").unwrap() {
            let entry = entry.unwrap();
            let filename = entry.file_name();
            for cap in re.captures_iter(filename.to_str().unwrap()) {
                let table_name = &cap[1];
                let mut found = false;
                for wti in &self.bank.wt_list {
                    if wti.name == table_name {
                        info!("{} already in wavetable list, skipping.", table_name);
                        found = true;
                        break;
                    }
                }
                if !found {
                    info!("Adding new table {}.", table_name);
                    let id = self.bank.wt_list.len() + 2; // Default wavetables are not in this list, so add 2
                    let new_entry = WtInfo{
                        id: id,
                        valid: true,
                        name: table_name.to_string(),
                        filename: filename.to_str().unwrap().to_string()};
                    self.sender.send(SynthMessage::Wavetable(new_entry.clone())).unwrap();
                    self.bank.wt_list.push(new_entry);
                    self.selector.wavetable_list.push((id, table_name.to_string()));
                }
            }
        }
    }

    /** Select a sound from the loaded sound bank.
     *
     * Creates a local copy of the selected sound, which can be modified. It
     * is copied back into the sound bank when saving the sound. Changing the
     * sound again before saving discards any changes.
     */
    fn select_sound(&mut self, mut sound_index: usize) {
        if sound_index == 128 {
            sound_index = 0;
        } else if sound_index > 127 {
            sound_index = 127; // Overflow, went below zero
        }
        self.selected_sound = sound_index;
        let sound_ref: &SoundPatch = self.bank.get_sound(self.selected_sound);
        let sound = &mut self.sound.borrow_mut();
        sound.name = sound_ref.name.clone();
        sound.data = sound_ref.data;

        // Send new sound to synth engine
        let sound_copy = sound.data;
        self.sender.send(SynthMessage::Sound(sound_copy)).unwrap();

        // Update display
        self.window.set_sound_info(self.selected_sound, &sound.name);
        self.window.update_all(&sound.data);
    }

    /* MIDI message received */
    fn handle_midi_event(&mut self, m: &MidiMessage) {
        match *m {
            MidiMessage::ControlChg{channel: _, controller, value} => {
                if self.selector.state == SelectorState::MidiLearn {
                    // MIDI learn: Send all controller events to the selector
                    self.selector.handle_control_input(&mut self.selector_sm, controller.into(), value.into(), self.sound.clone());
                    self.check_midi_learn_status();
                    return;
                }

                // Special handling of some controllers
                match controller {
                    0x01  => {
                        // ModWheel
                        let edit_mode = if let Mode::Edit = self.mode { true } else { false };
                        if edit_mode {
                            // ModWheel is used as general data entry for Selector in Edit mode.
                            if self.selector.handle_control_input(&mut self.selector_sm,
                                                                  controller.into(),
                                                                  value.into(),
                                                                  self.sound.clone()) {
                                self.send_event(); // Sound parameter has been changed
                            }
                        } else {
                            // In play mode, modwheel is both a global mod source and a
                            // controller. Send message to synth engine about mod
                            // source update.
                            self.sender.send(SynthMessage::Midi(*m)).unwrap();
                        }
                    }
                    0x40 => {
                        // Sustain pedal is always sent to synth, in addition to
                        // controller mappings below
                        self.sender.send(SynthMessage::Midi(*m)).unwrap();
                    }
                    _ => (),
                }

                // All controllers (including ModWheel and sustain pedal) might
                // be mapped to control a parameter directly
                self.handle_ctrl_change(controller.into(), value.into());

            },
            MidiMessage::ProgramChg{channel: _, program} => self.select_sound(program as usize - 1),
            _ => ()
        }
    }

    // Check if MIDI learn has been completed.
    //
    // This can be triggered either by having received enough controller data,
    // or by being cancelled via keyboard (backspace key).
    fn check_midi_learn_status(&mut self) {
        // Check if complete, if yes set CtrlMap
        if self.selector.ml.complete {
            let val_range = self.selector.get_value_range();
            let param = self.selector.get_param_id();
            let ml = &mut self.selector.ml;
            match ml.mapping_type {
                MappingType::Relative | MappingType::Absolute => {
                    self.ctrl_map.add_mapping(self.active_ctrl_set,
                                              ml.ctrl,
                                              ml.mapping_type,
                                              param,
                                              val_range);
                }
                MappingType::None => {
                    self.ctrl_map.delete_mapping(self.active_ctrl_set,
                                                 param);
                }
            }
            self.ctrl_map.save("Yazz_ControllerMapping.ysn").unwrap();
        }
    }

    fn handle_ctrl_change(&mut self, controller: u64, value: u64) {
        let result = self.ctrl_map.get_value(self.active_ctrl_set, controller, value, &self.sound.borrow().data);
        if let Ok(synth_param) = result {
            self.send_parameter(&synth_param);
        }
    }

    /* Received a buffer with samples from the synth engine. */
    fn handle_samplebuffer(&mut self, m: Vec<Float>, p: SynthParam) {
        let canvas = &mut self.window.canvas.borrow_mut();
        canvas.clear();
        match p.function {
            Parameter::Oscillator | Parameter::Lfo | Parameter::GlobalLfo => {
                canvas.plot(&m, -1.0, 1.0);
            }
            Parameter::Envelope => {
                canvas.plot(&m, 0.0, 1.0);
            }
            _ => ()
        }
    }

    /* Update idle time based on timings received from the synth egine. */
    fn update_idle_time(&mut self, idle: Duration, busy: Duration) {
        self.idle += idle;
        self.busy += busy;
        if idle < self.min_idle {
            self.min_idle = idle;
        }
        if busy > self.max_busy {
            self.max_busy = busy;
        }
    }

    /* Received a sync signal from the audio engine.
     *
     * This is used to control timing related actions like drawing the display.
     */
    fn handle_engine_sync(&mut self) {
        self.sync_counter += 1;
        if self.sync_counter == 20 {
            self.display();

            self.sync_counter = 0;
            self.query_samplebuffer();
        }
    }

    /* Send an updated value to the synth engine. */
    fn send_event(&mut self) {
        // Update sound data
        let param = self.selector.get_synth_param();
        self.send_parameter(&param);
    }

    /* Update all places this parameter is used. */
    fn send_parameter(&mut self, param: &SynthParam) {
        // Update local copy of the sound
        self.sound.borrow_mut().data.set_parameter(&param);

        // Send new value to synth engine
        self.sender.send(SynthMessage::Param(param.clone())).unwrap();

        // If the changed value is currently selected in the command line,
        // send it the updated value too.
        self.selector.value_has_changed(&mut self.selector_sm, param.clone());

        // Update UI
        let param_id = ParamId::new_from(param);
        let value = match param.value {
            ParameterValue::Float(v) => Value::Float(v.into()),
            ParameterValue::Int(v) => Value::Int(v),
            ParameterValue::Choice(v) => Value::Int(v.try_into().unwrap()),
            ParameterValue::Dynamic(_, v) => Value::Int(v.try_into().unwrap()), // TODO: Display string, not int
            _ => return
        };
        self.window.update_value(&param_id, value);
    }

    /* Queries a samplebuffer from the synth engine to display.
     *
     * The samplebuffer can contain wave shapes or envelopes.
     */
    fn query_samplebuffer(&self) {
        let buffer = vec!(0.0; 100);
        let param = self.selector.get_synth_param();
        self.sender.send(SynthMessage::SampleBuffer(buffer, param)).unwrap();
    }

    /* ====================================================================== */

    /** Display the UI. */
    fn display(&mut self) {
        if let TuiState::Help = self.state {
            return;
        }

        if self.selection_changed {
            print!("{}", clear::All);
            self.selection_changed = false;
            self.window.set_dirty(true);
        }

        if self.show_tui {
            self.window.draw();
        }

        if let Mode::Edit = self.mode {
            print!("{}{}", cursor::Goto(1, 1), clear::CurrentLine);
            Tui::display_selector(&self.selector);
        }
        if self.show_tui {
            self.display_idle_time();
            self.display_status_line();
        }

        stdout().flush().ok();
    }

    fn display_selector(s: &ParamSelector) {
        let selector_state = s.state;
        let mut display_state = SelectorState::Function;
        let mut x_pos: u16 = 1;
        let mut selection = &s.func_selection;
        loop {
            match display_state {
                SelectorState::Function => {
                    Tui::display_function(&s.func_selection, selector_state == SelectorState::Function);
                }
                SelectorState::FunctionIndex => {
                    Tui::display_function_index(&s.func_selection, selector_state == SelectorState::FunctionIndex);
                    x_pos = 12;
                }
                SelectorState::Param => {
                    Tui::display_param(&s.param_selection, selector_state == SelectorState::Param);
                    selection = &s.param_selection;
                    x_pos = 14;
                    if selector_state == SelectorState::Param {
                        // Display value after parameter
                        match s.param_selection.value {
                            ParameterValue::Int(_)
                            | ParameterValue::Float(_)
                            | ParameterValue::Choice(_)
                            | ParameterValue::Dynamic(_, _) => {
                                Tui::display_value(&s.param_selection, false, &s.wavetable_list);
                            },
                            ParameterValue::Function(_) => {
                                Tui::display_function(&s.value_func_selection, false);
                                Tui::display_function_index(&s.value_func_selection, false);
                            },
                            ParameterValue::Param(_) => {
                                Tui::display_function(&s.value_func_selection, false);
                                Tui::display_function_index(&s.value_func_selection, false);
                                Tui::display_param(&s.value_param_selection, false);
                            }
                            _ => ()
                        }
                    }
                }
                SelectorState::Value => {
                    Tui::display_value(&s.param_selection, selector_state == SelectorState::Value, &s.wavetable_list);
                    x_pos = 23;
                }
                SelectorState::MidiLearn => {
                    if selector_state == SelectorState::MidiLearn {
                        Tui::display_midi_learn();
                    }
                }
                SelectorState::AddMarker => {
                    if selector_state == SelectorState::AddMarker {
                        Tui::display_add_marker();
                    }
                }
                SelectorState::GotoMarker => {
                    if selector_state == SelectorState::GotoMarker {
                        Tui::display_goto_marker();
                    }
                }
                SelectorState::ValueFunction => {
                    Tui::display_function(&s.value_func_selection, selector_state == SelectorState::ValueFunction);
                    selection = &s.value_func_selection;
                    x_pos = 30;
                }
                SelectorState::ValueFunctionIndex => {
                    Tui::display_function_index(&s.value_func_selection, selector_state == SelectorState::ValueFunctionIndex);
                    x_pos = 38;
                }
                SelectorState::ValueParam => {
                    Tui::display_param(&s.value_param_selection, selector_state == SelectorState::ValueParam);
                    selection = &s.value_param_selection;
                    x_pos = 46;
                }
            }
            if display_state == selector_state {
                break;
            }
            display_state = next(display_state);
        }
        Tui::display_options(s, selection, x_pos);
    }

    fn display_function(func: &ItemSelection, selected: bool) {
        if selected {
            print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        } else {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
        print!("{}", func.item_list[func.item_index].item);
        if selected {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_function_index(func: &ItemSelection, selected: bool) {
        if selected {
            print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        }
        let function_id = if let ParameterValue::Int(x) = &func.value { *x as usize } else { panic!() };
        print!(" {}", function_id);
        if selected {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_param(param: &ItemSelection, selected: bool) {
        if selected {
            print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        }
        print!(" {} ", param.item_list[param.item_index].item);
        if selected {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_value(param: &ItemSelection, selected: bool, wt_list: &Vec<(usize, String)>) {
        if selected {
            print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        }
        match param.value {
            ParameterValue::Int(x) => print!(" {}", x),
            ParameterValue::Float(x) => print!(" {}", x),
            ParameterValue::Choice(x) => {
                let item = &param.item_list[param.item_index];
                let range = &item.val_range;
                let selection = if let ValueRange::Choice(list) = range { list } else { panic!("item: {:?}, range: {:?}", item, range) };
                let item = selection[x].item;
                print!(" {}", item);
            },
            ParameterValue::Dynamic(_, x) => {
                for (k, v) in wt_list {
                    if *k == x {
                        print!(" {}", v);
                        break;
                    }
                }
            },
            _ => ()
        }
        if selected {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_midi_learn() {
        print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        print!("  MIDI Learn: Send controller data");
        print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
    }

    fn display_add_marker() {
        print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        print!("  Select marker to add");
        print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
    }

    fn display_goto_marker() {
        print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        print!("  Select marker to go to");
        print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
    }

    fn display_options(selector: &ParamSelector, s: &ItemSelection, x_pos: u16) {
        print!("{}{}", color::Bg(Black), color::Fg(LightWhite));
        let selector_state = selector.state;
        if selector_state == SelectorState::Function || selector_state == SelectorState::ValueFunction
        || selector_state == SelectorState::Param || selector_state == SelectorState::ValueParam {
            let mut y_item = 2;
            let list = s.item_list;
            for item in list.iter() {
                print!("{} {} - {} ", cursor::Goto(x_pos, y_item), item.key, item.item);
                y_item += 1;
            }
        }
        if selector_state == SelectorState::FunctionIndex || selector_state == SelectorState::ValueFunctionIndex {
            let item = &s.item_list[s.item_index];
            let (min, max) = if let ValueRange::Int(min, max) = item.val_range { (min, max) } else { panic!() };
            print!("{} {} - {} ", cursor::Goto(x_pos, 2), min, max);
        }
        if selector_state == SelectorState::Value {
            let range = &s.item_list[s.item_index].val_range;
            match range {
                ValueRange::Int(min, max) => print!("{} {} - {} ", cursor::Goto(x_pos, 2), min, max),
                ValueRange::Float(min, max, _) => print!("{} {} - {} ", cursor::Goto(x_pos, 2), min, max),
                ValueRange::Choice(list) => print!("{} 1 - {} ", cursor::Goto(x_pos, 2), list.len()),
                ValueRange::Dynamic(param) => Tui::display_dynamic_options(selector, *param, x_pos),
                ValueRange::Func(_) => (),
                ValueRange::Param(_) => (),
                ValueRange::NoRange => ()
            }
        }
        print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
    }

    fn display_dynamic_options(s: &ParamSelector, param: Parameter, x_pos: u16) {
        let list = s.get_dynamic_list_no_mut(param);
        let mut y_item = 2;
        for (key, value) in list.iter() {
            print!("{} {} - {} ", cursor::Goto(x_pos, y_item), key, value);
            y_item += 1;
        }
    }

    fn display_idle_time(&mut self) {
        let idle = self.idle / self.sync_counter;
        let busy = self.busy / self.sync_counter;
        let value = Value::Int(idle.as_micros() as i64);
        let key = ParamId{function: Parameter::System, function_id: 0, parameter: Parameter::Idle};
        self.window.update_value(&key, value);

        let value = Value::Int(busy.as_micros() as i64);
        let key = ParamId{function: Parameter::System, function_id: 0, parameter: Parameter::Busy};
        self.window.update_value(&key, value);

        self.idle = Duration::new(0, 0);
        self.busy = Duration::new(0, 0);
    }

    fn display_status_line(&mut self) {
        let ctrl_set = self.active_ctrl_set as u8;
        let ctrl_set = (ctrl_set + if ctrl_set <= 9 { '0' as u8 } else { 'a' as u8 - 10 }) as char;
        print!("{}| Mode: {:?} | Active controller set: {} |                               Press <F1> for help ",
            cursor::Goto(1, 50),
            self.mode,
            ctrl_set);
    }

    fn display_help(&mut self) {
        self.state = TuiState::Help;
        print!("{}{}", clear::All, cursor::Goto(1, 1));
        println!("Global keys:\r");
        println!("------------\r");
        println!("<TAB> : Switch between Edit and Play mode\r");
        println!("<F1>  : Show this help text\r");
        println!("<F2>  : Save current sound bank\r");
        println!("<F3>  : Load current sound bank\r");
        println!("\r");
        println!("Keys in Edit mode:\r");
        println!("------------------\r");
        println!("</ >         : Move backwards/ forwards in the parameter history\r");
        println!("PgUp/ PgDown : Increase/ decrease function ID of current parameter\r");
        println!("[/ ]         : Move down/ up through the parameters of the current function\r");
        println!("\"<MarkerID>  : Set a marker with the MarkerID at the current parameter\r");
        println!("\'<MarkerID>  : Recall the parameter with the given MarkerID\r");
        println!("\r");
        println!("Keys in Play mode:\r");
        println!("------------------\r");
        println!("+/ -         : Select next/ previous patch\r");
        println!("0 - 9, a - z : Select MIDI controller assignment set\r");
        println!("\r");
        println!("Press any key to continue.\r");
        stdout().flush().ok();
    }
}
