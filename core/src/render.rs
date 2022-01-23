#[macro_export]
macro_rules! render {
    ($engine:ident, $type:ty, $payload:expr, $target:expr) => {{
        let mut scope = $crate::rhai::Scope::new();
        match $crate::rhai::serde::to_dynamic($payload) {
            Ok(value) => {
                scope.push_dynamic("payload", value);
                $engine.eval_with_scope::<$type>(&mut scope, $target)
            }
            Err(err) => Err(err),
        }
    }};
}

#[cfg(test)]
mod tests {
    use rhai::serde::to_dynamic;
    use rhai::{Engine, Scope};
    use serde_json::{json, Value};

    #[test]
    fn test_engine() {
        let engine = Engine::new();
        let mut scope = Scope::new();

        let json = json!({
            "item": [
                {
                    "item": 1,
                    "attach": "foo"
                },
                {
                    "item": 2,
                    "attach": "bar",
                }
            ],
            "target": "num"
        });

        let value = match serde_json::to_value(json) {
            Ok(value) => to_dynamic(value).unwrap(),
            Err(err) => panic!("Error: {:?}", err),
        };

        scope.push_dynamic("payload", value);

        let script = r#"
        payload.item[0].attach
        "#;

        match engine.eval_with_scope::<String>(&mut scope, script) {
            Ok(_) => assert!(true),
            Err(err) => println!("{:?}", err),
        }
    }
}
