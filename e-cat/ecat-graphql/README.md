# ecat-graphql

GraphQL integration for e-cat.

Part of the [e-cat](https://github.com/erik/e-cat) ecosystem.

## 用法 (Usage)

```rust
use ecat_graphql::{graphql_router, GraphQLSchema, GraphQLField, FieldRequest};

// Legacy resolver：收到合并后的参数（字段参数覆盖同名 variables）
let schema = GraphQLSchema::new()
    .query_fn("hello", |_v| Box::pin(async { Ok(serde_json::json!("world")) }))
    .query_field("user", UserField); // 富 resolver：参数 + 嵌套 selection

impl GraphQLField for UserField {
    async fn resolve(&self, req: FieldRequest) -> Result<serde_json::Value, String> {
        // req.args —— 字段参数；req.variables —— 原始 variables；
        // req.selection —— 嵌套 selection 树（HashMap<String, FieldNode>）
        Ok(serde_json::json!({"id": req.args["id"]}))
    }
}
```

注册方式：

| 方法 | resolver 签名 | 可见信息 |
|------|---------------|----------|
| `query` / `mutation` | `Fn(Value) -> Future<Value>` | 合并后的参数（与旧行为逐字节一致） |
| `query_fn` / `mutation_fn` | 同上（闭包糖） | 同上 |
| `query_field` / `mutation_field` | `impl GraphQLField` | 原始 `args` + `variables` + 嵌套 `selection` |

## 限制 (Limitations)

手写解析器，覆盖字段参数与嵌套 selection，其余按显式错误拒绝：

- 不支持别名 (aliases)、fragment（含 spread）、block string 与多顶层字段；
- 输入对象键不支持引号（GraphQL 规范键为裸标识符）；
- 操作级变量定义 (`query ($v: Int = 3)`) 与字段指令 (`@skip`) 被跳过；
- mutation 按顶层字段 dispatch 到 mutation resolvers；
- 请勿在生产服务中将其暴露为通用 GraphQL 端点，如需完整功能请接入成熟 GraphQL 引擎（如 async-graphql / juniper）。
