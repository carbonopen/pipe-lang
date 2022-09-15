#[macro_use]
extern crate pipe_core;

use pipe_core::{
    modules::{Config, Listener, Return},
    params::Params,
};

pub fn pipe_print<F: Fn(Return)>(listener: Listener, send: F, config: Config) {
    let mut params = Params::builder(&config.params, config.args).unwrap();

    for request in listener {
        match params.set_request(&request) {
            Ok(_) => match params.get_param("text") {
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