use std::{fs::File, io::Write};

use pipe_parser::value::Value;

pub fn to_json(pipe: &Value, path: &str) {
    match File::create(path) {
        Ok(mut file) => match file.write_all(pipe.as_json().as_bytes()) {
            Ok(_) => println!("Create file {}", path),
            Err(err) => println!("{:?}", err),
        },
        Err(_) => todo!(),
    }
}
