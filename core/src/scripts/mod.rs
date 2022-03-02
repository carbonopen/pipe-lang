pub extern crate rhai;
use std::{collections::HashMap, convert::TryFrom, fmt::Display};
#[macro_use]
pub mod macros;
use rhai::{serde::to_dynamic, Engine, EvalAltResult, ParseError, Scope, AST};
use serde_json::{Error as SerdeJsonError, Value};

#[derive(Debug)]
enum ParamError {
    NoObject,
    NotFoundParam,
}

impl Display for ParamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamError::NoObject => write!(f, "Params not a object."),
            ParamError::NotFoundParam => write!(f, "Param not found."),
        }
    }
}

#[derive(Debug, Default)]
pub struct Error {
    serde_json: Option<SerdeJsonError>,
    rhai: Option<Box<EvalAltResult>>,
    ast: Option<ParseError>,
    param: Option<ParamError>,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(error) = &self.serde_json {
            write!(f, "serde_json: {}", error)
        } else if let Some(error) = &self.rhai {
            write!(f, "hrai: {}", error)
        } else if let Some(error) = &self.ast {
            write!(f, "ast: {}", error)
        } else if let Some(error) = &self.param {
            write!(f, "param: {}", error)
        } else {
            write!(f, "Params error")
        }
    }
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

impl From<ParseError> for Error {
    fn from(error: ParseError) -> Self {
        Self {
            ast: Some(error),
            ..Default::default()
        }
    }
}

impl From<ParamError> for Error {
    fn from(error: ParamError) -> Self {
        Self {
            param: Some(error),
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct Params<'a> {
    pub default: HashMap<String, Value>,
    scripts: HashMap<String, AST>,
    engine: Engine,
    scope: Scope<'a>,
}

impl<'a> Params<'a> {
    pub fn set_payload(&mut self, payload: Value) -> Result<(), Error> {
        match to_dynamic(payload) {
            Ok(dynamic) => {
                self.scope.push_dynamic("payload", dynamic);
                Ok(())
            }
            Err(err) => Err(Error::from(err)),
        }
    }

    fn compile(engine: &Engine, scope: &mut Scope, ast: &AST) -> Result<Value, Error> {
        match engine.eval_ast_with_scope::<String>(scope, &ast) {
            Ok(value) => match serde_json::from_str(&value) {
                Ok(value) => Ok(value),
                Err(_) => match serde_json::to_value(value) {
                    Ok(value) => Ok(value),
                    Err(err) => Err(Error::from(err)),
                },
            },
            Err(err) => Err(Error::from(err)),
        }
    }

    pub fn get_map(&mut self) -> Result<HashMap<String, Value>, Error> {
        let mut result = self.default.clone();

        for (key, ast) in self.scripts.iter() {
            match Self::compile(&self.engine, &mut self.scope, &ast) {
                Ok(value) => {
                    result.insert(key.clone(), value);
                }
                Err(err) => return Err(Error::from(err)),
            }
        }

        Ok(result)
    }

    /// Returns param by name
    pub fn get_param(&mut self, name: &str) -> Result<Value, Error> {
        match self.scripts.get(name) {
            Some(ast) => match Self::compile(&self.engine, &mut self.scope, &ast) {
                Ok(value) => Ok(value),
                Err(err) => return Err(Error::from(err)),
            },
            None => Err(Error::from(ParamError::NotFoundParam)),
        }
    }

    /// Returns all parameters compiled in a map
    pub fn get_value(&mut self) -> Result<Value, Error> {
        match self.get_map() {
            Ok(value) => match serde_json::to_value(value) {
                Ok(value) => Ok(value),
                Err(err) => Err(Error::from(err)),
            },
            Err(err) => Err(Error::from(err)),
        }
    }
}

impl<'a> TryFrom<&Value> for Params<'a> {
    type Error = Error;

    fn try_from(target: &Value) -> Result<Self, Self::Error> {
        let mut default = HashMap::new();
        let mut scripts = HashMap::new();
        let engine = Engine::new();

        match target.as_object() {
            Some(obj) => {
                for (key, value) in obj.into_iter() {
                    if let Some(item) = value.as_object() {
                        if let Some(obj_type_value) = item.get("___type") {
                            if obj_type_value.as_str().unwrap().eq("script") {
                                let script = item
                                    .get("list")
                                    .unwrap()
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .map(|item| item.as_str().unwrap())
                                    .collect::<Vec<_>>()
                                    .join("+");

                                let handler = format!(
                                    "fn handler(payload){{{}}}; to_string(handler(payload))",
                                    script
                                );

                                println!("{:?}", handler);

                                match engine.compile(handler) {
                                    Ok(ast) => {
                                        scripts.insert(key.clone(), ast);
                                    }
                                    Err(err) => return Err(Error::from(err)),
                                };
                            }
                        }
                    }

                    default.insert(key.clone(), value.clone());
                }

                Ok(Self {
                    default,
                    scripts,
                    engine,
                    scope: Scope::new(),
                })
            }
            None => Err(Error::from(ParamError::NoObject)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::Params;
    use serde_json::json;
    use std::convert::TryFrom;

    #[test]
    fn test_interpolation_object() {
        let data = json!({
            "param1": 1,
            "param2": param_test!([r#""{\"item\": ""#,
                    "(payload.item)",
                    r#""}""#])
        });
        let compare = json!({
            "param1": 1,
            "param2": "{\"item\": 2}"
        });

        let payload = json!({
            "item": 2,
        });

        let mut params = Params::try_from(&data).unwrap();
        params.set_payload(payload).expect("Payload error.");
        let resolve = params.get_value().unwrap();

        assert_eq!(compare, resolve);
    }

    #[test]
    fn test_interpolation() {
        let data = json!({
            "param1": 1,
            "param2": param_test!(["10 * payload.item"])
        });
        let compare = json!({
            "param1": 1,
            "param2": "20"
        });

        let payload = json!({
            "item": 2,
        });

        let mut params = Params::try_from(&data).unwrap();
        params.set_payload(payload).expect("Payload error.");
        let value = params.get_value().unwrap();

        assert_eq!(compare, value);
    }
}
