#[macro_use]
extern crate lab_core;

use std::process;

use lab_core::modules::{Config, Listener, Return};

pub fn lab_exit<F: Fn(Return)>(listener: Listener, _: F, _: Config) {
    for _ in listener {
        process::exit(0x0100);
    }
}

create_module!(lab_exit);
