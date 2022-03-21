#[macro_use]
extern crate pipe_core;

use pipe_core::modules::{Config, Listener, Return, TraceId};

fn mock<F: Fn(Return)>(listener: Listener, send: F, config: Config) {
    if config.producer {
        let local_trace = TraceId::new();

        send(Return {
            payload: Ok(config.params),
            attach: config.default_attach.clone(),
            trace_id: local_trace.id,
        });
    }

    for request in listener {
        send(Return {
            payload: request.payload,
            attach: config.default_attach.clone(),
            trace_id: request.trace_id,
        });
    }
}

create_module!(mock);

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use pipe_core::modules::*;

    #[test]
    fn with_producer() {
        let config = Config {
            reference: "test".parse().unwrap(),
            params: None,
            producer: true,
            default_attach: None,
            tags: HashMap::default(),
        };
        create_module_assert_eq!(crate::mock, config, Ok(None), Ok(None), false);
    }

    #[test]
    fn without_producer() {
        let config = Config {
            reference: "test".parse().unwrap(),
            params: None,
            producer: true,
            default_attach: None,
            tags: HashMap::default(),
        };
        create_module_assert_eq!(crate::mock, config, Ok(None), Ok(None), true);
    }
}
