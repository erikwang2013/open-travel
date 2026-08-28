use super::*;

#[test]
fn parses_simple_field() {
    let f = parse_query("{ hello }", &serde_json::Value::Null).unwrap();
    assert_eq!(f.name, "hello");
    assert!(f.args.is_empty());
    assert!(f.selection.is_none());
    assert_eq!(
        parse_query("query { hello }", &serde_json::Value::Null)
            .unwrap()
            .name,
        "hello"
    );
    assert_eq!(
        parse_query("mutation { x }", &serde_json::Value::Null)
            .unwrap()
            .name,
        "x"
    );
}

#[test]
fn operation_keyword_is_parsed() {
    assert_eq!(
        parse_query("{ hello }", &serde_json::Value::Null)
            .unwrap()
            .operation,
        Operation::Query
    );
    assert_eq!(
        parse_query("query Hello { hello }", &serde_json::Value::Null)
            .unwrap()
            .operation,
        Operation::Query
    );
    assert_eq!(
        parse_query("mutation { x }", &serde_json::Value::Null)
            .unwrap()
            .operation,
        Operation::Mutation
    );
    assert_eq!(
        parse_query("subscription { s }", &serde_json::Value::Null)
            .unwrap()
            .operation,
        Operation::Subscription
    );
    // 嵌套字段不携带 operation（转 FieldNode 时丢弃），无歧义
    let f = parse_query("mutation { write { ok } }", &serde_json::Value::Null).unwrap();
    assert_eq!(f.operation, Operation::Mutation);
    assert!(f.selection.as_ref().unwrap().contains_key("ok"));
}

#[test]
fn parses_all_literal_arg_types() {
    let f = parse_query(
        r#"{ f(s: "str", n: 1.5, b: true, z: null, e: RED, l: [1, "two", null], o: {k: "v", n: 2}) }"#,
        &serde_json::Value::Null,
    )
    .unwrap();
    assert_eq!(f.args["s"], "str");
    assert_eq!(f.args["n"], serde_json::json!(1.5));
    assert_eq!(f.args["b"], serde_json::json!(true));
    assert_eq!(f.args["z"], serde_json::Value::Null);
    // 无类型系统：枚举映射为字符串
    assert_eq!(f.args["e"], "RED");
    assert_eq!(f.args["l"], serde_json::json!([1, "two", null]));
    assert_eq!(f.args["o"], serde_json::json!({"k": "v", "n": 2}));
}

#[test]
fn parses_args_with_braces_and_escapes_in_strings() {
    let f = parse_query(
        r#"{ f(a: "} ) {", b: "(\"x\")") }"#,
        &serde_json::Value::Null,
    )
    .unwrap();
    assert_eq!(f.args["a"], "} ) {");
    assert_eq!(f.args["b"], "(\"x\")");
}

#[test]
fn parses_single_quoted_string_with_escapes() {
    let f = parse_query(r#"{ f(s: 'it\'s') }"#, &serde_json::Value::Null).unwrap();
    assert_eq!(f.args["s"], "it's");
}

#[test]
fn resolves_variables() {
    let vars = serde_json::json!({"id": 42, "name": "erik"});
    parse_query("{ user(id: $id) { name } }", &vars).unwrap();
    let mut i = 0;
    let v = parse_value(r#"{id: $id}"#.as_bytes(), &mut i, &vars, 0).unwrap();
    assert_eq!(v["id"], 42);
}

#[test]
fn variable_missing_is_error() {
    let mut i = 0;
    let err = parse_value(b"$nope", &mut i, &serde_json::Value::Null, 0).unwrap_err();
    assert!(
        err.contains("variable 'nope' is not provided"),
        "got: {err}"
    );
}

#[test]
fn parses_nested_selection_tree_with_args() {
    let f = parse_query(
        "{ user { posts(limit: 5) { title } } }",
        &serde_json::Value::Null,
    )
    .unwrap();
    // selection 是顶层字段的嵌套子节点集合（非顶层字段自身）
    assert_eq!(f.name, "user");
    let posts = f.selection.as_ref().unwrap().get("posts").unwrap();
    assert_eq!(posts.args["limit"], 5);
    let title = posts.selection.as_ref().unwrap().get("title").unwrap();
    assert!(title.args.is_empty());
    assert!(title.selection.is_none());
}

#[test]
fn nested_args_in_selection() {
    let f = parse_query(
        "{ user(id: 1) { posts(limit: 5) { title } } }",
        &serde_json::Value::Null,
    )
    .unwrap();
    assert_eq!(f.args["id"], 1);
    assert_eq!(f.selection.as_ref().unwrap()["posts"].args["limit"], 5);
}

#[test]
fn errors_are_explicit() {
    let cases = [
        ("{ a b }", "multiple top-level fields"),
        ("{ f(a: 1, a: 2) }", "duplicate argument 'a'"),
        ("{ a: b }", "aliases not supported"),
        ("{ ...frag }", "fragment spreads"),
        ("{ }", "empty selection set"),
        ("{ f( }", "unbalanced parentheses"),
        ("{ f(s: \"\"\"x\"\"\") }", "block string"),
        ("{ f(s: '''x''') }", "block string"),
        ("", "expected '{'"),
        ("query", "expected '{'"),
        ("{ f(v: $missing) }", "variable 'missing' is not provided"),
    ];
    for (q, want) in cases {
        let err = parse_query(q, &serde_json::Value::Null).unwrap_err();
        assert!(err.contains(want), "query {q}: got {err}");
    }
}

#[test]
fn variables_must_be_object_when_args_present_in_legacy() {
    // execute 层 legacy 合并的报错；此处锁定 parse 侧不受影响
    let f = parse_query("{ f(a: 1) }", &serde_json::Value::Null).unwrap();
    assert_eq!(f.args["a"], 1);
}

#[test]
fn deep_selection_is_rejected() {
    let deep = "{ a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a } } } } } } } } } } } } } } } } } } } } } } } } } } } } } } } } } } }";
    let err = parse_query(deep, &serde_json::Value::Null).unwrap_err();
    assert!(err.contains("selection too deep"), "got: {err}");
}

#[test]
fn field_directives_are_skipped() {
    let f = parse_query("{ hello @skip(if: true) }", &serde_json::Value::Null).unwrap();
    assert_eq!(f.name, "hello");
    assert!(f.args.is_empty());
}

#[test]
fn operation_variable_definitions_with_defaults_are_skipped() {
    let f = parse_query("query ($v: Int = 3) { hello }", &serde_json::Value::Null).unwrap();
    assert_eq!(f.name, "hello");
}
