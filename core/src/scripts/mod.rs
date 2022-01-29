pub extern crate rhai;
use std::convert::TryFrom;
#[macro_use]
pub mod macros;
use regex::Regex;
use rhai::{serde::to_dynamic, Engine, EvalAltResult, Scope, AST};
use serde_json::{Error as SerdeJsonError, Map, Value};

use crate::debug;

#[derive(Debug, Default)]
pub struct Error {
    serde_json: Option<SerdeJsonError>,
    rhai: Option<Box<EvalAltResult>>,
}

impl From<SerdeJsonError> for Error {
    fn from(error: SerdeJsonError) -> Self {
        Self {
            serde_json: Some(error),
            ..Default::default()
        }
    }
}

impl From<Box<EvalAltResult>> for Error {
    fn from(error: Box<EvalAltResult>) -> Self {
        Self {
            rhai: Some(error),
            ..Default::default()
        }
    }
}

struct ScriptInner {
    value: Value,
    scripts: Vec<Interpolation>,
}

impl ScriptInner {
    fn new(engine: &Engine, value: &Value) -> Self {
        let mut scripts = Vec::new();

        if let Some(obj) = value.as_object() {
            if let Some(obj_type_value) = obj.get("__type") {
                if obj_type_value.as_str().unwrap().eq("interpolation") {
                    let raw = obj.get("__raw").unwrap().as_str().unwrap().to_string();
                    let scripts = obj
                        .get("__scripts")
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|script| {
                            let obj = script.as_object().unwrap();
                            let script = obj.get("__script").unwrap().as_str().unwrap().to_string();
                            let target = obj.get("__target").unwrap().as_str().unwrap().to_string();
                            /*
                            When the interpolation is not a string interpolation,
                            it is a String Value. Therefore, to replace it, it is necessary
                            add double quotes.
                            */
                            let re =
                                Regex::new(&format!(r"^\$\{{(\s*|){}(\s*|)\}}$", script)).unwrap();

                            let target_fix = if re.is_match(&raw) {
                                format!(r#""{}""#, target)
                            } else {
                                target
                            };

                            let fun = format!(
                                "fn handler(payload){{ {} }}; to_string(handler(payload))",
                                script
                            );

                            let ast = engine
                                .compile(&fun)
                                .expect("Could not compile interpolation.");
                            Interpolation {
                                ast,
                                target: target_fix,
                            }
                        })
                        .collect::<Vec<_>>();

                    let value = obj.get("__replaced").unwrap().clone();
                    return Self { value, scripts };
                }
            }

            let map = obj
                .into_iter()
                .map(|(key, value)| {
                    let replaced = Self::new(engine, value);
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
                    let replaced = Self::new(engine, item);
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

#[derive(Clone, Debug)]
pub struct Interpolation {
    ast: AST,
    target: String,
}

#[derive(Debug)]
pub struct Script {
    replaced: String,
    scripts: Vec<Interpolation>,
    engine: Engine,
}

impl Script {
    pub fn resolve(&self, payload: Value) -> Result<String, Error> {
        match to_dynamic(payload) {
            Ok(dynamic) => {
                let mut replaced = self.replaced.clone();

                for inter in self.scripts.iter() {
                    let mut scope = Scope::new();
                    scope.push("payload", dynamic.clone());

                    match self
                        .engine
                        .eval_ast_with_scope::<String>(&mut scope, &inter.ast)
                    {
                        Ok(output) => {
                            replaced = replaced.replace(&inter.target, &output);
                        }
                        Err(err) => return Err(Error::from(err)),
                    };
                }

                Ok(replaced)
            }
            Err(err) => Err(Error::from(err)),
        }
    }

    pub fn resolve_value(&self, payload: Value) -> Result<Value, Error> {
        match self.resolve(payload) {
            Ok(value) => match serde_json::from_str(&value) {
                Ok(value) => Ok(value),
                Err(err) => Err(Error::from(err)),
            },
            Err(err) => Err(err),
        }
    }
}

impl TryFrom<&Value> for Script {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let engine = Engine::new();
        let inner = ScriptInner::new(&engine, value);

        match serde_json::to_string(&inner.value) {
            Ok(replaced) => Ok(Self {
                replaced,
                scripts: inner.scripts,
                engine,
            }),
            Err(err) => Err(Error::from(err)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::Script;
    use serde_json::json;
    use std::convert::TryFrom;

    #[test]
    fn test_interpolation() {
        let data = json!({
            "number": 1,
            "inter": {
                "__type": "interpolation",
                "__raw": "${ payload.item }",
                "__replaced": "#__{123}",
                "__scripts": [{
                    "__target": "#__{123}",
                    "__script": "payload.item"
                }]
            }
        });
        let compare = json!({
            "number": 1,
            "inter": 2
        });

        let payload = json!({
            "item": 2,
        });

        let script = Script::try_from(&data).unwrap();
        let resolve = script.resolve_value(payload).unwrap();

        assert_eq!(compare, resolve);
    }

    #[test]
    fn test_string_interpolation() {
        let data = json!({
            "number": 1,
            "inter": {
                "__type": "interpolation",
                "__raw": "string interpolation: ${ payload.item }",
                "__replaced": "string interpolation: #__{123}",
                "__scripts": [{
                    "__target": "#__{123}",
                    "__script": "payload.item"
                }]
            }
        });
        let compare = json!({
            "number": 1,
            "inter": "string interpolation: 2"
        });

        let payload = json!({
            "item": 2,
        });

        let script = Script::try_from(&data).unwrap();
        let resolve = script.resolve_value(payload).unwrap();

        assert_eq!(compare, resolve);
    }

    #[test]
    fn test_string_interpolation_2() {
        let data = json!({
            "number": 1,
            "inter": {
                "inner": {
                    "__type": "interpolation",
                    "__raw": "${ payload.item }",
                    "__replaced": "#__{123}",
                    "__scripts": [{
                        "__target": "#__{123}",
                        "__script": "payload.item"
                    }]
                },
                "other": true
            }
        });
        let compare = json!({
            "number": 1,
            "inter": {
                "inner": false,
                "other": true
            }
        });

        let payload = json!({
            "item": false,
        });

        let script = Script::try_from(&data).unwrap();
        let resolve = script.resolve_value(payload).unwrap();

        assert_eq!(compare, resolve);
    }

    #[test]
    fn test_complex() {
        let data = json!({
            "inter": {
                "string": {
                    "__type": "interpolation",
                    "__raw": "${ payload.string }",
                    "__replaced": "#__{string}",
                    "__scripts": [{
                        "__target": "#__{string}",
                        "__script": "payload.string"
                    }]
                },
                "bool": {
                    "__type": "interpolation",
                    "__raw": "${ payload.bool }",
                    "__replaced": "#__{bool}",
                    "__scripts": [{
                        "__target": "#__{bool}",
                        "__script": "payload.bool"
                    }]
                },
                "number": {
                    "__type": "interpolation",
                    "__raw": "${ payload.number }",
                    "__replaced": "#__{number}",
                    "__scripts": [{
                        "__target": "#__{number}",
                        "__script": "payload.number"
                    }]
                },
                "object": {
                    "__type": "interpolation",
                    "__raw": "${ payload.object }",
                    "__replaced": "#__{object}",
                    "__scripts": [{
                        "__target": "#__{object}",
                        "__script": "payload.object"
                    }]
                },
                "array": {
                    "__type": "interpolation",
                    "__raw": "${ payload.array }",
                    "__replaced": "#__{array}",
                    "__scripts": [{
                        "__target": "#__{array}",
                        "__script": "payload.array"
                    }]
                },
            }
        });
        let compare = json!({
            "inter": {
                "string": "foo",
                "bool": true,
                "number": 123,
                "object": {
                    "foo": "bar"
                },
                "array": ["foo", true, 123],
            }
        });

        let payload = json!({
            "string": "foo",
            "bool": true,
            "number": 123,
            "object": {
                "foo": "bar"
            },
            "array": ["foo", true, 123],
        });

        let script = Script::try_from(&data).unwrap();
        let resolve = script.resolve_value(payload).unwrap();

        assert_eq!(compare, resolve);
    }
}
