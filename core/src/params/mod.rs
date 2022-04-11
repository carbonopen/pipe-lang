pub extern crate rhai;
use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt::{Debug, Display},
};
#[macro_use]
pub mod macros;
use regex::Regex;
use rhai::{serde::to_dynamic, Engine, EvalAltResult, ParseError, Scope, AST};
use serde_json::{Error as SerdeJsonError, Map, Value};

use crate::modules::Request;

#[derive(Debug)]
#[allow(dead_code)]

enum ParamError {
    NoObject,
    NotFoundParam,
    NotConvertNumber,
    NotConvertBoolean,
    NotConvertObject,
    NotConvertArray,
    CannotConvert,
    InternalError,
}

impl Display for ParamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamError::NoObject => write!(f, "Params not a object."),
            ParamError::NotFoundParam => write!(f, "Param not found."),
            ParamError::NotConvertNumber => write!(f, "Cannot convert to a number"),
            ParamError::NotConvertBoolean => write!(f, "Cannot convert to a boolean"),
            ParamError::NotConvertArray => write!(f, "Cannot convert to a array"),
            ParamError::NotConvertObject => write!(f, "Cannot convert to a object"),
            ParamError::CannotConvert => write!(f, "Cannot convert parameter"),
            ParamError::InternalError => write!(f, "Internal Error"),
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

impl Error {
    pub fn get_error(&self) -> Option<Value> {
        Some(Value::String(format!("{}", self)))
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

enum Converter {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Undefined,
}

impl Default for Converter {
    fn default() -> Self {
        Self::Undefined
    }
}

impl TryFrom<&str> for Converter {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "String" => Ok(Self::String),
            "Number" => Ok(Self::Number),
            "Boolean" => Ok(Self::Boolean),
            "Array" => Ok(Self::Array),
            "Object" => Ok(Self::Object),
            _ => Ok(Self::Undefined),
        }
    }
}

impl Debug for Converter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String => write!(f, "String"),
            Self::Number => write!(f, "Number"),
            Self::Boolean => write!(f, "Boolean"),
            Self::Array => write!(f, "Array"),
            Self::Object => write!(f, "Object"),
            Self::Undefined => write!(f, "Undefined"),
        }
    }
}

#[derive(Default, Debug)]
struct Param {
    converter: Option<Converter>,
    script: Option<AST>,
    default_value: Option<Value>,
}

macro_rules! to_value {
    ($value:expr) => {
        match serde_json::to_value($value) {
            Ok(value) => value,
            Err(err) => return Err(Error::from(err)),
        }
    };
}

impl Param {
    pub fn get(&self, engine: &Engine, scope: &mut Scope) -> Result<Value, Error> {
        let value = if let Some(ast) = &self.script {
            match engine.eval_ast_with_scope::<String>(scope, &ast) {
                Ok(value) => match serde_json::from_str(&value) {
                    Ok(value) => value,
                    Err(_) => to_value!(value),
                },
                Err(err) => return Err(Error::from(err)),
            }
        } else {
            match &self.default_value {
                Some(default_value) => default_value.clone(),
                None => return Err(Error::from(ParamError::InternalError)),
            }
        };

        if let Some(converter) = &self.converter {
            let value = match converter {
                Converter::String => {
                    if value.is_string() {
                        value
                    } else {
                        to_value!(format!("{}", value))
                    }
                }
                Converter::Number => match value.as_i64() {
                    Some(value) => to_value!(value),
                    None => match value.as_f64() {
                        Some(value) => to_value!(value),
                        None => return Err(Error::from(ParamError::NotConvertNumber)),
                    },
                },
                Converter::Boolean => match value.as_bool() {
                    Some(value) => to_value!(value),
                    None => return Err(Error::from(ParamError::NotConvertBoolean)),
                },
                Converter::Array => match value.as_array() {
                    Some(value) => to_value!(value),
                    None => return Err(Error::from(ParamError::NotConvertArray)),
                },
                Converter::Object => match value.as_object() {
                    Some(value) => to_value!(value),
                    None => return Err(Error::from(ParamError::NotConvertObject)),
                },
                Converter::Undefined => return Err(Error::from(ParamError::CannotConvert)),
            };

            Ok(value)
        } else {
            Ok(value)
        }
    }
}

