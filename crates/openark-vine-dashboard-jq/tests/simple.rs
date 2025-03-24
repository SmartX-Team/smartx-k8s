use openark_vine_dashboard_jq::Settings;
use serde_json::{Value, json};

#[test]
fn test_json_pointer() {
    let filter = ".hello.world";
    let input = ::serde_json::to_string_pretty(&json!({
        "hello": {
            "world": "Hello World!",
        },
    }))
    .unwrap();

    let settings = Settings::default();
    let value = ::openark_vine_dashboard_jq::run(filter, &input, &settings).unwrap();
    assert_eq!(value, Value::String("Hello World!".into()));
}
