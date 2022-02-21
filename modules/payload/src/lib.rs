#[macro_use]
extern crate pipe_core;

use pipe_core::{
    log,
    modules::{Config, Listener, Return},
    serde::{Deserialize, Serialize},
    serde_json::{json, Value},
};

pub fn payload<F: Fn(Return)>(listener: Listener, send: F, config: Config) {
    log::info!("{:?}", config);

    match config.params {
        Some(template) => {
            // debug!(template)
        }
        _ => (),
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
                "body" : {
                    "value": {
                        "end": 20,
                        "script": "payload.params.number",
                        "start": 0
                    },
                    "type": "default"
                },
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
}
