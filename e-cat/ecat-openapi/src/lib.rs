// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: OpenApiInfo,
    pub paths: HashMap<String, PathItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Components>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenApiInfo {
    pub title: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Operation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Operation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<RequestBody>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub responses: HashMap<String, Response>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestBody {
    pub content: HashMap<String, MediaType>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<HashMap<String, MediaType>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaType {
    pub schema: Schema,
}

#[derive(Debug, Clone, Serialize)]
pub struct Schema {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Schema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Components {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemas: Option<HashMap<String, Schema>>,
}

pub struct OpenApiBuilder {
    title: String,
    version: String,
    paths: HashMap<String, PathItem>,
    schemas: HashMap<String, Schema>,
}

impl OpenApiBuilder {
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            version: version.into(),
            paths: HashMap::new(),
            schemas: HashMap::new(),
        }
    }

    pub fn add_route(
        mut self,
        path: impl Into<String>,
        method: &str,
        summary: impl Into<String>,
        tags: Vec<String>,
    ) -> Self {
        let entry = self.paths.entry(path.into()).or_insert(PathItem {
            get: None,
            post: None,
            put: None,
            delete: None,
            patch: None,
            head: None,
            options: None,
        });
        let op = Operation {
            summary: Some(summary.into()),
            tags,
            request_body: None,
            responses: {
                let mut m = HashMap::new();
                m.insert(
                    "200".into(),
                    Response {
                        description: "Successful response".into(),
                        content: None,
                    },
                );
                m
            },
        };
        match method {
            "GET" | "get" => entry.get = Some(op),
            "POST" | "post" => entry.post = Some(op),
            "PUT" | "put" => entry.put = Some(op),
            "DELETE" | "delete" => entry.delete = Some(op),
            "PATCH" | "patch" => entry.patch = Some(op),
            "HEAD" | "head" => entry.head = Some(op),
            "OPTIONS" | "options" => entry.options = Some(op),
            _ => {}
        }
        self
    }

    pub fn add_schema(
        mut self,
        name: impl Into<String>,
        properties: HashMap<String, Schema>,
    ) -> Self {
        self.schemas.insert(
            name.into(),
            Schema {
                schema_type: Some("object".into()),
                properties: Some(properties),
                reference: None,
            },
        );
        self
    }

    pub fn build(self) -> OpenApiSpec {
        let components = if self.schemas.is_empty() {
            None
        } else {
            Some(Components {
                schemas: Some(self.schemas),
            })
        };
        OpenApiSpec {
            openapi: "3.0.3".into(),
            info: OpenApiInfo {
                title: self.title,
                version: self.version,
            },
            paths: self.paths,
            components,
        }
    }
}

pub fn schema_ref(name: &str) -> Schema {
    Schema {
        schema_type: None,
        properties: None,
        reference: Some(format!("#/components/schemas/{name}")),
    }
}

pub fn string_schema() -> Schema {
    Schema {
        schema_type: Some("string".into()),
        properties: None,
        reference: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_minimal_spec() {
        let spec = OpenApiBuilder::new("My API", "1.0.0")
            .add_route("/health", "GET", "Health check", vec!["health".into()])
            .build();
        assert_eq!(spec.openapi, "3.0.3");
    }

    #[test]
    fn build_json_output() {
        let spec = OpenApiBuilder::new("Test", "1.0").build();
        let json = serde_json::to_string(&spec).unwrap();
        assert!(json.contains("\"openapi\""));
    }

    #[test]
    fn all_seven_methods_serialize() {
        let spec = OpenApiBuilder::new("M", "1")
            .add_route("/r", "GET", "g", vec![])
            .add_route("/r", "POST", "p", vec![])
            .add_route("/r", "PUT", "u", vec![])
            .add_route("/r", "DELETE", "d", vec![])
            .add_route("/r", "PATCH", "a", vec![])
            .add_route("/r", "HEAD", "h", vec![])
            .add_route("/r", "OPTIONS", "o", vec![])
            .build();
        let json = serde_json::to_value(&spec).unwrap();
        let item = json.pointer("/paths/~1r").unwrap().as_object().unwrap();
        for method in ["get", "post", "put", "delete", "patch", "head", "options"] {
            assert!(item.contains_key(method), "missing {method}");
        }
    }

    #[test]
    fn unknown_method_ignored() {
        let spec = OpenApiBuilder::new("M", "1")
            .add_route("/x", "TRACE", "t", vec![])
            .build();
        let json = serde_json::to_value(&spec).unwrap();
        let item = json.pointer("/paths/~1x").unwrap().as_object().unwrap();
        assert!(item.is_empty(), "unknown method must be ignored");
    }

    #[test]
    fn add_schema_emits_components() {
        let spec = OpenApiBuilder::new("T", "1")
            .add_schema("User", {
                let mut p = HashMap::new();
                p.insert("name".into(), string_schema());
                p
            })
            .build();
        let schemas = spec.components.expect("components present").schemas;
        assert!(schemas.unwrap().contains_key("User"));
    }

    #[test]
    fn build_without_schemas_omits_components() {
        let spec = OpenApiBuilder::new("T", "1").build();
        assert!(spec.components.is_none());
        let json = serde_json::to_string(&spec).unwrap();
        assert!(!json.contains("components"), "got: {json}");
    }

    #[test]
    fn schema_ref_points_to_components() {
        let s = schema_ref("User");
        assert_eq!(s.reference.as_deref(), Some("#/components/schemas/User"));
        assert_eq!(s.schema_type, None);
    }

    #[test]
    fn string_schema_has_type() {
        assert_eq!(string_schema().schema_type.as_deref(), Some("string"));
    }

    #[test]
    fn same_method_twice_overwrites() {
        let spec = OpenApiBuilder::new("T", "1")
            .add_route("/r", "GET", "first", vec![])
            .add_route("/r", "GET", "second", vec![])
            .build();
        let json = serde_json::to_value(&spec).unwrap();
        let item = json.pointer("/paths/~1r").unwrap().as_object().unwrap();
        assert_eq!(item.len(), 1, "only one method slot");
        assert_eq!(item["get"]["summary"], "second");
    }

    #[test]
    fn default_response_200_added() {
        let spec = OpenApiBuilder::new("T", "1")
            .add_route("/r", "POST", "s", vec![])
            .build();
        let json = serde_json::to_value(&spec).unwrap();
        assert!(json.pointer("/paths/~1r/post/responses/200").is_some());
    }

    #[test]
    fn tags_serialized() {
        let spec = OpenApiBuilder::new("T", "1")
            .add_route("/r", "GET", "s", vec!["a".into(), "b".into()])
            .build();
        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(
            json.pointer("/paths/~1r/get/tags").unwrap(),
            &serde_json::json!(["a", "b"])
        );
    }
}
