use super::{ColorScheme, Printer};
use super::TermionWrapper;
use super::{Parameter, ParameterValue, SynthParam, ValueRange};
use super::MenuItem;
use super::{SelectorState, ParamSelector, next, ItemSelection};

use termion::{clear, cursor};

use std::fmt::Write;
use std::io::{stdout};
use std::io::Write as StdIoWrite;
use std::rc::Rc;

pub struct Display {
    color: Rc<ColorScheme>,
    termion: TermionWrapper,
    buffer: String,
}

impl Display {
    pub fn new(color: Rc<ColorScheme>, termion: TermionWrapper) -> Display {
        let buffer = String::with_capacity(100);
        Display{color, termion, buffer}
    }

    pub fn set_color_scheme(&mut self, color: Rc<ColorScheme>) {
        self.color = color;
    }

    pub fn display_last_parameter(&self, p: &mut dyn Printer, v: &SynthParam, wt_list: &[(usize, String)]) {
        p.set_color(self.color.bg_base, self.color.fg_base_l);
        print!("{} {} {}", v.function, v.function_id, v.parameter);
        p.set_color(self.color.bg_base, self.color.fg_base);
        match v.value {
            ParameterValue::Int(x) => print!(" {}", x),
            ParameterValue::Float(x) => print!(" {:.3}", x),
            ParameterValue::Choice(x) => {
                let val_range = MenuItem::get_val_range(v.function, v.parameter);
                if let ValueRange::Choice(list) = val_range {
                    print!(" {:?}", list[x].item);
                } else {
                    print!(" Unknown");
                }
            }
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
        p.set_color(self.color.fg_base, self.color.bg_base);
    }

    pub fn display_selector(&mut self, p: &mut dyn Printer, s: &ParamSelector) {
        let selector_state = s.state;
        let mut display_state = SelectorState::Function;
        let mut x_pos: usize;
        let mut selection = &s.func_selection;
        p.set_color(self.color.bg_base, self.color.fg_base_l);
        self.buffer.clear();
        loop {
            x_pos = self.buffer.len();
            if x_pos > 1 { x_pos += 1; }
            match display_state {
                SelectorState::Function => {
                    self.display_function(p, &s.func_selection, selector_state == SelectorState::Function);
                }
                SelectorState::FunctionIndex => {
                    self.display_function_index(p, &s.func_selection, selector_state == SelectorState::FunctionIndex);
                }
                SelectorState::Param => {
                    self.display_param(p, &s.param_selection, selector_state == SelectorState::Param);
                    selection = &s.param_selection;
                    if selector_state == SelectorState::Param {
                        // Display value after parameter
                        match s.param_selection.value {
                            ParameterValue::Int(_)
                            | ParameterValue::Float(_)
                            | ParameterValue::Choice(_)
                            | ParameterValue::Dynamic(_, _) => {
                                self.display_value(p, &s.param_selection, false, &s.wavetable_list);
                            },
                            ParameterValue::Function(_) => {
                                self.display_function(p, &s.value_func_selection, false);
                                self.display_function_index(p, &s.value_func_selection, false);
                            },
                            ParameterValue::Param(_) => {
                                self.display_function(p, &s.value_func_selection, false);
                                self.display_function_index(p, &s.value_func_selection, false);
                                self.display_param(p, &s.value_param_selection, false);
                            }
                            _ => ()
                        }
                    }
                }
                SelectorState::Value => {
                    self.display_value(p, &s.param_selection, selector_state == SelectorState::Value, &s.wavetable_list);
                }
                SelectorState::MidiLearn => {
                    if selector_state == SelectorState::MidiLearn {
                        self.display_midi_learn(p);
                    }
                }
                SelectorState::AddMarker => {
                    if selector_state == SelectorState::AddMarker {
                        self.display_add_marker(p);
                    }
                }
                SelectorState::GotoMarker => {
                    if selector_state == SelectorState::GotoMarker {
                        self.display_goto_marker(p);
                    }
                }
                SelectorState::ValueFunction => {
                    self.display_function(p, &s.value_func_selection, selector_state == SelectorState::ValueFunction);
                    selection = &s.value_func_selection;
                }
                SelectorState::ValueFunctionIndex => {
                    self.display_function_index(p, &s.value_func_selection, selector_state == SelectorState::ValueFunctionIndex);
                }
                SelectorState::ValueParam => {
                    self.display_param(p, &s.value_param_selection, selector_state == SelectorState::ValueParam);
                    selection = &s.value_param_selection;
                }
            }
            if display_state == selector_state {
                break;
            }
            display_state = next(display_state);
        }
        self.display_options(p, s, selection, x_pos as u16);
        p.set_color(self.color.fg_base, self.color.bg_base);
    }

    fn display_function(&mut self, p: &mut dyn Printer, func: &ItemSelection, selected: bool) {
        if selected {
            p.set_color(self.color.bg_base, self.color.fg_base);
        }
        print!("{}", func.item_list[func.item_index].item);
        write!(self.buffer, "{}", func.item_list[func.item_index].item).unwrap();
        if selected {
            p.set_color(self.color.bg_base, self.color.fg_base_l);
        }
    }

    fn display_function_index(&mut self, p: &mut dyn Printer, func: &ItemSelection, selected: bool) {
        if selected {
            p.set_color(self.color.bg_base, self.color.fg_base);
        }
        let function_id = if let ParameterValue::Int(x) = &func.value { *x as usize } else { panic!() };
        print!(" {}", function_id);
        write!(self.buffer, " {}", function_id).unwrap();
        if selected {
            p.set_color(self.color.bg_base, self.color.fg_base_l);
        }
    }

    fn display_param(&mut self, p: &mut dyn Printer, param: &ItemSelection, selected: bool) {
        if selected {
            p.set_color(self.color.bg_base, self.color.fg_base);
        }
        print!(" {} ", param.item_list[param.item_index].item);
        write!(self.buffer, " {} ", param.item_list[param.item_index].item).unwrap();
        if selected {
            p.set_color(self.color.bg_base, self.color.fg_base_l);
        }
    }

    fn display_value(&mut self, p: &mut dyn Printer, param: &ItemSelection, selected: bool, wt_list: &[(usize, String)]) {
        if selected {
            p.set_color(self.color.bg_base, self.color.fg_base);
        }
        match param.value {
            ParameterValue::Int(x) => {
                print!(" {}", x);
                write!(self.buffer, " {}", x).unwrap();
            }
            ParameterValue::Float(x) => {
                print!(" {:.3}", x);
                write!(self.buffer, " {:.3}", x).unwrap();
            }
            ParameterValue::Choice(x) => {
                let item = &param.item_list[param.item_index];
                let range = &item.val_range;
                let selection = if let ValueRange::Choice(list) = range { list } else { panic!("item: {:?}, range: {:?}", item, range) };
                let item = selection[x].item;
                print!(" {}", item);
                write!(self.buffer, " {}", item).unwrap();
            },
            ParameterValue::Dynamic(_, x) => {
                for (k, v) in wt_list {
                    if *k == x {
                        print!(" {}", v);
                        write!(self.buffer, " {}", v).unwrap();
                        break;
                    }
                }
            },
            _ => ()
        }
        if selected {
            p.set_color(self.color.bg_base, self.color.fg_base_l);
        }
    }

    fn display_midi_learn(&self, p: &mut dyn Printer) {
        p.set_color(self.color.bg_base, self.color.fg_base);
        print!("  MIDI Learn: Send controller data");
        p.set_color(self.color.bg_base, self.color.fg_base_l);
    }

    fn display_add_marker(&self, p: &mut dyn Printer) {
        p.set_color(self.color.bg_base, self.color.fg_base);
        print!("  Select marker to add");
        p.set_color(self.color.bg_base, self.color.fg_base_l);
    }

    fn display_goto_marker(&self, p: &mut dyn Printer) {
        p.set_color(self.color.bg_base, self.color.fg_base);
        print!("  Select marker to go to");
        p.set_color(self.color.bg_base, self.color.fg_base_l);
    }

    fn display_options(&self, p: &mut dyn Printer, selector: &ParamSelector, s: &ItemSelection, x_pos: u16) {
        p.set_color(self.color.bg_base, self.color.fg_base_l);
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
                ValueRange::Dynamic(param) => self.display_dynamic_options(selector, *param, x_pos),
                ValueRange::Func(_) => (),
                ValueRange::Param(_) => (),
                ValueRange::NoRange => ()
            }
        }
    }

    fn display_dynamic_options(&self, s: &ParamSelector, param: Parameter, x_pos: u16) {
        let list = s.get_dynamic_list_no_mut(param);
        let mut y_item = 2;
        for (key, value) in list.iter() {
            print!("{} {} - {} ", cursor::Goto(x_pos, y_item), key, value);
            y_item += 1;
        }
    }

    pub fn display_help(&mut self, p: &mut dyn Printer) {
        print!("{}{}", clear::All, cursor::Goto(1, 1));
        p.set_color(self.color.fg_base, self.color.bg_base);
        println!("Global keys:\r");
        println!("------------\r");
        println!("<TAB>    : Switch between Edit and Play mode\r");
        println!("<F1>     : Show this help text\r");
        println!("<F2>     : Save default sound bank\r");
        println!("<F3>     : Load default sound bank\r");
        println!("<Ctrl-C> : Copy current sound\r");
        println!("<Ctrl-V> : Paste copied sound to current patch\r");
        println!("<Ctrl-N> : Rename the current patch\r");
        println!("<F7>     : Cycle through color schemes\r");
        println!("<F12>    : Quit Yazz\r");
        println!("\r");
        println!("Keys in Edit mode:\r");
        println!("------------------\r");
        println!("</ >         : Move backwards/ forwards in the parameter history\r");
        println!("PgUp/ PgDown : Increase/ decrease function ID of current parameter\r");
        println!("[/ ]         : Move backwards/ forwards through the parameter list of the current function\r");
        println!("\"<MarkerID>  : Set a marker with the MarkerID at the current parameter\r");
        println!("\'<MarkerID>  : Recall the parameter with the given MarkerID\r");
        println!("/            : Create a new modulator for currently edited parameter\r");
        println!("<Ctrl-L>     : MIDI learn, assigns a MIDI controller to the currently edited parameter\r");
        println!("<Ctrl-L><Bsp>: Clear a MIDI controller assignment for the currently edited parameter\r");
        println!("\r");
        println!("Keys in Play mode:\r");
        println!("------------------\r");
        println!("+/ -         : Select next/ previous patch (discards changes if not saved)\r");
        println!("0 - 9, a - z : Select MIDI controller assignment set\r");
        println!("\r");
        println!("Press any key to continue.\r");
        stdout().flush().ok();
    }

}
