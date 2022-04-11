#[macro_use]
extern crate pipe_core;

use pipe_core::{
    modules::{Config, Listener, Request, Return, Trace, TraceId},
    params::Params,
    serde_json::Value,
};

pub fn payload<F: Fn(Return)>(listener: Listener, send: F, config: Config) {
    let mut params = Params::builder(&config.params, config.args).unwrap();

    if config.producer {
        let mut trace = TraceId::new();

        match params.set_request(&Request::default()) {
            Ok(_) => match params.get_value() {
                Ok(new_payload) => send(Return {
                    payload: Ok(Some(new_payload)),
                    attach: config.default_attach.clone(),
                    trace: Trace::new(trace.get_trace(), Default::default()),
                }),
                Err(err) => send(Return {
                    payload: Err(Some(Value::from(format!("{}", err)))),
                    attach: config.default_attach.clone(),
                    trace: Trace::new(trace.get_trace(), Default::default()),
                }),
            },
            Err(err) => send(Return {
                payload: Err(Some(Value::from(format!("{}", err)))),
                attach: config.default_attach.clone(),
                trace: Trace::new(trace.get_trace(), Default::default()),
            }),
        }
    }

    for request in listener {
        match params.set_request(&request) {
            Ok(_) => match params.get_value() {
                Ok(new_payload) => send(Return {
                    payload: Ok(Some(new_payload)),
                    attach: config.default_attach.clone(),
                    trace: request.trace,
                }),
                Err(err) => send(Return {
                    payload: Err(Some(Value::from(format!("{}", err)))),
                    attach: config.default_attach.clone(),
                    trace: request.trace,
                }),
            },
            Err(err) => send(Return {
                payload: Err(Some(Value::from(format!("{}", err)))),
                attach: config.default_attach.clone(),
                trace: request.trace,
            }),
        }
    }
}

create_module!(payload);
#[cfg(test)]
mod tests {

    use super::*;
    use pipe_core::serde_json::json;

    #[test]
    fn test_payload() {
        let config = Config {
            reference: "test".parse().unwrap(),
            params: json!({
                "body" : pipe_param_script!([
                    r#""{\"value\": ""#,
                    "(payload.number)",
                    r#"", \"type\": \"default\"}""#
                ]),
                "headers": {
                    "content-type": "application/json"
                }
            })
            .as_object()
            .unwrap()
            .clone(),
            producer: false,
            default_attach: None,
            tags: Default::default(),

            args: Default::default(),
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
            params: json!({
                "body" : pipe_param_script!([
                    r#""{\"value\": ""#,
                    "(\"\\\"\" + payload.number + \"\\\"\")",
                    r#"", \"type\": \"default\"}""#
                ]),
                "headers": {
                    "content-type": "application/json"
                }
            })
            .as_object()
            .unwrap()
            .clone(),
            producer: false,
            default_attach: None,
            tags: Default::default(),

            args: Default::default(),
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
