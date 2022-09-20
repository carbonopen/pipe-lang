#[macro_use]
extern crate pipe_core;

use pipe_core::{
    modules::{Config, Listener, Return},
    params::Params,
    serde_json::Value,
};

#[derive(Debug)]
struct Case {
    pub case: Value,
    pub attach: String,
}

impl Case {
    pub fn new(case: Value, attach: String) -> Self {
        Self { case, attach }
    }
}

fn switch<F: Fn(Return)>(listener: Listener, send: F, mut config: Config) {
    let switch_default_attach = config.default_attach.clone();

    let cases = match config.params.default_values.get("case") {
        Some(case) => match case.as_array() {
            Some(cases) => {
                let mut cases_full = Vec::new();
                for case in cases {
                    let obj = case.as_object().unwrap();
                    let value_case = obj.get("case").unwrap().clone();
                    let value_attach = obj.get("attach").unwrap().clone();

                    cases_full.push(Case::new(
                        value_case,
                        value_attach.as_str().unwrap().to_string(),
                    ));
                }
                cases_full
            }
            _ => panic!("No case"),
        },
        _ => panic!("No case"),
    };

    'listener: for request in listener {
        let trace = request.trace.clone();
        macro_rules! send_error {
            ($attach:expr) => {{
                send(Return {
                    payload: request.payload.clone(),
                    attach: $attach.clone(),
                    trace: request.trace.clone(),
                });
                continue;
            }};
            ($attach:expr, $err:expr) => {{
                send(Return {
                    payload: Err(Some(Value::from(format!("{}", $err)))),
                    attach: $attach.clone(),
                    trace,
                });
                continue;
            }};
        }

        match config.params.set_request(&request) {
            Ok(_) => match config.params.get_param("target") {
                Ok(target_value) => {
                    for case in cases.iter() {
                        if target_value.eq(&case.case) {
                            send(Return {
                                payload: request.payload.clone(),
                                attach: Some(case.attach.clone()),
                                trace,
                            });
                            continue 'listener;
                        }
                    }

                    send(Return {
                        payload: request.payload.clone(),
                        attach: switch_default_attach.clone(),
                        trace,
                    });
                }
                Err(err) => {
                    send_error!(switch_default_attach, err);
                }
            },
            Err(err) => {
                send_error!(switch_default_attach, err);
            }
        }

        send_error!(config.default_attach)
    }
}

create_module!(switch);

#[cfg(test)]
mod tests {

    use std::convert::TryFrom;

    use pipe_core::{
        modules::*,
        params::Params,
        serde_json::{json, Value},
    };

    #[test]
    fn test_success() {
        let pre_config = PreConfig {
            reference: "test".parse().unwrap(),
            params: Params::try_from(
                json!({
                    "case": [
                        {
                            "case": "foo",
                            "attach": "foo"
                        },
                        {
                            "case": "bar",
                            "attach": "bar",
                        }
                    ],
                    "target": pipe_param_script!(["payload.num"])
                })
                .as_object()
                .unwrap()
                .clone(),
            )
            .unwrap(),
            producer: false,
            default_attach: None,
            tags: Default::default(),

            args: Default::default(),
        };
        let payload = Ok(Some(json!({
            "num": "bar"
        })));
        let compare = Some("bar".to_string());
        create_module_assert_eq_attach!(crate::switch, pre_config, payload, compare);
    }

    #[test]
    fn test_error() {
        let pre_config = PreConfig {
            reference: "test".parse().unwrap(),
            params: Params::try_from(
                json!({
                    "case": [
                        {
                            "case": "foo",
                            "attach": "foo"
                        },
                        {
                            "case": "bar",
                            "attach": "bar",
                        }
                    ],
                    "target": pipe_param_script!(["payload.num"]),
                    "attach": ""
                })
                .as_object()
                .unwrap()
                .clone(),
            )
            .unwrap(),
            producer: false,
            default_attach: None,
            tags: Default::default(),

            args: Default::default(),
        };
        let payload = Ok(Some(Value::default()));
        let compare = Err(Some(Value::from("hrai: Unknown property 'num' - a getter is not registered for type '()' (line 1, position 29) in call to function handler (line 1, position 46)".to_string())));

        create_module_assert_eq!(crate::switch, pre_config, payload, compare);
    }
}
