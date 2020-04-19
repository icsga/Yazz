use super::{CtrlMap, MappingType};
use super::{Parameter, ParameterValue, ParamId, SynthParam, ValueRange, FUNCTIONS, OSC_PARAMS, MOD_SOURCES};
use super::{Canvas, CanvasRef};
use super::Float;
use super::MidiMessage;
use super::{SelectorEvent, SelectorState, ParamSelector, next, ItemSelection};
use super::{SoundBank, SoundData, SoundPatch};
use super::{UiMessage, SynthMessage};
use super::surface::Surface;
use super::Value;
use super::{SOUND_DATA_VERSION, SYNTH_ENGINE_VERSION};
use super::RetCode;
use super::{StateMachine, SmEvent, SmResult};

use crossbeam_channel::{Sender, Receiver};
use log::{info, trace, warn};
use termion::{clear, color, cursor};
use termion::color::{Black, White, Red, LightWhite, Reset, Rgb};
use termion::event::Key;

use std::convert::TryInto;
use std::io;
use std::io::{stdout, Write};
use std::thread::spawn;
use std::time::{Duration, SystemTime};
use std::cell::RefCell;
use std::rc::Rc;

pub struct Tui {
    // Function selection
    sender: Sender<SynthMessage>,
    ui_receiver: Receiver<UiMessage>,

    // TUI handling
    selector: ParamSelector,
    selection_changed: bool,

    // Actual UI
    window: Surface,
    canvas: CanvasRef<ParamId>,
    sync_counter: u32,
    idle: Duration, // Accumulated idle times of the engine
    busy: Duration, // Accumulated busy times of the engine
    min_idle: Duration,
    max_busy: Duration,

    bank: SoundBank,   // Bank with sound patches
    sound: SoundPatch, // Current sound patch as loaded from disk
    selected_sound: usize,
    ctrl_map: CtrlMap, // Mapping of MIDI controller to parameter

    // State machine for ParamSelector
    sm: StateMachine<ParamSelector, SelectorEvent>,
}

impl Tui {
    pub fn new(sender: Sender<SynthMessage>, ui_receiver: Receiver<UiMessage>) -> Tui {
        let mut window = Surface::new();
        let canvas: CanvasRef<ParamId> = Canvas::new(50, 20);
        let sound = SoundPatch::new();
        window.set_position(1, 3);
        window.update_all(&sound.data);
        let (_, y) = window.get_size();
        window.add_child(canvas.clone(), 1, y);

        let mut tui = Tui{
            sender: sender,
            ui_receiver: ui_receiver,
            selector: ParamSelector::new(&FUNCTIONS, &MOD_SOURCES),
            selection_changed: true,
            window: window,
            canvas: canvas,
            sync_counter: 0,
            idle: Duration::new(0, 0),
            busy: Duration::new(0, 0),
            min_idle: Duration::new(10, 0),
            max_busy: Duration::new(0, 0),
            bank: SoundBank::new(SOUND_DATA_VERSION, SYNTH_ENGINE_VERSION),
            sound: sound,
            selected_sound: 0,
            ctrl_map: CtrlMap::new(),
            sm: StateMachine::new(ParamSelector::state_function),
        };
        tui.select_sound(0);
        let param = ParamId::new(Parameter::Oscillator, 1, Parameter::Level);

        // For testing: Hardcode volume controller
        tui.ctrl_map.add_mapping(tui.selected_sound,
                                 7 /* Channel volume */,
                                 MappingType::Absolute,
                                 &param,
                                 ValueRange::FloatRange(0.0, 100.0));
        tui
    }

