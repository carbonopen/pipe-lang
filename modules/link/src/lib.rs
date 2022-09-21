#[macro_use]
extern crate lab_core;

use lab_core::modules::{Config, Listener, Return, Trace, TraceId};

pub fn lab_link<F: Fn(Return)>(listener: Listener, send: F, config: Config) {
    if config.producer {
        let mut trace = TraceId::new();

        send(Return {
            payload: Ok(None),
            attach: config.default_attach.clone(),
            trace: Trace::new(trace.get_trace(), Default::default()),
        })
    }

    for request in listener {
        send(Return {
            payload: request.payload,
            attach: config.default_attach.clone(),
            trace: request.trace,
        })
    }
}

create_module!(lab_link);
