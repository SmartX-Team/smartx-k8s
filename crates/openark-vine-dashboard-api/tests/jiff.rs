use jiff::Timestamp;

#[test]
fn test_jiff_parse_timestamp() {
    let time: Timestamp = "2024-07-11T01:14:00Z".parse().unwrap();
    assert_eq!(time.to_string(), "2024-07-11T01:14:00Z")
}

#[test]
fn test_jiff_deserialize_timestamp() {
    let time: Timestamp = serde_json::from_str("\"2024-07-11T01:14:00Z\"").unwrap();
    assert_eq!(time.to_string(), "2024-07-11T01:14:00Z")
}
