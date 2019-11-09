use super::Value;

pub trait Observer {
    fn update(&mut self, value: Value);
}
