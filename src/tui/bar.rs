use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use super::Observer;
use super::{Value, get_int, get_float};
use super::{Printer, Widget, WidgetProperties};

type BarRef<Key> = Rc<RefCell<Bar<Key>>>;

/** A horizontal bar representing a value.
 *
 * Can have logarithmic scaling to improve visibility of smaller values.
 */
pub struct Bar<Key: Copy + Eq + Hash> {
    props: WidgetProperties<Key>,
    min: Value,
    max: Value,
    value: Value,
    logarithmic: bool, // Use logarithmic curve for values
}

impl<Key: Copy + Eq + Hash> Bar<Key> {
    pub fn new(min: Value, max: Value, value: Value) -> BarRef<Key> {
        let width = 10;
        let height = 1;
        let props = WidgetProperties::new(width, height);
        let logarithmic = false;
        Rc::new(RefCell::new(Bar{props, min, max, value, logarithmic}))
    }

    pub fn set_logarithmic(&mut self, l: bool) {
        self.logarithmic = l;
    }

    fn get_length(&self, value: &Value) -> usize {
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
        let scale = self.props.width as f64 / range;
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

impl<Key: Copy + Eq + Hash> Widget<Key> for Bar<Key> {
    fn get_widget_properties_mut(&mut self) -> &mut WidgetProperties<Key> {
        &mut self.props
    }

    fn get_widget_properties(&self) -> &WidgetProperties<Key> {
        &self.props
    }

    fn draw(&self, p: &mut dyn Printer) {
        let index = self.get_length(&self.value);
        // TODO: Optimize by using array
        p.set_color(self.props.colors.fg_compl, self.props.colors.bg_compl);
        for x in self.props.pos_x..self.props.pos_x + index {
            p.print(x, self.props.pos_y,"‾");
        }
    }
}

impl<Key: Copy + Eq + Hash> Observer for Bar<Key> {
    fn update(&mut self, value: Value) {
        self.value = value;
        self.set_dirty(true);
    }
}

#[test]
fn test_bar_translation() {
    // =====
    // Float
    // =====
    // Case 1: 0.0 - 1.0
    let b: BarRef<i32> = Bar::new(Value::Float(0.0), Value::Float(1.0), Value::Float(0.0));
    assert_eq!(b.borrow().get_length(&Value::Float(0.0)), 0);
    assert_eq!(b.borrow().get_length(&Value::Float(0.5)), 5);
    assert_eq!(b.borrow().get_length(&Value::Float(1.0)), 10);

    // Case 2: -1.0 - 1.0
    let b: BarRef<i32> = Bar::new(Value::Float(-1.0), Value::Float(1.0), Value::Float(0.0));
    assert_eq!(b.borrow().get_length(&Value::Float(-1.0)), 0);
    assert_eq!(b.borrow().get_length(&Value::Float(0.0)), 5);
    assert_eq!(b.borrow().get_length(&Value::Float(1.0)), 10);

    // Case 3: 2.0 - 10.0
    let b: BarRef<i32> = Bar::new(Value::Float(2.0), Value::Float(10.0), Value::Float(0.0));
    assert_eq!(b.borrow().get_length(&Value::Float(2.0)), 0);
    assert_eq!(b.borrow().get_length(&Value::Float(6.0)), 5);
    assert_eq!(b.borrow().get_length(&Value::Float(10.0)), 10);

    // ===
    // Int
    // ===
    // Case 1: 0 - 8
    let b: BarRef<i32> = Bar::new(Value::Int(0), Value::Int(8), Value::Int(0));
    assert_eq!(b.borrow().get_length(&Value::Int(0)), 0);
    assert_eq!(b.borrow().get_length(&Value::Int(4)), 5);
    assert_eq!(b.borrow().get_length(&Value::Int(8)), 10);

    // Case 2: -4 - 4
    let b: BarRef<i32> = Bar::new(Value::Int(-4), Value::Int(4), Value::Int(0));
    assert_eq!(b.borrow().get_length(&Value::Int(-4)), 0);
    assert_eq!(b.borrow().get_length(&Value::Int(0)), 5);
    assert_eq!(b.borrow().get_length(&Value::Int(4)), 10);

    // Case 3: 2 - 10
    let b: BarRef<i32> = Bar::new(Value::Int(2), Value::Int(10), Value::Int(0));
    assert_eq!(b.borrow().get_length(&Value::Int(2)), 0);
    assert_eq!(b.borrow().get_length(&Value::Int(6)), 5);
    assert_eq!(b.borrow().get_length(&Value::Int(10)), 10);
}
