// open-travel admin-service：CDN 云服务商管理（配置 / 启停 / 命令预览）
//
// 安全边界：本服务不持有任何云凭据，接口仅管理配置并生成 dry-run 命令预览
// 文本，真实执行在部署机（需配置对应云 CLI 凭据）。
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use ecat_data::RdbmsClient;
use ecat_data_sqlx::SqlxClient;
use serde::Deserialize;
use serde_json::{json, Value};

use super::handlers::{db_unavailable, not_found};
use super::line_handlers::col_u64;
use super::{err, AdminGuard, ApiResponse, AppState};

// TINYINT 列 CAST AS SIGNED（sqlx Any 解码限制，同 payments_handlers）
const PROVIDER_SELECT: &str = "provider_code, name, CAST(enabled AS SIGNED) AS enabled, \
    bucket, region, domain, endpoint, CAST(updated_at AS CHAR) AS updated_at";

fn col_str(row: &ecat_data::Row, col: &str) -> String {
    row.get(col).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn provider_from_row(row: &ecat_data::Row) -> Value {
    json!({
        "provider_code": col_str(row, "provider_code"),
        "name": col_str(row, "name"),
        "enabled": col_u64(row, "enabled") == 1,
        "bucket": col_str(row, "bucket"),
        "region": col_str(row, "region"),
        "domain": col_str(row, "domain"),
        "endpoint": col_str(row, "endpoint"),
        "updated_at": col_str(row, "updated_at"),
    })
}

async fn fetch_provider(db: &SqlxClient, code: &str) -> Option<Value> {
    let rows = db
        .query_with(
            &format!("SELECT {PROVIDER_SELECT} FROM travel_cdn_providers WHERE provider_code = ?"),
            &[json!(code)],
        )
        .await
        .ok()?;
    rows.first().map(provider_from_row)
}

#[derive(Deserialize)]
pub(crate) struct EnabledReq {
    pub(crate) enabled: bool,
}

/// 全量列表：含禁用项，供管理端展示开关。
pub(crate) async fn list_providers(
    State(state): State<AppState>,
    _guard: AdminGuard,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let rows = match db
        .query(&format!("SELECT {PROVIDER_SELECT} FROM travel_cdn_providers ORDER BY provider_code ASC"))
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "cdn provider list query failed");
            return db_unavailable();
        }
    };
    let list: Vec<Value> = rows.iter().map(provider_from_row).collect();
    ApiResponse::ok(json!({ "items": list })).into_response()
}

/// 启停：enabled 0 停用 / 1 启用，code 不存在返回 404。
pub(crate) async fn update_provider_status(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(code): Path<String>,
    Json(body): Json<EnabledReq>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    match db
        .execute_with(
            "UPDATE travel_cdn_providers SET enabled = ? WHERE provider_code = ?",
            &[json!(if body.enabled { 1 } else { 0 }), json!(code)],
        )
        .await
    {
        Ok(0) => not_found("provider"),
        Ok(_) => match fetch_provider(&db, &code).await {
            Some(p) => (StatusCode::OK, ApiResponse::ok(p)).into_response(),
            None => db_unavailable(),
        },
        Err(e) => {
            tracing::warn!(error = %e, "cdn provider status update failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}

/// 更新配置：bucket/region/domain/endpoint 全字段可选，只更新提供的字段
/// （空串表示清空；region 不允许为空），code 不存在返回 404。
pub(crate) async fn update_provider(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(code): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Value> = Vec::new();
    for f in ["bucket", "region", "domain", "endpoint"] {
        if let Some(v) = obj.get(f) {
            if v.is_null() {
                continue;
            }
            let s = v.as_str().unwrap_or("").to_string();
            if f == "region" && s.is_empty() {
                return err::<Value>(StatusCode::BAD_REQUEST, 400, "region must not be empty").into_response();
            }
            sets.push(format!("{f} = ?"));
            params.push(json!(s));
        }
    }
    if sets.is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "no fields to update").into_response();
    }
    let Some(db) = state.db.clone() else { return db_unavailable() };
    params.push(json!(code));
    let sql = format!("UPDATE travel_cdn_providers SET {} WHERE provider_code = ?", sets.join(","));
    match db.execute_with(&sql, &params).await {
        Ok(0) => not_found("provider"),
        Ok(_) => match fetch_provider(&db, &code).await {
            Some(p) => (StatusCode::OK, ApiResponse::ok(p)).into_response(),
            None => db_unavailable(),
        },
        Err(e) => {
            tracing::warn!(error = %e, "cdn provider update failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}

/// 命令预览（dry-run）：由配置拼出部署机可执行的脚本命令，此处不执行。
/// bucket 未配置时提示先配置 bucket。
pub(crate) async fn provider_plan(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(code): Path<String>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let Some(p) = fetch_provider(&db, &code).await else {
        return not_found("provider");
    };
    let bucket = p["bucket"].as_str().unwrap_or("");
    if bucket.is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "configure bucket first").into_response();
    }
    let mut setup = format!(
        "cd /home/wwwroot/open-travel && ./scripts/cdn_setup.sh --provider {code} --bucket {bucket} --region {}",
        p["region"].as_str().unwrap_or("us-east-1")
    );
    for (flag, key) in [("--domain", "domain"), ("--endpoint", "endpoint")] {
        if let Some(v) = p[key].as_str().filter(|v| !v.is_empty()) {
            setup.push_str(&format!(" {flag} {v}"));
        }
    }
    let upload = format!(
        "cd /home/wwwroot/open-travel && ./scripts/cdn_upload.sh --provider {code} --bucket {bucket}"
    );
    ApiResponse::ok(json!({
        "provider_code": code,
        "commands": [setup, upload],
        "hint": "真实执行需在部署机配置对应云 CLI 凭据后运行"
    }))
    .into_response()
}
