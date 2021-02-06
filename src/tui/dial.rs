use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use super::Observer;
use super::{Value, get_int, get_float};
use super::{Printer, Widget, WidgetProperties};

type DialRef<Key> = Rc<RefCell<Dial<Key>>>;

/** A circular dial representing a value.
 *
 * Can have logarithmic scaling to improve visibility of smaller values.
 */
pub struct Dial<Key: Copy + Eq + Hash> {
    props: WidgetProperties<Key>,
    min: Value,
    max: Value,
    value: Value,
    logarithmic: bool, // Use logarithmic curve for values
}

impl<Key: Copy + Eq + Hash> Dial<Key> {
    pub fn new(min: Value, max: Value, value: Value) -> DialRef<Key> {
        let width = 2;
        let height = 2;
        let props = WidgetProperties::new(width, height);
        let logarithmic = false;
        Rc::new(RefCell::new(Dial{props, min, max, value, logarithmic}))
    }

    pub fn set_logarithmic(&mut self, l: bool) {
        self.logarithmic = l;
    }

    pub fn get_index(&self, value: &Value) -> usize {
        let min: f64;
        let max: f64;
        let fvalue: f64;
        match value {
            Value::Int(v) => {
                min = get_int(&self.min) as f64;
                max = get_int(&self.max) as f64;
                fvalue = *v as f64;
            }
            Value::Float(v) => {
                min = get_float(&self.min);
                max = get_float(&self.max);
                fvalue = *v;
            }
            Value::Str(_) => panic!(),
        }
        let offset = min * -1.0;
        let range = max - min;
        let scale = 8.0 / range;
        let mut value = fvalue + offset;
        if self.logarithmic {
            // Using a logarithmic curve makes smaller values easier to see.
            let percent = value / range;
            let factor = percent.sqrt().sqrt(); // TODO: Slow, find a nicer way
            value = factor * range;
        }
        (value * scale) as usize
    }
}

impl<Key: Copy + Eq + Hash> Widget<Key> for Dial<Key> {
    fn get_widget_properties_mut(&mut self) -> &mut WidgetProperties<Key> {
        &mut self.props
    }

    fn get_widget_properties(&self) -> &WidgetProperties<Key> {
        &self.props
    }

    fn draw(&self, p: &mut dyn Printer) {
        let index = self.get_index(&self.value);
        // TODO: Optimize by using array
        let chars = match index {
            0 => "  ",
            1 => "  ",
            2 => "▁ ",
            3 => "\\ ",
            4 => " ▏",
            5 => " /",
            6 => " ▁",
            7 => "  ",
            _ => "  ",
            //_ => "  ",
        };
        p.set_color(self.props.colors.fg_compl_l, self.props.colors.bg_compl_l);
        p.print(self.props.pos_x, self.props.pos_y, chars);
        let chars = match index {
            0 => "/ ",
            1 => "▔ ",
            2 => "  ",
            3 => "  ",
            4 => "  ",
            5 => "  ",
            6 => "  ",
            7 => " ▔",
            _ => " \\",
            //_ => " ▏",
        };
        p.print(self.props.pos_x, self.props.pos_y + 1, chars);
    }
}

impl<Key: Copy + Eq + Hash> Observer for Dial<Key> {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }

    /*
    fn handle_mouse_event(&mut self, msg: &MouseMessage) {
        self.value = self.get_widget_properties().update_mouse(msg);
    }
    */
}

#[test]
fn dial_translation() {
    // =====
    // Float
    // =====
    // Case 1: 0.0 - 1.0
    let d: DialRef<i32> = Dial::new(Value::Float(0.0), Value::Float(1.0), Value::Float(0.0));
    assert_eq!(d.borrow().get_index(&Value::Float(0.0)), 0);
    assert_eq!(d.borrow().get_index(&Value::Float(0.5)), 4);
    assert_eq!(d.borrow().get_index(&Value::Float(1.0)), 8);

    // Case 2: -1.0 - 1.0
    let d: DialRef<i32> = Dial::new(Value::Float(-1.0), Value::Float(1.0), Value::Float(0.0));
    assert_eq!(d.borrow().get_index(&Value::Float(-1.0)), 0);
    assert_eq!(d.borrow().get_index(&Value::Float(0.0)), 4);
    assert_eq!(d.borrow().get_index(&Value::Float(1.0)), 8);

    // Case 3: 2.0 - 10.0
    let d: DialRef<i32> = Dial::new(Value::Float(2.0), Value::Float(10.0), Value::Float(0.0));
    assert_eq!(d.borrow().get_index(&Value::Float(2.0)), 0);
    assert_eq!(d.borrow().get_index(&Value::Float(6.0)), 4);
    assert_eq!(d.borrow().get_index(&Value::Float(10.0)), 8);

    // ===
    // Int
    // ===
    // Case 1: 0 - 8
    let d: DialRef<i32> = Dial::new(Value::Int(0), Value::Int(8), Value::Int(0));
    assert_eq!(d.borrow().get_index(&Value::Int(0)), 0);
    assert_eq!(d.borrow().get_index(&Value::Int(4)), 4);
    assert_eq!(d.borrow().get_index(&Value::Int(8)), 8);

    // Case 2: -4 - 4
    let d: DialRef<i32> = Dial::new(Value::Int(-4), Value::Int(4), Value::Int(0));
    assert_eq!(d.borrow().get_index(&Value::Int(-4)), 0);
    assert_eq!(d.borrow().get_index(&Value::Int(0)), 4);
    assert_eq!(d.borrow().get_index(&Value::Int(4)), 8);

    // Case 3: 2 - 10
    let d: DialRef<i32> = Dial::new(Value::Int(2), Value::Int(10), Value::Int(0));
    assert_eq!(d.borrow().get_index(&Value::Int(2)), 0);
    assert_eq!(d.borrow().get_index(&Value::Int(6)), 4);
    assert_eq!(d.borrow().get_index(&Value::Int(10)), 8);
}
