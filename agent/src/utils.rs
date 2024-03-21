pub(crate) fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    value == &Default::default()
}

pub(crate) fn humantime_duration(
    gen: &mut schemars::gen::SchemaGenerator,
) -> schemars::schema::Schema {
    use schemars::schema::*;
    use schemars::JsonSchema;
    use serde_json::json;

    let mut schema: SchemaObject = <String>::json_schema(gen).into();
    schema.metadata = Some(Box::new(Metadata {
        id: None,
        title: None,
        description: Some(r#"A duration in the humantime format. For example: '30s' for 30 seconds. '5m' for 5 minutes."#.to_string()),
        default: None,
        deprecated: false,
        read_only: false,
        write_only: false,
        examples: vec![json!("30s"), json!("1m")],
    }));
    schema.into()
}
