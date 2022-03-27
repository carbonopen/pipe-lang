#[macro_use]
extern crate pipe_core;

use std::process;

use pipe_core::modules::{Config, Listener, Return};

pub fn pipe_exit<F: Fn(Return)>(listener: Listener, _: F, _: Config) {
    for _ in listener {
        process::exit(0x0100);
    }
}

create_module!(pipe_exit);
