#[derive(Debug)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
}

pub fn get_int(value: &Value) -> i64 {
    if let Value::Int(x) = value { *x } else { panic!("Got value {:?}", value) }
}

pub fn get_float(value: &Value) -> f64 {
    if let Value::Float(x) = value { *x } else { panic!("Got value {:?}", value) }
}

pub fn get_str(value: &Value) -> &String {
    if let Value::Str(x) = value { &x } else { panic!("Got value {:?}", value) }
}
