#[macro_use]
extern crate pipe_core;

use pipe_core::modules::{Config, Listener, Return};

pub fn pipe_print<F: Fn(Return)>(listener: Listener, send: F, mut config: Config) {
    for request in listener {
        match config.params.set_request(&request) {
            Ok(_) => match config.params.get_param("text") {
                Ok(message) => {
                    println!("{}", message);

                    send(Return {
                        payload: request.payload,
                        attach: config.default_attach.clone(),
                        trace: request.trace,
                    })
                }
                Err(err) => send(Return {
                    payload: Err(err.get_error()),
                    attach: config.default_attach.clone(),
                    trace: request.trace,
                }),
            },
            Err(err) => send(Return {
                payload: Err(err.get_error()),
                attach: config.default_attach.clone(),
                trace: request.trace,
            }),
        }
    }
}

create_module!(pipe_print);
