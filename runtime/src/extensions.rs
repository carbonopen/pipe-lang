use std::any::Any;
use std::fmt::Debug;

use crate::pipe::step::Step;

pub type Steps = Vec<Step>;

#[derive(PartialEq, Eq)]
pub enum ExtensionType {
    PreParse,
    PosParse,
}

pub trait Extension: Any + Send {
    fn handler(&self, steps: &mut Steps);
    fn extension_type(&self) -> ExtensionType;
}

impl Debug for dyn Extension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Extension").finish()
    }
}

#[macro_export]
macro_rules! declare_extension {
    ($module_type:ty, $constructor:path) => {
        #[no_mangle]
        pub extern "C" fn _Extension() -> *mut $crate::extensions::Extension {
            let constructor: fn() -> $module_type = $constructor;
            let object = constructor();
            let boxed: Box<$crate::extensions::Extension> = Box::new(object);
            Box::into_raw(boxed)
        }
    };
}

#[macro_export]
macro_rules! create_extension {
    ($handler:ident, $type:tt) => {
        use std::convert::TryInto;
        use $crate::{extensions::{Extension, ExtensionType}, declare_extension};

        #[derive(Debug, Default, Clone)]
        pub struct Custom {}

        impl Extension for Custom {
            fn handler(&self, step_list: &mut $crate::extensions::Steps) {
                $handler(step_list);
            }
            fn extension_type(&self) -> ExtensionType {
                ExtensionType::$type
            }
        }

        declare_extension!(Custom, Custom::default);
    };
}
