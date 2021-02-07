use super::{ColorScheme, Printer, StdioPrinter};
use super::{CtrlMap, MappingType};
use super::Display;
use super::{Parameter, ParameterValue, ParamId, SynthParam, FUNCTIONS, MOD_SOURCES};
use super::Float;
use super::MidiMessage;
use super::{SelectorEvent, SelectorState, ParamSelector};
use super::{SoundBank, SoundPatch};
use super::StateMachine;
use super::TermionWrapper;
use super::{UiMessage, SynthMessage};
use super::surface::Surface;
use super::Value;
use super::WtInfo;
use super::{SOUND_DATA_VERSION, SYNTH_ENGINE_VERSION};

use crossbeam_channel::{Sender, Receiver};
use log::info;
use termion::{clear, cursor};
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
    Name, // Enter sound patch name
}

pub struct Tui {
    // Function selection
    sender: Sender<SynthMessage>,
    ui_receiver: Receiver<UiMessage>,

    // TUI handling
    selector: ParamSelector,
    selection_changed: bool,

    // Actual UI
    display: Display,
    window: Surface,
    sync_counter: u32,
    idle: Duration, // Accumulated idle times of the engine
    busy: Duration, // Accumulated busy times of the engine
    min_idle: Duration,
    max_busy: Duration,
    show_tui: bool,
    printer: StdioPrinter,
    colors: Vec<Rc<ColorScheme>>,
    current_color: Rc<ColorScheme>,
    color_index: usize,

    bank: SoundBank,                // Bank with sound patches
    sound: Rc<RefCell<SoundPatch>>, // Current sound patch as loaded from disk
    sound_copy: SoundPatch,         // For copying sounds
    selected_sound: usize,
    ctrl_map: CtrlMap,              // Mapping of MIDI controller to parameter
    active_ctrl_set: usize,
    temp_name: String,
    last_value: SynthParam,         // Copy of the last value set via controller

    // State machine for ParamSelector
    selector_sm: StateMachine<ParamSelector, SelectorEvent>,
    mode: Mode,
    state: TuiState,
}

