use std::env;

pub struct Envs {
    pub runtime_path: String,
    pub runtime_extension_path: String,
}

impl Envs {
    pub fn builder() -> Self {
        let runtime_path = env::var("LAB_LANG_PATH")
            .unwrap_or(env::current_dir().unwrap().to_str().unwrap().to_string());
        let runtime_extension_path = env::var("LAB_LANG_EXTENSIONS_PATH")
            .unwrap_or(format!("{}/{}", runtime_path, "extensions"));

        Self {
            runtime_path,
            runtime_extension_path,
        }
    }
}
