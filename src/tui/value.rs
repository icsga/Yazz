pub enum Value {
    Int(i64),
    Float(f64),
    Str(&'static str),
}

pub fn get_int(value: &Value) -> i64 {
    if let Value::Int(x) = value { *x } else { panic!() }
}

pub fn get_float(value: &Value) -> f64 {
    if let Value::Float(x) = value { *x } else { panic!() }
}

pub fn get_str(value: &Value) -> &'static str {
    if let Value::Str(x) = value { x } else { panic!() }
}
