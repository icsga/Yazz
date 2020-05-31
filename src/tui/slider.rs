use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use termion::{color, cursor};

use super::Observer;
use super::{Value, get_int, get_float};
use super::{Widget, WidgetProperties};

pub type SliderRef<Key> = Rc<RefCell<Slider<Key>>>;

pub struct Slider<Key: Copy + Eq + Hash> {
    props: WidgetProperties<Key>,
    min: Value,
    max: Value,
    value: Value,
    logarithmic: bool, // Use logarithmic curve for values
}

impl<Key: Copy + Eq + Hash> Slider<Key> {
    pub fn new(min: Value, max: Value, value: Value) -> SliderRef<Key> {
        let width = 1;
        let height = 5;
        let props = WidgetProperties::new(width, height);
        let logarithmic = false;
        Rc::new(RefCell::new(Slider{props, min, max, value, logarithmic}))
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
        let scale = (self.props.height * 8) as f64 / range;
        let mut value = fvalue + offset;
        if self.logarithmic {
            // Using a logarithmic curve makes smaller values easier to see.
            let percent = value / range;
            let factor = percent.sqrt().sqrt(); // TODO: Slow, find a nicer way
            value = factor * range;
        }
        let index = (value * scale) as usize;
        index
    }

    pub fn set_logarithmic(&mut self, l: bool) {
        self.logarithmic = l;
    }
}

impl<Key: Copy + Eq + Hash> Widget<Key> for Slider<Key> {
    fn get_widget_properties_mut<'a>(&'a mut self) -> &'a mut WidgetProperties<Key> {
        return &mut self.props;
    }

    fn get_widget_properties<'a>(&'a self) -> &'a WidgetProperties<Key> {
        return &self.props;
    }

    fn draw(&self) {
        let mut index = self.get_index(&self.value);
        print!("{}{}", color::Bg(self.props.colors.bg_light2), color::Fg(self.props.colors.fg_dark2));
        for i in 0..self.props.height {
            let chars = if index >= 8 { "█" } else {
                match index % 8 {
                    0 =>  " ",
                    1 => "▁",
                    2 => "▂",
                    3 => "▃",
                    4 => "▄",
                    5 => "▅",
                    6 => "▆",
                    7 => "▇",
                    _ => panic!(),
                }
            };
            index = if index > 8 { index - 8 } else { 0 };
            print!("{}{}", cursor::Goto(self.props.pos_x, self.props.pos_y + (4 - i)), chars);
        }
    }
}

impl<Key: Copy + Eq + Hash> Observer for Slider<Key> {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}

#[test]
fn test_slider_translation() {
    // =====
    // Float
    // =====
    // Case 1: 0.0 - 1.0
    // Slider has default height of 4, so a value range of 0 - 32
    let d: SliderRef<i32> = Slider::new(Value::Float(0.0), Value::Float(1.0), Value::Float(0.0));
    assert_eq!(d.borrow().get_index(&Value::Float(0.0)), 0);
    assert_eq!(d.borrow().get_index(&Value::Float(0.5)), 16);
    assert_eq!(d.borrow().get_index(&Value::Float(1.0)), 32);

    // Case 2: -1.0 - 1.0
    let d: SliderRef<i32> = Slider::new(Value::Float(-1.0), Value::Float(1.0), Value::Float(0.0));
    assert_eq!(d.borrow().get_index(&Value::Float(-1.0)), 0);
    assert_eq!(d.borrow().get_index(&Value::Float(0.0)), 16);
    assert_eq!(d.borrow().get_index(&Value::Float(1.0)), 32);

    // Case 3: 2.0 - 10.0
    let d: SliderRef<i32> = Slider::new(Value::Float(2.0), Value::Float(10.0), Value::Float(0.0));
    assert_eq!(d.borrow().get_index(&Value::Float(2.0)), 0);
    assert_eq!(d.borrow().get_index(&Value::Float(6.0)), 16);
    assert_eq!(d.borrow().get_index(&Value::Float(10.0)), 32);

    // ===
    // Int
    // ===
    // Case 1: 0 - 8
    let d: SliderRef<i32> = Slider::new(Value::Int(0), Value::Int(8), Value::Int(0));
    assert_eq!(d.borrow().get_index(&Value::Int(0)), 0);
    assert_eq!(d.borrow().get_index(&Value::Int(4)), 16);
    assert_eq!(d.borrow().get_index(&Value::Int(8)), 32);

    // Case 2: -4 - 4
    let d: SliderRef<i32> = Slider::new(Value::Int(-4), Value::Int(4), Value::Int(0));
    assert_eq!(d.borrow().get_index(&Value::Int(-4)), 0);
    assert_eq!(d.borrow().get_index(&Value::Int(0)), 16);
    assert_eq!(d.borrow().get_index(&Value::Int(4)), 32);

    // Case 3: 2 - 10
    let d: SliderRef<i32> = Slider::new(Value::Int(2), Value::Int(10), Value::Int(0));
    assert_eq!(d.borrow().get_index(&Value::Int(2)), 0);
    assert_eq!(d.borrow().get_index(&Value::Int(6)), 16);
    assert_eq!(d.borrow().get_index(&Value::Int(10)), 32);
}