impl Tui {
    pub fn new(sender: Sender<SynthMessage>,
               ui_receiver: Receiver<UiMessage>,
               show_tui: bool,
               termion: TermionWrapper) -> Tui {
        let mut colors = vec![Rc::new(ColorScheme::new())];
        colors.push(Rc::new(ColorScheme::dark()));
        colors.push(Rc::new(ColorScheme::amber()));
        let color_index = 0;
        let current_color = colors[color_index].clone();
        let display = Display::new(current_color.clone(), termion);
        let mut window = Surface::new(current_color.clone());
        let sound = Rc::new(RefCell::new(SoundPatch::new()));
        window.set_position(1, 3);
        window.update_all(&sound.borrow().data);

        let mut tui = Tui{
            sender,
            ui_receiver,
            selector: ParamSelector::new(&FUNCTIONS, &MOD_SOURCES),
            selection_changed: true,
            display,
            window,
            sync_counter: 0,
            idle: Duration::new(0, 0),
            busy: Duration::new(0, 0),
            min_idle: Duration::new(10, 0),
            max_busy: Duration::new(0, 0),
            show_tui,
            printer: StdioPrinter::new(),
            colors,
            current_color,
            color_index,
            bank: SoundBank::new(SOUND_DATA_VERSION, SYNTH_ENGINE_VERSION),
            sound,
            sound_copy: SoundPatch::new(),
            selected_sound: 0,
            ctrl_map: CtrlMap::new(),
            active_ctrl_set: 0,
            temp_name: "".to_string(),
            last_value: SynthParam{..Default::default()},
            selector_sm: StateMachine::new(ParamSelector::state_function),
            mode: Mode::Edit,
            state: TuiState::Play,
        };
        tui.bank.load_bank("Yazz_FactoryBank.ysn").unwrap();
        tui.load_wavetables();
        tui.scan_wavetables();
        tui.select_sound(0);
        tui.selector_sm.init(&mut tui.selector);
        tui.ctrl_map.load("Yazz_ControllerMapping.ysn").unwrap();
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
               show_tui: bool,
               termion: TermionWrapper) -> std::thread::JoinHandle<()> {
        spawn(move || {
            let mut tui = Tui::new(to_synth_sender, ui_receiver, show_tui, termion);
            loop {
                let msg = tui.ui_receiver.recv().unwrap();
                if !tui.handle_ui_message(msg) {
                    break;
                }
            }
        })
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
                self.state = TuiState::Help;
                self.display.display_help(&mut self.printer);
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
            Key::F(7) => {
                // Cycle through color schemes
                self.color_index += 1;
                if self.color_index >= self.colors.len() {
                    self.color_index = 0;
                }
                self.current_color = self.colors[self.color_index].clone();
                self.display.set_color_scheme(self.current_color.clone());
                self.window.set_color_scheme(self.current_color.clone());
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
            Key::Ctrl(c) => {
                match c {
                    'c' => { // Copy sound
                        self.sound_copy = self.sound.borrow().clone();
                        true
                    }
                    'v' => { // Paste sound
                        self.bank.set_sound(self.selected_sound, &self.sound_copy);
                        self.select_sound(self.selected_sound);
                        true
                    }
                    'n' => { // Rename sound
                        self.state = TuiState::Name;
                        self.temp_name.clear();
                        self.temp_name.push_str(&self.sound.borrow().name);
                        self.display_name_prompt();
                        true
                    }
                    _ => false
                }
            }
            _ => false
        } {
            // Mode-specific key handling
            if let Mode::Edit = self.mode {
                self.handle_edit_mode_input(key);
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

    fn handle_edit_mode_input(&mut self, key: Key) {
        match self.state {
            TuiState::Help => self.state = TuiState::Play,
            TuiState::Name => self.state = self.state_name(key),
            TuiState::Play => {
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
            }
        }
    }

    fn handle_play_mode_input(&mut self, key: Key) {
        self.state = match self.state {
            TuiState::Play => self.state_play(key),
            TuiState::Help => self.state_help(key),
            TuiState::Name => self.state_name(key),
        }
    }

    fn state_play(&mut self, key: Key) -> TuiState {
        if let Key::Char(c) = key {
            match c {
                '0' ..= '9' => self.active_ctrl_set = c as usize - '0' as usize,
                'a' ..= 'z' => self.active_ctrl_set = c as usize - 'a' as usize + 10,
                '+' => self.select_sound(self.selected_sound + 1),
                '-' => self.select_sound(self.selected_sound - 1),
                _ => ()
            }
        }
        TuiState::Play
    }

    fn state_help(&mut self, _key: Key) -> TuiState {
        TuiState::Play
    }

    fn state_name(&mut self, key: Key) -> TuiState {
        let next_state = match key {
            Key::Char(c) => {
                match c {
                    '0'..='9' | 'a'..='z' | 'A'..='Z' | ' ' => {
                        self.temp_name.push(c);
                        TuiState::Name
                    }
                    '\n' => {
                        self.sound.borrow_mut().name = self.temp_name.clone();
                        //self.bank.set_sound(self.selected_sound, &self.sound.borrow());
                        //self.select_sound(self.selected_sound);
                        self.window.set_sound_info(self.selected_sound, &self.sound.borrow().name);
                        TuiState::Play
                    }
                    _ => TuiState::Name,
                }
            }
            Key::Backspace => {
                if !self.temp_name.is_empty() {
                    self.temp_name.pop();
                }
                TuiState::Name
            }
            Key::Esc => TuiState::Play,
            _ => TuiState::Name,
        };
        if let TuiState::Name = next_state {
            self.display_name_prompt();
        }
        next_state
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
                        id,
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
        let sound_copy = Box::new(sound.data);
        self.sender.send(SynthMessage::Sound(sound_copy)).unwrap();

        // Update display
        self.window.set_sound_info(self.selected_sound, &sound.name);
        self.window.update_all(&sound.data);
    }

    /* MIDI message received */
    fn handle_midi_event(&mut self, m: &MidiMessage) {
        match *m {
            MidiMessage::ControlChg{channel, controller, value} => {
                let ctrl: u64 = ((channel as u64) << 8) | controller as u64; // Encode channel and CC number

                if self.selector.state == SelectorState::MidiLearn {
                    // MIDI learn: Send all controller events to the selector
                    // TODO: Directly handle MIDI learn in TUI
                    self.selector.handle_control_input(&mut self.selector_sm, ctrl, value.into(), self.sound.clone());
                    self.check_midi_learn_status();
                    return;
                }

                // Special handling of some controllers
                match controller {
                    0x01  => {
                        // ModWheel
                        if matches!(self.mode, Mode::Edit) {
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
                self.handle_ctrl_change(ctrl, value.into());

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
            self.last_value = synth_param;
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
        if self.sync_counter == 10 {
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
        self.sender.send(SynthMessage::Param(*param)).unwrap();

        // If the changed value is currently selected in the command line,
        // send it the updated value too.
        self.selector.value_has_changed(&mut self.selector_sm, *param);

        // Update UI
        let param_id = ParamId::new_from(param);
        let value = match param.value {
            ParameterValue::Float(v) => Value::Float(v),
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
        match self.state {
            TuiState::Help | TuiState::Name => return,
            _ => ()
        }

        if self.selection_changed {
            self.printer.set_color(self.current_color.fg_base, self.current_color.bg_base);
            print!("{}", clear::All);
            self.selection_changed = false;
            self.window.set_dirty(true);
        }

        if self.show_tui {
            self.window.draw(&mut self.printer);
        }

        if let Mode::Edit = self.mode {
            print!("{}{}", cursor::Goto(1, 1), clear::CurrentLine);
            self.display.display_selector(&mut self.printer, &self.selector);
        } else {
            print!("{}{}", cursor::Goto(1, 1), clear::CurrentLine);
            self.display.display_last_parameter(&mut self.printer, &self.last_value, &self.selector.wavetable_list);
        }
        if self.show_tui {
            self.display_idle_time();
            self.display_status_line();
        }
        self.printer.update();
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
        let ctrl_set = (ctrl_set + if ctrl_set <= 9 { b'0' } else { b'a' - 10 }) as char;
        self.printer.set_color(self.current_color.fg_base_l, self.current_color.bg_base);
        print!("{}| Mode: {:?} | Active controller set: {} |",
            cursor::Goto(1, 47), // TODO: Calculate y-position
            self.mode,
            ctrl_set);
        print!("{}Press <F1> for help, <F12> to exit ",
            cursor::Goto(80, 47));
    }

    fn display_name_prompt(&mut self) {
        self.printer.set_color(self.current_color.bg_base, self.current_color.fg_base);
        print!("{}{} Patch name: ", cursor::Goto(1, 1), clear::CurrentLine);
        self.printer.set_color(self.current_color.bg_base, self.current_color.fg_base_l);
        print!("{}", self.temp_name);
        stdout().flush().ok();
    }
}
