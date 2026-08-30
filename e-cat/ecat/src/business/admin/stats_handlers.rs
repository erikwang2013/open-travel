// open-travel admin-service：数据看板统计（订单总览 / Top 目的地与线路 / 7 天趋势）
//
// 全部聚合在 MySQL 内完成；SUM 结果按 SIGNED CAST（MySQL SUM 返回 DECIMAL，
// sqlx Any 驱动无法解码）；无数据时 Top 列表返回空数组、趋势恒为 7 天补零。
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use ecat_data::RdbmsClient;
use serde_json::{json, Value};

use super::handlers::db_unavailable;
use super::line_handlers::col_u64;
use super::{err, AdminGuard, ApiResponse, AppState};

/// 7 天日期序列（当日含）由 MySQL 生成，LEFT JOIN 补零，避免 Rust 侧时区问题
const TREND_SQL: &str = "SELECT d.day, COALESCE(g.cnt, 0) AS cnt FROM ( \
    SELECT DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 6 DAY), '%Y-%m-%d') AS day \
    UNION ALL SELECT DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 5 DAY), '%Y-%m-%d') \
    UNION ALL SELECT DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 4 DAY), '%Y-%m-%d') \
    UNION ALL SELECT DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 3 DAY), '%Y-%m-%d') \
    UNION ALL SELECT DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 2 DAY), '%Y-%m-%d') \
    UNION ALL SELECT DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 1 DAY), '%Y-%m-%d') \
    UNION ALL SELECT DATE_FORMAT(CURDATE(), '%Y-%m-%d') \
) d LEFT JOIN ( \
    SELECT DATE_FORMAT(created_at, '%Y-%m-%d') AS day, COUNT(*) AS cnt \
    FROM travel_orders WHERE created_at >= DATE_SUB(CURDATE(), INTERVAL 6 DAY) \
    GROUP BY DATE_FORMAT(created_at, '%Y-%m-%d') \
) g ON g.day = d.day ORDER BY d.day";

/// 总览：订单数、GMV（status>=1）、转化率、各状态订单数。金额单位 cents。
pub(crate) async fn overview(
    State(state): State<AppState>,
    _guard: AdminGuard,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let sql = "SELECT COUNT(*) AS total, \
        CAST(COALESCE(SUM(CASE WHEN status >= 1 THEN 1 ELSE 0 END), 0) AS SIGNED) AS paid_count, \
        CAST(COALESCE(SUM(CASE WHEN status >= 1 THEN amount_cents ELSE 0 END), 0) AS SIGNED) AS gmv_cents, \
        CAST(COALESCE(SUM(status = 0), 0) AS SIGNED) AS s0, \
        CAST(COALESCE(SUM(status = 1), 0) AS SIGNED) AS s1, \
        CAST(COALESCE(SUM(status = 2), 0) AS SIGNED) AS s2, \
        CAST(COALESCE(SUM(status = 3), 0) AS SIGNED) AS s3, \
        CAST(COALESCE(SUM(status = 4), 0) AS SIGNED) AS s4 \
        FROM travel_orders";
    let rows = match db.query_with(sql, &[]).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "stats overview query failed");
            return db_unavailable();
        }
    };
    let Some(row) = rows.first() else {
        return err::<Value>(axum::http::StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error")
            .into_response();
    };
    let total = col_u64(row, "total");
    let paid = col_u64(row, "paid_count");
    let conversion = if total == 0 { 0.0 } else { paid as f64 / total as f64 * 100.0 };
    ApiResponse::ok(json!({
        "total_orders": total,
        "paid_orders": paid,
        "gmv_cents": col_u64(row, "gmv_cents"),
        "conversion_rate": (conversion * 100.0).round() / 100.0,
        "status_counts": {
            "0": col_u64(row, "s0"), "1": col_u64(row, "s1"),
            "2": col_u64(row, "s2"), "3": col_u64(row, "s3"), "4": col_u64(row, "s4"),
        },
    }))
    .into_response()
}

/// Top 5 目的地 / Top 5 线路（按线路订单量），名称取中文优先。
pub(crate) async fn top(
    State(state): State<AppState>,
    _guard: AdminGuard,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let dest_sql = "SELECT d.id, d.name_zh, d.name_en, COUNT(*) AS cnt \
        FROM travel_orders o \
        JOIN travel_lines l ON l.id = o.product_id \
        JOIN travel_destinations d ON d.id = l.destination_id \
        WHERE o.order_type = 1 GROUP BY d.id, d.name_zh, d.name_en \
        ORDER BY cnt DESC LIMIT 5";
    let line_sql = "SELECT o.product_id AS id, l.title_zh, l.title_en, COUNT(*) AS cnt \
        FROM travel_orders o JOIN travel_lines l ON l.id = o.product_id \
        WHERE o.order_type = 1 GROUP BY o.product_id, l.title_zh, l.title_en \
        ORDER BY cnt DESC LIMIT 5";
    let dest_rows = match db.query_with(dest_sql, &[]).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "stats top destinations query failed");
            return db_unavailable();
        }
    };
    let line_rows = match db.query_with(line_sql, &[]).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "stats top lines query failed");
            return db_unavailable();
        }
    };
    let destinations: Vec<Value> = dest_rows
        .iter()
        .map(|r| {
            let zh = r.get("name_zh").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let en = r.get("name_en").and_then(|v| v.as_str()).unwrap_or("").to_string();
            json!({
                "id": col_u64(r, "id"),
                "name": if zh.is_empty() { en } else { zh },
                "orders": col_u64(r, "cnt"),
            })
        })
        .collect();
    let lines: Vec<Value> = line_rows
        .iter()
        .map(|r| {
            let zh = r.get("title_zh").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let en = r.get("title_en").and_then(|v| v.as_str()).unwrap_or("").to_string();
            json!({
                "id": col_u64(r, "id"),
                "name": if zh.is_empty() { en } else { zh },
                "orders": col_u64(r, "cnt"),
            })
        })
        .collect();
    ApiResponse::ok(json!({ "top_destinations": destinations, "top_lines": lines }))
        .into_response()
}

/// 近 7 天订单趋势：恒定 7 行（无单日补零），day 为 YYYY-MM-DD。
pub(crate) async fn trend(
    State(state): State<AppState>,
    _guard: AdminGuard,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let rows = match db.query_with(TREND_SQL, &[]).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "stats trend query failed");
            return db_unavailable();
        }
    };
    let items: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "day": r.get("day").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                "orders": col_u64(r, "cnt"),
            })
        })
        .collect();
    ApiResponse::ok(json!({ "items": items })).into_response()
}
