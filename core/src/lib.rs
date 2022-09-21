pub extern crate serde;
pub extern crate serde_derive;
pub extern crate serde_json;

#[cfg(feature = "modules")]
pub mod modules;
#[cfg(feature = "modules")]
pub extern crate log;

#[cfg(feature = "scripts")]
pub mod params;

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        println!("{:#?}", $($arg)*)
    };
}
