#[macro_use]
extern crate pipe_core;

use std::convert::TryFrom;

use pipe_core::{
    modules::{Config, Listener, Return},
    scripts::Params,
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

fn switch<F: Fn(Return)>(listener: Listener, send: F, config: Config) {
    let switch_default_attach = config.default_attach.clone();

    if let Some(params_raw) = config.params {
        let mut params = Params::try_from(&params_raw).unwrap();

        let cases = match params.default.get("case") {
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

        println!("PARAMS: {:#?}", params);

        'listener: for request in listener {
            macro_rules! send_error {
                ($attach:expr) => {{
                    send(Return {
                        payload: request.payload.clone(),
                        attach: $attach.clone(),
                        trace_id: request.trace_id,
                    });
                    continue;
                }};
                ($attach:expr, $err:expr) => {{
                    send(Return {
                        payload: Err(Some(Value::from(format!("{}", $err)))),
                        attach: $attach.clone(),
                        trace_id: request.trace_id,
                    });
                    continue;
                }};
            }

            if let Ok(payload) = request.payload.clone() {
                match params.set_payload(payload.unwrap()) {
                    Ok(_) => match params.get_param("target") {
                        Ok(target_value) => {
                            for case in cases.iter() {
                                println!("CASES: {}, {}", target_value, case.case);
                                if target_value.eq(&case.case) {
                                    println!("CASES MATCH:  {}", target_value);
                                    send(Return {
                                        payload: request.payload.clone(),
                                        attach: Some(case.attach.clone()),
                                        trace_id: request.trace_id,
                                    });
                                    continue 'listener;
                                }
                            }

                            println!("NO MATCH");

                            send(Return {
                                payload: request.payload.clone(),
                                attach: switch_default_attach.clone(),
                                trace_id: request.trace_id,
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
            } else if switch_default_attach.is_some() {
                send_error!(switch_default_attach)
            }

            send_error!(config.default_attach)
        }
    }
}

create_module!(switch);

#[cfg(test)]
mod tests {
    use pipe_core::{
        modules::*,
        serde_json::{json, Value},
    };

    #[test]
    fn test_success() {
        let config = Config {
            reference: "test".parse().unwrap(),
            params: Some(json!({
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
                "target": param_test!(["payload.num"])
            })),
            producer: false,
            default_attach: None,
        };
        let payload = Ok(Some(json!({
            "num": "bar"
        })));
        let compare = Some("bar".to_string());
        create_module_assert_eq_attach!(crate::switch, config, payload, compare);
    }

    #[test]
    fn test_error() {
        let config = Config {
            reference: "test".parse().unwrap(),
            params: Some(json!({
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
                "target": param_test!(["payload.num"]),
                "attach": ""
            })),
            producer: false,
            default_attach: None,
        };
        let payload = Ok(Some(Value::default()));
        let compare = Err(Some(Value::from("hrai: Unknown property 'num' - a getter is not registered for type '()' (line 1, position 29) in call to function handler (line 1, position 46)".to_string())));

        create_module_assert_eq!(crate::switch, config, payload, compare);
    }
}
