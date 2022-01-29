use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub extern crate rhai;

#[macro_use]
pub mod macros;

#[derive(Clone, Serialize, Deserialize)]
pub struct Interpolation {
    script: String,
    id: String,
}

#[derive(Serialize, Deserialize)]
pub struct Replaced {
    value: Value,
    scripts: Vec<Interpolation>,
}

impl Replaced {
    pub fn resolve(&self) -> Result<String, ()> {
        match serde_json::to_string(&self.value) {
            Ok(json) => Ok(json),
            Err(_err) => Err(()),
        }
    }
}

impl From<&Value> for Replaced {
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
