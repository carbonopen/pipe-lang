mod pipe;
mod runtime;
mod to_file;
use clap::Parser;
use env_logger::{Builder, Env, Target};
use pipe_core::log;
use pipe_parser::Pipe;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    path: String,

    #[clap(long = "json")]
    to_json: Option<String>,
}

fn main() {
    let mut builder = Builder::from_env(Env::default().default_filter_or("info"));
    builder.target(Target::Stdout);
    builder.init();

    log::trace!("Start Pipe.");

    let args = Args::parse();

    match Pipe::from_path(&args.path) {
        Ok(pipe) => match args.to_json {
            Some(path) => to_file::to_json(&pipe, &path),
            None => runtime::runtime(pipe),
        },
        Err(err) => log::error!("{:?}", err),
    };
}
