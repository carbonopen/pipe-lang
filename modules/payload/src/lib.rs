#[macro_use]
extern crate pipe_core;

use std::convert::TryFrom;

use pipe_core::{
    modules::{Config, Listener, Return},
    scripts::Params,
    serde_json::Value,
};

pub fn payload<F: Fn(Return)>(listener: Listener, send: F, config: Config) {
    match config.params {
        Some(params_raw) => {
            let mut params = Params::try_from(&params_raw).unwrap();

            for request in listener {
                match params.set_payload(request.payload.unwrap().unwrap()) {
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
        }
        _ => panic!("No params"),
    };
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
        };

        let payload = Ok(Some(json!({
            "number": 10
        })));
        let compare = Ok(Some(json!({
            "body" : "{\"value\": 10, \"type\": \"default\"}",
            "headers": {
                "content-type": "application/json"
            }
        })));

        create_module_assert_eq!(crate::payload, config, payload, compare);
    }
}
