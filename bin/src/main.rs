pub mod step;
mod pipe;
mod pipeline;
pub mod pos_parse;
mod runtime;
mod to_file;
use clap::Parser;
use env_logger::{Builder, Env, Target};
use pipe_core::log;
use pipe_parser::Pipe;
use runtime::Runtime;
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

    match args.to_json {
        Some(path) => match Pipe::from_path(&args.path) {
            Ok(pipe) => to_file::to_json(&pipe, &path),
            Err(err) => log::error!("{:?}: {}", err, &args.path),
        },
        None => match Runtime::builder(&args.path) {
            Ok(run) => run.start(),
            Err(err) => log::error!("{:?}: {}", err, &args.path),
        },
    }
}
