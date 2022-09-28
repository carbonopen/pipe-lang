mod trace;
mod envs;
pub mod extensions;
mod lab;
mod pipeline;
mod runtime;
pub mod step;
mod to_file;

use clap::Parser;
use env_logger::{Builder, Env, Target};
use envs::Envs;
use lab_core::log;
use lab_parser::Lab;
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

    log::trace!("Start Lab.");

    let args = Args::parse();
    let envs = Envs::builder();

    match args.to_json {
        Some(path) => match Lab::from_path(&args.path) {
            Ok(lab) => to_file::to_json(&lab, &path),
            Err(err) => log::error!("{:?}: {}", err, &args.path),
        },
        None => match Runtime::builder(&args.path, &envs) {
            Ok(run) => run.start(),
            Err(err) => log::error!("{:?}: {}", err, &args.path),
        },
    }
}
