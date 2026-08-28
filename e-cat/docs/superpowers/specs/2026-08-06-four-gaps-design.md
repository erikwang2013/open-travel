# 四项缺口补全设计 — metrics 端点 / openapi 方法 / config-remote watch / ClickHouse TsdbClient

日期：2026-08-06
状态：已批准（用户确认「可以」）

## 背景

项目确认存在四项能力缺口：

1. `ecat-metrics` 无自动 HTTP 端点（仅 `registry()` + `metrics_text()` 两个纯函数）
2. `ecat-openapi` 仅支持 GET/POST（`PathItem` 只有 get/post 字段，`add_route` 其余方法静默忽略）
3. `ecat-config-remote` 无 watch（仅 Consul KV 一次性 `load()`）
4. `ecat-data-clickhouse` 的 `ClickhouseClient` 只实现 `RdbmsClient`，未实现 `TsdbClient`

## 决策（用户确认）

| 项 | 决策 |
|---|---|
| metrics 端点 | `HttpServer` 自动挂载 `/metrics` |
| ClickHouse 建表 | `write()` 自动建表（OnceLock 缓存），MergeTree + tags String + fields 类型化 + timestamp Int64 纳秒 |
| watch 机制 | Consul 阻塞查询（index + wait，X-Consul-Index 续期），mpsc channel 输出 |
| openapi 方法 | PUT/DELETE/PATCH/HEAD/OPTIONS 全补，未知方法保持静默忽略 |

## 1. Metrics 自动端点 — ecat-metrics + ecat-transport-http

- `ecat-metrics` 新增 `axum` 依赖（workspace 0.8），提供：

  ```rust
  pub fn metrics_router() -> Router
  ```

  路由 `/metrics`，GET handler 返回 `metrics_text()`，Content-Type 为
  `text/plain; version=0.0.4; charset=utf-8`（Prometheus 文本格式）。

- `ecat-transport-http` 新增 `ecat-metrics` path 依赖，`HttpServer::start()`
  中 `router.merge(metrics_router())` 自动挂载。
- 注意：axum `merge` 对重复路径 panic，`/metrics` 成为框架保留路径，在
  `HttpServer` 文档注释中注明。
- 测试：`tower::ServiceExt::oneshot` 验证 `/metrics` 返回 200 与文本内容。

## 2. OpenAPI 全方法 — ecat-openapi

- `PathItem` 增加 `put` / `delete` / `patch` / `head` / `options` 五个
  `Option<Operation>` 字段，均带 `skip_serializing_if = "Option::is_none"`。
- `add_route` 的 match 扩展 5 个新方法分支（大小写均匹配，与现有 GET/POST 一致）；
  未知方法保留 `_ => {}` 静默忽略。
- 测试：7 种方法全序列化断言 + 未知方法被忽略（不 panic、不产生字段）。

## 3. config-remote watch — ecat-config-remote

- 内部重构：抽出

  ```rust
  async fn fetch(&self, index: Option<&str>)
      -> Result<(HashMap<String, serde_json::Value>, Option<String>), ConfigError>
  ```

  返回 (map, 新 X-Consul-Index)。`load()` 改为 `fetch(None).await.map(|(m, _)| m)`。

- 新增：

  ```rust
  pub fn watch(&self) -> mpsc::Receiver<Result<HashMap<String, serde_json::Value>, ConfigError>>
  ```

  行为：
  - `tokio::spawn` 后台任务，`GET /v1/kv/<prefix>?recurse=true&index=<last>&wait=5m`
    长轮询；
  - X-Consul-Index 变化才推送（首帧立即推送）；同 index 去重；
  - 请求级 `.timeout(Duration::from_secs(330))`（略大于 wait=5m）；
  - 出错：推送 `Err` 后延时 1s 重试。
- 测试：dev-dep 加 `axum` 起本地 mock Consul（KV entries + X-Consul-Index 头），
  验证首帧推送、变更推送、同 index 去重。

## 4. ClickHouse TsdbClient — ecat-data-clickhouse

- 轻重构：抽出私有 `fn post(&self, sql, params) -> reqwest::RequestBuilder`，
  消除 `execute` / `query` 的重复 POST 构建。
- 新增 `impl TsdbClient for ClickhouseClient`：

  - **write**：
    - 按 measurement 分组；
    - 每组 `CREATE TABLE IF NOT EXISTS <measurement>`：
      MergeTree 引擎；tag 键 → `String` 列；field 键 → 按值类型
      `Float64` / `Int64` / `String` / `UInt8`；`timestamp Int64 DEFAULT 0`（纳秒）；
    - 建表结果以 `OnceLock<HashMap<String, ()>>` 缓存（每 measurement 只建一次，
      CREATE IF NOT EXISTS 幂等，并发安全）；
    - `INSERT INTO <measurement> (cols) FORMAT JSONEachRow`，每点一个 JSON 对象；
      timestamp 为 None 时省略该键（落到 DEFAULT 0）；
    - 标识符（表名/列名）反引号转义；值经 serde_json 序列化（防注入）。
  - **query**：内部直接解析 JSONEachRow 返回 `serde_json::json!([...])`
    行对象数组（不依赖 `ecat-data` 的 `Row` 私有字段）。
  - **delete**：原样执行传入 SQL（ClickHouse 轻量删除语法为
    `ALTER TABLE x DELETE WHERE ...`，文档注明）。
- 纯函数抽取便于测试：`build_create_table(...)`、`build_insert_body(points)`、
  标识符转义函数。
- 测试：建表 SQL 生成、标识符转义、JSONEachRow 序列化（含注入字符）、
  客户端构造。

## 依赖变更

| crate | 变更 |
|---|---|
| ecat-metrics | + axum |
| ecat-transport-http | + ecat-metrics (path) |
| ecat-config-remote | + axum (dev-dep，mock Consul 测试用) |
| 其他 | 无 |

无循环依赖：ecat-metrics 仅依赖 prometheus；ecat-transport-http → ecat-metrics
不构成环。

## 实施顺序

1. ecat-openapi（独立，最小）
2. ecat-metrics + ecat-transport-http（自动端点）
3. ecat-config-remote（fetch 重构 + watch）
4. ecat-data-clickhouse（TsdbClient）

每步 `cargo build + cargo test` 验证工作区可编译。

## 风险与边界

- `/metrics` 为保留路径，用户自定义 router 含 `/metrics` 时 merge panic（文档注明）。
- ClickHouse 表 schema 由首批写入的字段推导；后续新字段写入会失败
  （ClickHouse 无 ALTER 自动加列），文档注明。
- Consul watch 任务随 receiver drop 退出（发送失败即终止循环）。
