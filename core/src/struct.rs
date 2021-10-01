use std::collections::HashMap;

// use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Number {
    n: String,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Step {
    pub module: Option<String>,
    pub params: Option<Value>,
    pub payload: Option<Payload>,
    pub reference: Option<String>,
    pub producer: Option<bool>,
    pub attach: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Payload {
    pub request: Option<Value>,
    pub response: Option<Value>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Module {
    pub name: String,
    pub source: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Co2 {
    pub version: Option<String>,
    pub modules: Option<Vec<Module>>,
    pub pipeline: Vec<Step>,
}