    /** Start input handling thread.
     *
     * This thread receives messages from the terminal, the MIDI port, the
     * synth engine and the audio engine.
     */
    pub fn run(to_synth_sender: Sender<SynthMessage>,
               ui_receiver: Receiver<UiMessage>) -> std::thread::JoinHandle<()> {
        let handler = spawn(move || {
            let mut tui = Tui::new(to_synth_sender, ui_receiver);
            let mut keep_running = true;
            let sound_data = Rc::new(RefCell::new(tui.sound.data));
            while keep_running {
                let msg = tui.ui_receiver.recv().unwrap();
                match msg {
                    UiMessage::Midi(m)  => tui.handle_midi_event(&m, sound_data.clone()),
                    UiMessage::Key(m) => {
                        match m {
                            Key::F(1) => {
                                // Read bank from disk
                                tui.bank.load_bank("Yazz_FactoryBank.ysn").unwrap();
                                tui.select_sound(0);
                            },
                            Key::F(2) => {
                                // Copy current sound to selected sound in bank
                                tui.bank.set_sound(tui.selected_sound, &tui.sound);
                                // Write bank to disk
                                tui.bank.save_bank("Yazz_FactoryBank.ysn").unwrap()
                            },
                            _ => {
                                if tui.selector.handle_user_input(&mut tui.sm, m, sound_data.clone()) {
                                    tui.send_event();
                                }
                            }
                        }
                        tui.selection_changed = true; // Trigger full UI redraw
                    },
                    UiMessage::MousePress{x, y} |
                    UiMessage::MouseHold{x, y} |
                    UiMessage::MouseRelease{x, y} => {
                        tui.window.handle_event(&msg);
                    }
                    UiMessage::SampleBuffer(m, p) => tui.handle_samplebuffer(m, p),
                    UiMessage::EngineSync(idle, busy) => {
                        tui.update_idle_time(idle, busy);
                        tui.handle_engine_sync();
                    }
                    UiMessage::Exit => {
                        info!("Stopping TUI");
                        keep_running = false;
                        tui.sender.send(SynthMessage::Exit).unwrap();
                    }
                };
            }
        });
        handler
    }

    fn select_sound(&mut self, mut sound_index: usize) {
        if sound_index > 127 {
            sound_index = 127;
        }
        self.selected_sound = sound_index;
        let sound_ref: &SoundPatch = self.bank.get_sound(self.selected_sound);
        self.sound.name = sound_ref.name.clone();
        self.sound.data = sound_ref.data;
        // Send new sound to synth engine
        let sound_copy = self.sound.data;
        self.sender.send(SynthMessage::Sound(sound_copy)).unwrap();
        // Update display
        self.window.set_sound_info(self.selected_sound, &self.sound.name);
        self.window.update_all(&self.sound.data);
    }

    /* MIDI message received */
    fn handle_midi_event(&mut self, m: &MidiMessage, sound_data: Rc<RefCell<SoundData>>) {
        match *m {
            MidiMessage::ControlChg{channel, controller, value} => {
                if self.selector.state == SelectorState::MidiLearn {

                    // MIDI learn: Send all controller events to the selector
                    self.selector.handle_control_input(&mut self.sm, controller.into(), value.into(), sound_data);

                    // Check if complete, if yes set CtrlMap
                    if self.selector.ml.complete {
                        let ml = &mut self.selector.ml;
                        let function = &self.selector.func_selection.item_list[self.selector.func_selection.item_index];
                        let function_id = if let ParameterValue::Int(x) = &self.selector.func_selection.value { *x as usize } else { panic!() };
                        let parameter = &self.selector.param_selection.item_list[self.selector.param_selection.item_index];
                        let param = ParamId::new(function.item, function_id, parameter.item);
                        self.ctrl_map.add_mapping(self.selected_sound,
                                                  ml.ctrl,
                                                  MappingType::Absolute,
                                                  &param,
                                                  parameter.val_range);
                    }

                } else if controller == 0x01 { // ModWheel

                    // ModWheel is used as general data entry for selector
                    self.selector.handle_control_input(&mut self.sm, controller.into(), value.into(), sound_data);
                    self.send_event();

                } else {

                    // All others might be mapped to control a parameter directly
                    self.handle_ctrl_change(controller.into(), value.into());

                }
            },
            MidiMessage::ProgramChg{channel, program} => self.select_sound(program as usize - 1),
            _ => ()
        }
    }

    fn handle_ctrl_change(&mut self, controller: u64, value: u64) {
        let result = self.ctrl_map.get_value(self.selected_sound, controller, value, &self.sound.data);
        if let Ok(synth_param) = result {
            self.send_parameter(&synth_param);
        }
    }

