// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use axum::Router;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

mod parser;
mod validation;

pub use parser::{FieldNode, Operation, SelectionSet};
pub use validation::QueryLimits;

type Resolver = Arc<
    dyn Fn(
            serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>,
        > + Send
        + Sync,
>;

/// 富 resolver 请求上下文：字段参数、原始 variables 与嵌套 selection 树。
#[derive(Debug, Clone)]
pub struct FieldRequest {
    pub args: Map<String, Value>,
    pub variables: Value,
    pub selection: Option<SelectionSet>,
}

/// 富 resolver trait：可访问字段参数与嵌套 selection（经
/// [`GraphQLSchema::query_field`] / [`GraphQLSchema::mutation_field`] 注册）。
/// async fn in trait 非 dyn-compatible，故用 #[async_trait] 换取对象安全
/// （可存入 `FieldHandler::Rich` 的 `Arc<dyn GraphQLField>`）。
#[async_trait::async_trait]
pub trait GraphQLField: Send + Sync {
    async fn resolve(&self, req: FieldRequest) -> Result<Value, String>;
}

enum FieldHandler {
    Legacy(Resolver),
    Rich(Arc<dyn GraphQLField>),
}

pub struct GraphQLSchema {
    query_resolvers: HashMap<String, FieldHandler>,
    mutation_resolvers: HashMap<String, FieldHandler>,
    limits: QueryLimits,
}

impl GraphQLSchema {
    pub fn new() -> Self {
        Self {
            query_resolvers: HashMap::new(),
            mutation_resolvers: HashMap::new(),
            limits: QueryLimits::default(),
        }
    }

    /// 配置执行前查询预算（防查询放大 DoS），默认已启用。
    pub fn with_limits(mut self, limits: QueryLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn query(mut self, name: impl Into<String>, r: Resolver) -> Self {
        self.query_resolvers
            .insert(name.into(), FieldHandler::Legacy(r));
        self
    }

    pub fn mutation(mut self, name: impl Into<String>, r: Resolver) -> Self {
        self.mutation_resolvers
            .insert(name.into(), FieldHandler::Legacy(r));
        self
    }

    pub fn query_fn(
        self,
        name: impl Into<String>,
        f: impl Fn(
            serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>,
        > + Send
        + Sync
        + 'static,
    ) -> Self {
        self.query(name, Arc::new(f))
    }

    /// 注册富 query resolver：接收 `FieldRequest`（参数 + 嵌套 selection）。
    pub fn query_field(mut self, name: impl Into<String>, f: impl GraphQLField + 'static) -> Self {
        self.query_resolvers
            .insert(name.into(), FieldHandler::Rich(Arc::new(f)));
        self
    }

    /// 注册富 mutation resolver：接收 `FieldRequest`（参数 + 嵌套 selection）。
    pub fn mutation_field(
        mut self,
        name: impl Into<String>,
        f: impl GraphQLField + 'static,
    ) -> Self {
        self.mutation_resolvers
            .insert(name.into(), FieldHandler::Rich(Arc::new(f)));
        self
    }
}

impl Default for GraphQLSchema {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct GqlReq {
    query: String,
    #[serde(default)]
    variables: serde_json::Value,
}

pub fn graphql_router(schema: GraphQLSchema) -> Router {
    let schema = Arc::new(schema);

    async fn handler(axum::Json(req): axum::Json<GqlReq>, schema: Arc<GraphQLSchema>) -> Response {
        match execute(&schema, &req.query, &req.variables).await {
            Ok(data) => axum::Json(serde_json::json!({"data": data})).into_response(),
            Err(errors) => (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({"errors": errors})),
            )
                .into_response(),
        }
    }

    let s = Arc::clone(&schema);
    Router::new().route("/graphql", post(move |body| handler(body, Arc::clone(&s))))
}

async fn execute(
    schema: &GraphQLSchema,
    query: &str,
    variables: &serde_json::Value,
) -> Result<serde_json::Value, Vec<String>> {
    let trimmed = query.trim();
    let mut errors = Vec::new();

    let field = match parser::parse_query(trimmed, variables) {
        Ok(f) => f,
        Err(e) => {
            errors.push(e);
            return Err(errors);
        }
    };

    if let Err(e) = validation::validate(&field, &schema.limits) {
        errors.push(e);
        return Err(errors);
    }

    let (resolvers, field_name) = if field.operation == Operation::Mutation {
        (&schema.mutation_resolvers, &field.name)
    } else {
        (&schema.query_resolvers, &field.name)
    };

    match resolvers.get(field_name) {
        Some(FieldHandler::Legacy(resolver)) => {
            let vars = match merge_args(variables, &field.args) {
                Ok(v) => v,
                Err(e) => {
                    errors.push(e);
                    return Err(errors);
                }
            };
            match resolver(vars).await {
                Ok(data) => {
                    let mut result = serde_json::Map::new();
                    result.insert(field.name.clone(), data);
                    return Ok(serde_json::Value::Object(result));
                }
                Err(e) => errors.push(e),
            }
        }
        Some(FieldHandler::Rich(r)) => {
            let req = FieldRequest {
                args: field.args,
                variables: variables.clone(),
                selection: field.selection,
            };
            match r.resolve(req).await {
                Ok(data) => {
                    let mut result = serde_json::Map::new();
                    result.insert(field.name.clone(), data);
                    return Ok(serde_json::Value::Object(result));
                }
                Err(e) => errors.push(e),
            }
        }
        None => errors.push(format!("unknown field: {field_name}")),
    }

    Err(errors)
}

/// Legacy resolver 的参数合并：字段参数并入 variables（同名时参数胜出）。
/// 无参数时与旧行为逐字节一致（直接克隆 variables）。
fn merge_args(variables: &Value, args: &Map<String, Value>) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(variables.clone());
    }
    match variables {
        Value::Object(m) => {
            let mut merged = m.clone();
            for (k, v) in args {
                merged.insert(k.clone(), v.clone());
            }
            Ok(Value::Object(merged))
        }
        Value::Null => Ok(Value::Object(args.clone())),
        _ => Err("variables must be a JSON object when field arguments are present".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    struct EchoField;

    #[async_trait::async_trait]
    impl GraphQLField for EchoField {
        async fn resolve(&self, req: FieldRequest) -> Result<Value, String> {
            Ok(serde_json::json!({
                "args": req.args,
                "variables": req.variables,
                "has_selection": req.selection.is_some(),
            }))
        }
    }

    #[test]
    fn legacy_resolver_receives_merged_args() {
        let schema =
            GraphQLSchema::new().query_fn("echo", |vars| Box::pin(async move { Ok(vars) }));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = serde_json::json!({"id": 1});
        let data = rt
            .block_on(execute(&schema, "{ echo(id: 2, name: \"x\") }", &vars))
            .unwrap();
        assert_eq!(data["echo"], serde_json::json!({"id": 2, "name": "x"}));
    }

    #[test]
    fn legacy_resolver_without_args_is_unchanged() {
        let schema =
            GraphQLSchema::new().query_fn("echo", |vars| Box::pin(async move { Ok(vars) }));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = serde_json::json!({"a": 1});
        let data = rt.block_on(execute(&schema, "{ echo }", &vars)).unwrap();
        assert_eq!(data["echo"], serde_json::json!({"a": 1}));
    }

    #[test]
    fn legacy_resolver_args_override_same_named_variables() {
        let schema =
            GraphQLSchema::new().query_fn("echo", |vars| Box::pin(async move { Ok(vars) }));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = serde_json::json!({"id": 1});
        let data = rt
            .block_on(execute(&schema, "{ echo(id: 9) }", &vars))
            .unwrap();
        assert_eq!(data["echo"]["id"], 9);
    }

    #[test]
    fn legacy_resolver_args_with_null_variables() {
        let schema =
            GraphQLSchema::new().query_fn("echo", |vars| Box::pin(async move { Ok(vars) }));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let data = rt
            .block_on(execute(&schema, "{ echo(id: 1) }", &Value::Null))
            .unwrap();
        assert_eq!(data["echo"], serde_json::json!({"id": 1}));
    }

    #[test]
    fn legacy_resolver_errors_on_non_object_variables_with_args() {
        let schema =
            GraphQLSchema::new().query_fn("echo", |vars| Box::pin(async move { Ok(vars) }));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(execute(&schema, "{ echo(id: 1) }", &serde_json::json!(5)))
            .unwrap_err();
        assert!(
            err.iter()
                .any(|e| e.contains("variables must be a JSON object")),
            "got: {err:?}"
        );
    }

    #[test]
    fn rich_resolver_receives_full_request() {
        let schema = GraphQLSchema::new().query_field("user", EchoField);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = serde_json::json!({"env": "prod"});
        let data = rt
            .block_on(execute(&schema, "{ user(id: 7) { name } }", &vars))
            .unwrap();
        // Rich resolver 收到原样 variables 与解析后的 args，二者不合并
        assert_eq!(data["user"]["args"]["id"], 7);
        assert_eq!(
            data["user"]["variables"],
            serde_json::json!({"env": "prod"})
        );
        assert_eq!(data["user"]["has_selection"], true);
    }

    #[test]
    fn rich_resolver_without_selection() {
        let schema = GraphQLSchema::new().query_field("ping", EchoField);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let data = rt
            .block_on(execute(&schema, "{ ping }", &Value::Null))
            .unwrap();
        assert_eq!(data["ping"]["has_selection"], false);
        assert!(data["ping"]["args"].as_object().unwrap().is_empty());
    }

    #[test]
    fn unknown_field_and_resolver_error_go_to_errors() {
        let schema = GraphQLSchema::new().query_field("boom", ResolveErr);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(execute(&schema, "{ nope }", &Value::Null))
            .unwrap_err();
        assert!(err.iter().any(|e| e.contains("unknown field: nope")));

        let err = rt
            .block_on(execute(&schema, "{ boom }", &Value::Null))
            .unwrap_err();
        assert!(err.iter().any(|e| e == "resolver exploded"));
    }

    struct ResolveErr;

    #[async_trait::async_trait]
    impl GraphQLField for ResolveErr {
        async fn resolve(&self, _req: FieldRequest) -> Result<Value, String> {
            Err("resolver exploded".into())
        }
    }

    #[test]
    fn mutation_dispatches_to_mutation_resolvers() {
        let schema = GraphQLSchema::new().query_fn("write", |_v| {
            Box::pin(async { Ok(serde_json::json!("query")) })
        });
        let schema = schema.mutation_field("write", MutWrite);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let data = rt
            .block_on(execute(&schema, "mutation { write(id: 3) }", &Value::Null))
            .unwrap();
        assert_eq!(data["write"]["id"], 3);
    }

    struct MutWrite;

    #[async_trait::async_trait]
    impl GraphQLField for MutWrite {
        async fn resolve(&self, req: FieldRequest) -> Result<Value, String> {
            Ok(serde_json::json!({"id": req.args["id"]}))
        }
    }

    #[test]
    fn subscription_dispatches_to_query_resolvers() {
        let schema = GraphQLSchema::new().query_fn("sub", |_v| {
            Box::pin(async { Ok(serde_json::json!("query")) })
        });
        let schema = schema.mutation_field("sub", MutWrite);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let data = rt
            .block_on(execute(&schema, "subscription { sub }", &Value::Null))
            .unwrap();
        assert_eq!(data["sub"], "query");
    }

    fn router() -> Router {
        graphql_router(
            GraphQLSchema::new()
                .query_fn("hello", |_v| {
                    Box::pin(async { Ok(serde_json::json!("world")) })
                })
                .query_field("user", EchoField),
        )
    }

    #[tokio::test]
    async fn router_serves_simple_query() {
        let res = router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/graphql")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"query":"{ hello }"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["data"]["hello"], "world");
    }

    #[tokio::test]
    async fn router_serves_query_with_args_and_nested_selection() {
        let res = router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/graphql")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"query":"{ user(id: 7, env: $e) { name } }","variables":{"e":"prod"}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["data"]["user"]["args"]["id"], 7);
        assert_eq!(v["data"]["user"]["args"]["env"], "prod");
        assert_eq!(
            v["data"]["user"]["variables"],
            serde_json::json!({"e": "prod"})
        );
        assert_eq!(v["data"]["user"]["has_selection"], true);
    }

    #[tokio::test]
    async fn router_returns_400_with_errors() {
        let res = router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/graphql")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"query":"{ nope }"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert!(v["errors"][0].as_str().unwrap().contains("unknown field"));
    }

    #[tokio::test]
    async fn router_returns_400_on_parse_error() {
        let res = router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/graphql")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"query":"{ a b }"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert!(
            v["errors"][0]
                .as_str()
                .unwrap()
                .contains("multiple top-level fields")
        );
    }

    #[test]
    fn schema_field_name_conflict_latest_wins() {
        let schema = GraphQLSchema::new()
            .query_fn("f", |_v| {
                Box::pin(async { Ok(serde_json::json!("legacy")) })
            })
            .query_field("f", EchoField);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let data = rt
            .block_on(execute(&schema, "{ f(a: 1) }", &Value::Null))
            .unwrap();
        assert_eq!(data["f"]["args"]["a"], 1);
    }
}
