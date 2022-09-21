#[macro_use]
extern crate lab_core;

use lab_core::{
    modules::{Config, Listener, Return},
    serde_json::{Map, Value},
};

macro_rules! message {
    ($level:expr, $message:expr) => {
        format!(r#"{{"level":"{}", "message": {}}}"#, $level, $message)
    };
}

enum OutputType {
    Stdout,
}

struct Output {
    output_type: OutputType,
}

impl Output {
    pub fn new(options: Map<String, Value>) -> Self {
        let output_type = {
            let out_type = match options.get("type") {
                Some(value) => value.as_str().unwrap().to_string(),
                None => "stdout".to_string(),
            };

            if out_type.eq("stdout") {
                OutputType::Stdout
            } else {
                OutputType::Stdout
            }
        };
        Self { output_type }
    }

    pub fn send(&self, message: String) {
        match self.output_type {
            OutputType::Stdout => println!("{}", message),
        }
    }
}

pub fn lab_log<F: Fn(Return)>(listener: Listener, send: F, mut config: Config) {
    let mut default_config = Map::new();
    default_config.insert("type".to_string(), Value::String("stdout".to_string()));

    let level = match config.params.default_values.get("level") {
        Some(value) => value.as_str().unwrap().to_string(),
        None => "info".to_string(),
    };

    let options = match config.params.get_param("output") {
        Ok(value) => match value.as_object() {
            Some(value) => value.clone(),
            None => panic!("Error loading module settings."),
        },
        _ => default_config,
    };

    let output = Output::new(options);

    for request in listener {
        match config.params.set_request(&request) {
            Ok(_) => match config.params.get_param("message") {
                Ok(message) => {
                    output.send(message!(level, message));

                    send(Return {
                        payload: request.payload,
                        attach: config.default_attach.clone(),
                        trace: request.trace,
                    })
                }
                Err(err) => {
                    output.send(message!("error", err));

                    send(Return {
                        payload: request.payload,
                        attach: config.default_attach.clone(),
                        trace: request.trace,
                    })
                }
            },
            Err(err) => {
                output.send(message!("error", err));

                send(Return {
                    payload: request.payload,
                    attach: config.default_attach.clone(),
                    trace: request.trace,
                })
            }
        }
    }
}

create_module!(lab_log);
