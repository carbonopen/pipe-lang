#[macro_use]
extern crate pipe_core;

use pipe_core::modules::{Config, Listener, Return};

pub fn pipe_link<F: Fn(Return)>(listener: Listener, send: F, config: Config) {
    for request in listener {
        send(Return {
            payload: request.payload,
            attach: config.default_attach.clone(),
            trace_id: request.trace_id,
        })
    }
}

create_module!(pipe_link);
