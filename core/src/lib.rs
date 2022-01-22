#[macro_use]
pub extern crate serde_derive;
#[cfg(feature = "handlebars_helpers")]
pub mod handlebars_helpers;
#[cfg(feature = "modules")]
pub mod modules;
#[cfg(feature = "modules")]
pub mod log {
    use env_logger::{Builder, Env, Target};

    pub fn setup() {
        let mut builder = Builder::from_env(Env::default().default_filter_or("trace"));
        builder.target(Target::Stdout);
        builder.init();
    }
}
#[cfg(feature = "render")]
pub extern crate rhai;

#[cfg(feature = "render")]
pub mod render;
