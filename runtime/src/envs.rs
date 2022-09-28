use std::env;

pub struct Envs {
    pub runtime_path: String,
    pub runtime_extension_path: String,
    pub debug_enabled: bool,
    pub debug_size_limit: usize,
}

impl Envs {
    pub fn builder() -> Self {
        let runtime_path = env::var("LAB_LANG_PATH")
            .unwrap_or(env::current_dir().unwrap().to_str().unwrap().to_string());
        let runtime_extension_path = env::var("LAB_LANG_EXTENSIONS_PATH")
            .unwrap_or(format!("{}/{}", runtime_path, "extensions"));

        let debug_enabled = match env::var("LAB_LANG_DEBUG_ENABLED") {
            Ok(value) => value.as_str().eq("1"),
            Err(_) => false,
        };

        let debug_size_limit = match env::var("LAB_LANG_DEBUG_SIZE_LIMIT") {
            Ok(value) => value
                .parse::<usize>()
                .expect("LAB_LANG_DEBUG_SIZE_LIMIT invalid"),
            Err(_) => 30000,
        };

        Self {
            runtime_path,
            runtime_extension_path,
            debug_enabled,
            debug_size_limit,
        }
    }
}