    /* Received a buffer with samples from the synth engine. */
    fn handle_samplebuffer(&mut self, m: Vec<Float>, p: SynthParam) {
        self.canvas.borrow_mut().clear();
        match p.function {
            Parameter::Oscillator => {
                self.canvas.borrow_mut().plot(&m, -1.0, 1.0);
            }
            Parameter::Envelope => {
                self.canvas.borrow_mut().plot(&m, 0.0, 1.0);
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
            let display_time = SystemTime::now();
            self.display();

            self.sync_counter = 0;
            self.query_samplebuffer();
        }
    }

    /* Send an updated value to the synth engine. */
    fn send_event(&mut self) {
        // Update sound data
        let function = &self.selector.func_selection.item_list[self.selector.func_selection.item_index];
        let function_id = if let ParameterValue::Int(x) = &self.selector.func_selection.value { *x as usize } else { panic!() };
        let parameter = &self.selector.param_selection.item_list[self.selector.param_selection.item_index];
        let param_val = &self.selector.param_selection.value;
        let param = SynthParam::new(function.item, function_id, parameter.item, *param_val);
        self.send_parameter(&param);
    }

    fn send_parameter(&mut self, param: &SynthParam) {
        self.sound.data.set_parameter(&param);

        // Send new value to synth engine
        self.sender.send(SynthMessage::Param(param.clone())).unwrap();

        // Update UI
        let param_id = ParamId::new_from(param);
        let value = match param.value {
            ParameterValue::Float(v) => Value::Float(v.into()),
            ParameterValue::Int(v) => Value::Int(v),
            ParameterValue::Choice(v) => Value::Int(v.try_into().unwrap()),
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
        let f_s = &self.selector.func_selection;
        let function = &f_s.item_list[f_s.item_index];
        let function_id = if let ParameterValue::Int(x) = &f_s.value { *x as usize } else { panic!() };
        let p_s = &self.selector.param_selection;
        let parameter = &p_s.item_list[p_s.item_index];
        let param_val = &p_s.value;
        let param = SynthParam::new(function.item, function_id, parameter.item, *param_val);
        self.sender.send(SynthMessage::SampleBuffer(buffer, param)).unwrap();
    }

    /* ====================================================================== */

    /** Display the UI. */
    fn display(&mut self) {
        if self.selection_changed {
            print!("{}", clear::All);
            self.selection_changed = false;
            self.window.set_dirty(true);
        }

        self.window.draw();

        print!("{}{}", cursor::Goto(1, 1), clear::CurrentLine);
        Tui::display_selector(&self.selector);
        self.display_idle_time();

        io::stdout().flush().ok();
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
                }
                SelectorState::Value => {
                    Tui::display_value(&s.param_selection, selector_state == SelectorState::Value);
                    x_pos = 23;
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
                SelectorState::MidiLearn => {
                    Tui::display_midi_learn();
                }
            }
            if display_state == selector_state {
                break;
            }
            display_state = next(display_state);
        }
        Tui::display_options(selection, x_pos, selector_state);
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
        print!(" {}", param.item_list[param.item_index].item);
        if selected {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_value(param: &ItemSelection, selected: bool) {
        if selected {
            print!("{}{} ", color::Bg(LightWhite), color::Fg(Black));
        }
        match param.value {
            ParameterValue::Int(x) => print!("{}", x),
            ParameterValue::Float(x) => print!("{}", x),
            ParameterValue::Choice(x) => {
                let item = &param.item_list[param.item_index];
                let range = &item.val_range;
                let selection = if let ValueRange::ChoiceRange(list) = range { list } else { panic!() };
                let item = selection[x].item;
                print!("{}", item);
            },
            _ => ()
        }
        if selected {
            print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
        }
    }

    fn display_midi_learn() {
        print!("{}{}", color::Bg(LightWhite), color::Fg(Black));
        print!(" MIDI Learn: Send controller value");
        print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
    }


    fn display_options(s: &ItemSelection, x_pos: u16, selector_state: SelectorState) {
        print!("{}{}", color::Bg(Black), color::Fg(LightWhite));
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
            let (min, max) = if let ValueRange::IntRange(min, max) = item.val_range { (min, max) } else { panic!() };
            print!("{} {} - {} ", cursor::Goto(x_pos, 2), min, max);
        }
        if selector_state == SelectorState::Value {
            let range = &s.item_list[s.item_index].val_range;
            match range {
                ValueRange::IntRange(min, max) => print!("{} {} - {} ", cursor::Goto(x_pos, 2), min, max),
                ValueRange::FloatRange(min, max) => print!("{} {} - {} ", cursor::Goto(x_pos, 2), min, max),
                ValueRange::ChoiceRange(list) => print!("{} 1 - {} ", cursor::Goto(x_pos, 2), list.len()),
                ValueRange::FuncRange(list) => (),
                ValueRange::ParamRange(list) => (),
                ValueRange::NoRange => ()
            }
        }
        print!("{}{}", color::Bg(Rgb(255, 255, 255)), color::Fg(Black));
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
}
