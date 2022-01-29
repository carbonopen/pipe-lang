pub extern crate serde;
pub extern crate serde_derive;
pub extern crate serde_json;
#[cfg(feature = "handlebars_helpers")]
pub mod handlebars_helpers;
#[cfg(feature = "modules")]
pub mod modules;
#[cfg(feature = "modules")]
pub extern crate log;

#[cfg(feature = "template")]
pub mod template;

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        println!("{:?}", $($arg)*)
    };
}