#[derive(Debug)]
pub struct Params<'a> {
    pub default: HashMap<String, Value>,
    params: HashMap<String, Param>,
    engine: Engine,
    scope: Scope<'a>,
}

impl<'a> Params<'a> {
    pub fn set_request(&mut self, request: &Request) -> Result<(), Error> {
        let payload = match request.payload.clone() {
            Ok(payload) => match payload {
                Some(payload) => payload,
                None => Value::Null,
            },
            Err(err) => match err {
                Some(payload) => payload,
                None => Value::Null,
            },
        };
        let steps = match request.steps.clone() {
            Some(steps) => steps,
            None => HashMap::default(),
        };
        let origin = request.origin.clone();
        let trace_id = request.trace_id.clone();

        match to_dynamic(payload) {
            Ok(value) => {
                self.scope.push_dynamic("payload", value);
                match to_dynamic(steps) {
                    Ok(value) => {
                        self.scope.push_dynamic("steps", value);
                        match to_dynamic(origin) {
                            Ok(value) => {
                                self.scope.push_dynamic("origin", value);
                                match to_dynamic(trace_id) {
                                    Ok(value) => {
                                        self.scope.push_dynamic("trace_id", value);
                                        Ok(())
                                    }
                                    Err(err) => Err(Error::from(err)),
                                }
                            }
                            Err(err) => Err(Error::from(err)),
                        }
                    }
                    Err(err) => Err(Error::from(err)),
                }
            }
            Err(err) => Err(Error::from(err)),
        }
    }

    pub fn get_map(&mut self) -> Result<HashMap<String, Value>, Error> {
        let mut result = self.default.clone();

        for (key, param) in self.params.iter() {
            match param.get(&self.engine, &mut self.scope) {
                Ok(value) => {
                    result.insert(key.clone(), value);
                }
                Err(err) => return Err(err),
            }
        }

        Ok(result)
    }

