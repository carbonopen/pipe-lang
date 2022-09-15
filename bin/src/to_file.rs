use pipe_core::log;
use pipe_parser::value::Value;
use std::{fs::File, io::Write};

pub fn to_json(pipe: &Value, path: &str) {
    match File::create(path) {
        Ok(mut file) => match file.write_all(pipe.as_json().as_bytes()) {
            Ok(_) => log::info!("Create file {}", path),
            Err(err) => log::error!("{:?}", err),
        },
        Err(_) => (),
    }
}
