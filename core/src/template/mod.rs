pub extern crate rhai;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[macro_use]
pub mod macros;

#[derive(Clone, Serialize, Deserialize)]
pub struct Interpolation {
    script: String,
    id: String,
}

#[derive(Serialize, Deserialize)]
pub struct Template {
    value: Value,
    scripts: Vec<Interpolation>,
}

impl Template {
    pub fn resolve(&self) -> Result<String, ()> {
        match serde_json::to_string(&self.value) {
            Ok(json) => {
                let mut value = json.clone();
                self.scripts.iter().for_each(|inter| {
                    // TODO executar script com rhai
                    value = value.replace(&inter.id, "RUN_SCRIPT");
                });
                Ok(value)
            }
            Err(_err) => Err(()),
        }
    }
}

impl From<&Value> for Template {
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
