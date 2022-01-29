pub extern crate rhai;
use std::convert::TryFrom;

use rhai::{serde::to_dynamic, Engine, Scope};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

//TODO criar Error
pub struct Error {}

#[macro_use]
pub mod macros;

#[derive(Clone, Serialize, Deserialize)]
pub struct Interpolation {
    script: String,
    id: String,
}

pub struct Template {
    value: String,
    scripts: Vec<Interpolation>,
    engine: Engine,
}

impl Template {
    pub fn resolve(&self, payload: Value) -> Result<String, ()> {
        match to_dynamic(payload) {
            Ok(dynamic) => {
                let mut value = self.value.clone();

                for inter in self.scripts.iter() {
                    let mut scope = Scope::new();
                    scope.push_dynamic("payload", dynamic.clone());

                    match self
                        .engine
                        .eval_with_scope::<String>(&mut scope, &inter.script)
                    {
                        Ok(output) => {
                            value = value.replace(&inter.id, &output);
                        }
                        Err(_) => return Err(()),
                    };
                }

                Ok(value)
            }
            Err(_) => Err(()),
        }
    }

    pub fn resolve_value(&self, payload: Value) -> Result<Value, ()> {
        match self.resolve(payload) {
            Ok(value) => match serde_json::from_str(&value) {
                Ok(value) => Ok(value),
                Err(_) => Err(()),
            },
            Err(_) => Err(()),
        }
    }
}

impl TryFrom<&Value> for Template {
    type Error = ();

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let inner = TemplateInner::from(value);

        match serde_json::to_string(&inner.value) {
            Ok(value) => Ok(Self {
                value,
                scripts: inner.scripts,
                engine: Engine::new(),
            }),
            Err(_) => Err(()),
        }
    }
}

struct TemplateInner {
    value: Value,
    scripts: Vec<Interpolation>,
}

impl From<&Value> for TemplateInner {
    fn from(value: &Value) -> Self {
        let mut scripts = Vec::new();

        if let Some(obj) = value.as_object() {
            if let Some(obj_type_value) = obj.get("__type") {
                if obj_type_value.as_str().unwrap().eq("interpolation") {
                    let scripts = obj
                        .get("__scripts")
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|script| {
                            let obj = script.as_object().unwrap();
                            let script = obj.get("__script").unwrap().as_str().unwrap().to_string();
                            let id = obj.get("__id").unwrap().as_str().unwrap().to_string();
                            Interpolation { script, id }
                        })
                        .collect::<Vec<_>>();

                    let value = obj.get("__replaced").unwrap().clone();
                    return Self { value, scripts };
                }
            }

            let map = obj
                .into_iter()
                .map(|(key, value)| {
                    let replaced = Self::from(value);
                    scripts.extend(replaced.scripts);
                    (key.clone(), replaced.value)
                })
                .collect::<Map<String, Value>>();

            let value = Value::from(map);
            Self { value, scripts }
        } else if let Some(array) = value.as_array() {
            let list = array
                .iter()
                .map(|item| {
                    let replaced = Self::from(item);
                    scripts.extend(replaced.scripts);
                    replaced.value
                })
                .collect::<Vec<_>>();

            let value = Value::from(list);
            Self { value, scripts }
        } else {
            Self {
                value: value.clone(),
                scripts,
            }
        }
    }
}
