#[macro_use]
extern crate pipe_core;

use std::convert::TryFrom;

use pipe_core::{
    modules::{Config, Listener, Return, TraceId},
    scripts::Params,
    serde_json::{Map, Value},
};

pub fn payload<F: Fn(Return)>(listener: Listener, send: F, config: Config) {
    match config.params {
        Some(params_raw) => {
            let mut params = Params::try_from(&params_raw).unwrap();

            if config.producer {
                let mut trace = TraceId::new();

                match params.set_payload(Value::Null) {
                    Ok(_) => match params.get_value() {
                        Ok(new_payload) => send(Return {
                            payload: Ok(Some(new_payload)),
                            attach: config.default_attach.clone(),
                            trace_id: trace.get_trace(),
                        }),
                        Err(err) => send(Return {
                            payload: Err(Some(Value::from(format!("{}", err)))),
                            attach: config.default_attach.clone(),
                            trace_id: trace.get_trace(),
                        }),
                    },
                    Err(err) => send(Return {
                        payload: Err(Some(Value::from(format!("{}", err)))),
                        attach: config.default_attach.clone(),
                        trace_id: trace.get_trace(),
                    }),
                }
            }

            for request in listener {
                match request.payload {
                    Ok(payload) => {
                        let value = payload.unwrap_or(Value::Object(Map::default()));
                        match params.set_payload(value) {
                            Ok(_) => match params.get_value() {
                                Ok(new_payload) => send(Return {
                                    payload: Ok(Some(new_payload)),
                                    attach: config.default_attach.clone(),
                                    trace_id: request.trace_id,
                                }),
                                Err(err) => send(Return {
                                    payload: Err(Some(Value::from(format!("{}", err)))),
                                    attach: config.default_attach.clone(),
                                    trace_id: request.trace_id,
                                }),
                            },
                            Err(err) => send(Return {
                                payload: Err(Some(Value::from(format!("{}", err)))),
                                attach: config.default_attach.clone(),
                                trace_id: request.trace_id,
                            }),
                        }
                    }
                    Err(err) => send(Return {
                        payload: Err(err),
                        attach: config.default_attach.clone(),
                        trace_id: request.trace_id,
                    }),
                }
            }
        }
        _ => panic!("No params"),
    };
}

create_module!(payload);
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use pipe_core::serde_json::json;

    #[test]
    fn test_payload() {
        let config = Config {
            reference: "test".parse().unwrap(),
            params: Some(json!({
                "body" : param_test!([
                    r#""{\"value\": ""#,
                    "(payload.number)",
                    r#"", \"type\": \"default\"}""#
                ]),
                "headers": {
                    "content-type": "application/json"
                }
            })),
            producer: false,
            default_attach: None,
            tags: HashMap::default(),
        };

        let payload = Ok(Some(json!({
            "number": 10
        })));
        let compare = Ok(Some(json!({
            "body" : {
                "value": 10,
                "type": "default"
            },
            "headers": {
                "content-type": "application/json"
            }
        })));

        create_module_assert_eq!(crate::payload, config, payload, compare);
    }

    #[test]
    fn test_payload_quotes() {
        let config = Config {
            reference: "test".parse().unwrap(),
            params: Some(json!({
                "body" : param_test!([
                    r#""{\"value\": ""#,
                    "(\"\\\"\" + payload.number + \"\\\"\")",
                    r#"", \"type\": \"default\"}""#
                ]),
                "headers": {
                    "content-type": "application/json"
                }
            })),
            producer: false,
            default_attach: None,
            tags: HashMap::default(),
        };

        let payload = Ok(Some(json!({
            "number": 10
        })));
        let compare = Ok(Some(json!({
            "body" : {
                "value": "10",
                "type": "default"
            },
            "headers": {
                "content-type": "application/json"
            }
        })));

        create_module_assert_eq!(crate::payload, config, payload, compare);
    }
}
