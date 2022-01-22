#[macro_use]
extern crate pipe_core;
extern crate log;
extern crate serde_json;

use pipe_core::{
    log::setup as log_setup,
    modules::{Config, Listener, Return},
    rhai::{serde::to_dynamic, Engine, Scope},
};
use serde_json::Value;

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
    if !cfg!(test) {
        log_setup();
    }

    let engine = Engine::new();

    log::info!("{:?}", config);

    if let Some(params) = config.params {
        if let Some(params) = params.as_object() {
            let target = match params.get("target") {
                Some(value) => {
                    if value.is_object() {
                        value
                            .as_object()
                            .unwrap()
                            .get("scripts")
                            .unwrap()
                            .as_array()
                            .unwrap()
                            .get(0)
                            .unwrap()
                            .get("script")
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_string()
                    } else {
                        panic!("Target no Interpolation")
                    }
                }
                None => panic!("No target"),
            };

            let cases = match params.get("case") {
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

            let switch_default_attach = match params.get("attach") {
                Some(value) => Some(value.as_str().unwrap().to_string()),
                None => None,
            };

            let target = target.clone();
            'listener: for request in listener {
                macro_rules! listener_error {
                    () => {{
                        send(Return {
                            payload: request.payload.clone(),
                            attach: config.default_attach.clone(),
                            trace_id: request.trace_id,
                        });
                        continue;
                    }};
                }

                let target = target.clone();

                if let Ok(payload) = request.payload.clone() {
                    if let Some(payload) = payload {
                        let mut scope = Scope::new();
                        let payload_dyn = match to_dynamic(payload) {
                            Ok(value) => value,
                            Err(_) => listener_error!(),
                        };

                        scope.push_dynamic("payload", payload_dyn);

                        let target_value =
                            match engine.eval_with_scope::<String>(&mut scope, &target) {
                                Ok(value) => Value::from(value),
                                Err(_) => listener_error!(),
                            };

                        for case in cases.iter() {
                            if target_value.eq(&case.case) {
                                send(Return {
                                    payload: request.payload.clone(),
                                    attach: Some(case.attach.clone()),
                                    trace_id: request.trace_id,
                                });
                                continue 'listener;
                            }
                        }
                    }
                } else if switch_default_attach.is_some() {
                    listener_error!()
                }

                listener_error!()
            }
        }
    }
}

create_module!(switch);

#[cfg(test)]
mod tests {
    use pipe_core::modules::*;
    use serde_json::json;

    #[test]
    fn condition() {
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
                "target": {
                    "scripts": [
                        {
                            "script": "payload.num"
                        }
                    ]
                }
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
}
