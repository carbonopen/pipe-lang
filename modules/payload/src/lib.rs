#[macro_use]
extern crate pipe_core;

use pipe_core::{
    handlebars_helpers::syntax_wrapper,
    log,
    modules::{Config, Listener, Return, TraceId},
    serde_json::json,
};
use regex::Regex;

pub fn payload<F: Fn(Return)>(listener: Listener, send: F, config: Config) {
    log::info!("{:?}", config);

    let template = match config.params {
        Some(template) => {
            let origin = template.to_string();
            let mut text = origin.clone();
            let re = Regex::new(r#"(?P<occurrence>("\{\{)((.|\n)*?)(}}"))"#).unwrap();

            for caps in re.captures_iter(&origin) {
                let with_quotes = caps["occurrence"].replace("\\\"", "\"");
                let new_value = {
                    let len = &with_quotes.len() - 3;
                    syntax_wrapper(&with_quotes[3..len])
                };
                text = text.replace(&caps["occurrence"], &new_value);
            }

            let re_calc = Regex::new(r#"(("\{\{)([ ]|)calc(?P<content>(.|\n)*?)(}}"))"#).unwrap();

            for caps in re_calc.captures_iter(&origin) {
                let with_quotes = format!("\"{}\"", &caps["content"]);
                text = text.replace(&caps["content"], &with_quotes);
            }

            Some(text)
        }
        _ => None,
    };

    if template.is_none() {
        for request in listener {
            send(Return {
                payload: request.payload,
                attach: config.default_attach.clone(),
                trace_id: request.trace_id,
            })
        }
    } else {
        register_helpers!(handlebars);

        match handlebars.register_template_string("template", &template.clone().unwrap()) {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        };

        if config.producer {
            match handlebars.render("template", &json!({})) {
                Ok(result) => {
                    let local_trace = TraceId::new();
                    let data = pipe_core::serde_json::from_str(&result).unwrap();

                    send(Return {
                        payload: Ok(data),
                        attach: config.default_attach.clone(),
                        trace_id: local_trace.id,
                    })
                }
                _ => {}
            };
        }

        for request in listener {
            match request.payload {
                Ok(payload) => match payload {
                    Some(content) => match handlebars.render("template", &content) {
                        Ok(result) => match pipe_core::serde_json::from_str(&result) {
                            Ok(data) => send(Return {
                                payload: Ok(data),
                                attach: config.default_attach.clone(),
                                trace_id: request.trace_id,
                            }),
                            _ => (),
                        },
                        Err(err) => {
                            log::error!("{}", err);

                            send(Return {
                                payload: Err(None), //todo: repassar erro
                                attach: config.default_attach.clone(),
                                trace_id: request.trace_id,
                            });
                        }
                    },
                    None => send(Return {
                        payload: Ok(None),
                        attach: config.default_attach.clone(),
                        trace_id: request.trace_id,
                    }),
                },
                Err(err) => send(Return {
                    payload: Err(err),
                    attach: config.default_attach.clone(),
                    trace_id: request.trace_id,
                }),
            }
        }
    }
}

create_module!(payload);
#[cfg(test)]
mod tests {
    use super::*;
    use pipe_core::serde_json::json;

    #[test]
    fn without_params() {
        let config = Config {
            reference: "test".parse().unwrap(),
            params: None,
            producer: false,
            default_attach: None,
        };
        create_module_assert_eq!(crate::payload, config, Ok(None), Ok(None));
    }

    #[test]
    fn payload() {
        let config = Config {
            reference: "test".parse().unwrap(),
            params: Some(json!({
                "value": "{{ products.0.price }}"
            })),
            producer: false,
            default_attach: None,
        };
        let payload = Ok(Some(json!({
            "products": [
                {
                    "price": 2.5
                }
            ]
        })));
        let compare = Ok(Some(json!({
            "value": 2.5
        })));

        create_module_assert_eq!(crate::payload, config, payload, compare);
    }

    #[test]
    fn calc() {
        let config = Config {
            reference: "test".parse().unwrap(),
            params: Some(json!({
                "value": "{{ calc \"((1.99 + 0.01) * number) / price\" number=number price=products.0.price}}"
            })),
            producer: false,
            default_attach: None,
        };
        let payload = Ok(Some(json!({
            "number": 10,
            "products": [
                {
                    "price": 2.5
                }
            ]
        })));
        let compare = Ok(Some(json!({
            "value": 8
        })));

        create_module_assert_eq!(crate::payload, config, payload, compare);
    }
}
