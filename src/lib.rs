#[macro_use]
mod careful;

use miniserde::{
    de::{Map, Seq, Visitor},
    json::{Number, Value},
    Deserialize, Error, Result,
};
use std::collections::btree_map;
use std::slice;

enum Event<'a> {
    Visitor(&'a Value, &'a mut dyn Visitor),
    Seq(slice::Iter<'a, Value>, Box<dyn Seq>),
    Map(btree_map::Iter<'a, String, Value>, Box<dyn Map>),
}

pub fn from_value<T: Deserialize>(v: &Value) -> Result<T> {
    let mut out = None;
    let mut stack = Vec::new();
    stack.push(Event::Visitor(v, T::begin(&mut out)));
    while let Some(event) = stack.pop() {
        match event {
            Event::Visitor(v, visitor) => match v {
                Value::Null => visitor.null()?,
                Value::Bool(b) => visitor.boolean(*b)?,
                Value::String(ref s) => visitor.string(s)?,
                Value::Number(Number::U64(n)) => visitor.nonnegative(*n)?,
                Value::Number(Number::I64(n)) => visitor.negative(*n)?,
                Value::Number(Number::F64(n)) => visitor.float(*n)?,
                Value::Array(a) => {
                    stack.push(Event::Seq(
                        a.iter(),
                        careful!(visitor.seq()? as Box<dyn Seq>),
                    ));
                }
                Value::Object(o) => {
                    stack.push(Event::Map(
                        o.iter(),
                        careful!(visitor.map()? as Box<dyn Map>),
                    ));
                }
            },
            Event::Seq(mut arr, mut seq) => match arr.next() {
                Some(v) => {
                    let element = careful!(seq.element()? as &mut dyn Visitor);
                    stack.push(Event::Seq(arr, seq));
                    stack.push(Event::Visitor(v, element));
                }
                None => seq.finish()?,
            },
            Event::Map(mut obj, mut map) => match obj.next() {
                Some((k, v)) => {
                    let key = careful!(map.key(k)? as &mut dyn Visitor);
                    stack.push(Event::Map(obj, map));
                    stack.push(Event::Visitor(v, key));
                }
                None => map.finish()?,
            },
        }
    }
    out.ok_or(Error)
}

#[test]
fn simple() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct S {
        s: String,
        i: i32,
        v: Vec<f64>,
    }
    let v: Value =
        miniserde::json::from_str(r#"{"s":"This is a test", "i":24, "v": [10, 1.2, -50]}"#)
            .unwrap();
    let s: S = from_value(&v).unwrap();
    assert_eq!(
        S {
            s: "This is a test".into(),
            i: 24,
            v: vec![10.0, 1.2, -50.0]
        },
        s
    );
}