    /// Returns param by name
    pub fn get_param(&mut self, name: &str) -> Result<Value, Error> {
        match self.params.get(name) {
            Some(param) => match param.get(&self.engine, &mut self.scope) {
                Ok(value) => Ok(value),
                Err(err) => Err(err),
            },
            None => match self.default.get(name) {
                Some(value) => Ok(value.clone()),
                None => Err(Error::from(ParamError::NotFoundParam)),
            },
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

impl<'a> Params<'a> {
    fn script_to_ast(
        engine: &Engine,
        re_quotes: &Regex,
        item: &Map<String, Value>,
    ) -> Result<AST, Error> {
        let script = item
            .get("___PIPE___list")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                let value = item.as_str().unwrap();
                value.to_string()
            })
            .collect::<Vec<_>>()
            .join("+");

        let script_escape_quotes = re_quotes.replace_all(&script, r#"""#);

        let handler = format!(
                                    "fn handler(payload, steps, origin, trace_id, args){{{}}};  to_string(handler(payload, steps, origin, trace_id, args))",
                                    script_escape_quotes
                                );

        match engine.compile(handler) {
            Ok(ast) => Ok(ast),
            Err(err) => Err(Error::from(err)),
        }
    }

    fn get_param_by_script(
        engine: &Engine,
        re_quotes: &Regex,
        item: &Map<String, Value>,
    ) -> Result<Param, Error> {
        match Self::script_to_ast(engine, re_quotes, item) {
            Ok(ast) => Ok(Param {
                converter: None,
                script: Some(ast),
                default_value: None,
            }),
            Err(err) => Err(Error::from(err)),
        }
    }

    fn get_param_by_converter(
        engine: &Engine,
        re_quotes: &Regex,
        item: &Map<String, Value>,
    ) -> Result<Param, Error> {
        let (script, default_value) = match item.get("___PIPE___default") {
            Some(default_value) => {
                if let Some(default_value_obj) = default_value.as_object() {
                    if let Some(pipe_param_type) = default_value_obj.get("___PIPE___type") {
                        if pipe_param_type.as_str().unwrap().eq("script") {
                            match Self::script_to_ast(&engine, &re_quotes, default_value_obj) {
                                Ok(ast) => (Some(ast), Some(default_value.clone())),
                                Err(err) => return Err(err),
                            }
                        } else {
                            (None, Some(default_value.clone()))
                        }
                    } else {
                        (None, Some(default_value.clone()))
                    }
                } else {
                    (None, Some(default_value.clone()))
                }
            }
            None => (None, None),
        };

        match item.get("___PIPE___value") {
            Some(value) => match value.as_str() {
                Some(value) => {
                    let converter = match Converter::try_from(value) {
                        Ok(value) => Some(value),
                        Err(err) => return Err(err),
                    };

                    Ok(Param {
                        converter,
                        script,
                        default_value,
                    })
                }
                None => todo!(),
            },
            None => Ok(Param {
                converter: None,
                script,
                default_value,
            }),
        }
    }
}

impl<'a> Params<'a> {
    pub fn builder(
        target: &Map<String, Value>,
        args: HashMap<String, Value>,
    ) -> Result<Self, Error> {
        let mut default = HashMap::new();
        let mut params = HashMap::new();
        let engine = Engine::new();
        let re_quotes = Regex::new(r#"\\\\""#).unwrap();

        for (key, value) in target.into_iter() {
            if let Some(item) = value.as_object() {
                if let Some(obj_type_value) = item.get("___PIPE___type") {
                    if obj_type_value.as_str().unwrap().eq("converter") {
                        match Self::get_param_by_converter(&engine, &re_quotes, item) {
                            Ok(param) => {
                                params.insert(key.clone(), param);
                            }
                            Err(err) => return Err(err),
                        }
                    } else if obj_type_value.as_str().unwrap().eq("script") {
                        match Self::get_param_by_script(&engine, &re_quotes, item) {
                            Ok(param) => {
                                params.insert(key.clone(), param);
                            }
                            Err(err) => return Err(err),
                        }
                    }
                }
            }

            default.insert(key.clone(), value.clone());
        }

        let mut scope = Scope::new();

        match to_dynamic(args) {
            Ok(value) => {
                scope.push_dynamic("args", value);
            }
            Err(err) => return Err(Error::from(err)),
        };

        Ok(Self {
            default,
            params,
            engine,
            scope,
        })
    }
}
#[cfg(test)]
mod test {
    use crate::modules::Request;

    use super::Params;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_full() {
        let data = json!({
            "param1": 1,
            "param2": pipe_param_convert!("String", pipe_param_script!(["(payload.number)"])),
            "param3": pipe_param_convert!("Number", pipe_param_script!(["(payload.string)"])),
            "param4": pipe_param_convert!("Boolean", pipe_param_script!(["(payload.boolean)"])),
            "param5": pipe_param_convert!("Array", pipe_param_script!(["(payload.array)"])),
            "param6": pipe_param_convert!("Object", pipe_param_script!(["(payload.object)"])),
            "param7": pipe_param_script!([r#""{\"item\": ""#,"(payload.number)", r#""}""#]),
        })
        .as_object()
        .unwrap()
        .clone();

        let compare = json!({
            "param1": 1,
            "param2": "2",
            "param3": 3,
            "param4": true,
            "param5": ["1", 2, true],
            "param6": {
                "item": 123
            },
            "param7": {
                "item": 2
            },
        });

        let payload = json!({
            "number": 2,
            "string": "3",
            "boolean": "true",
            "array": r#"["1", 2, true]"#,
            "object": r#"{
                "item": 123
            }"#,
        });

        let mut params = Params::builder(&data, HashMap::default()).unwrap();

        params
            .set_request(&Request::from_payload(payload))
            .expect("Payload error.");

        let resolve = params.get_value().unwrap();

        assert_eq!(compare, resolve);
    }
}
