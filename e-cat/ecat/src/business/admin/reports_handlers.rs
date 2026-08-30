// open-travel admin-service：报表中心（销售日报 / 支付渠道聚合）
//
// 日期区间参数可选，缺省近 30 天（含今日）；日期序列在 Rust 侧补零生成，
// 区间切分与缺省日期均以 MySQL 服务器时区为准（同 stats TREND，不做
// Rust 侧时区转换）。金额单位 cents，SUM 结果 SIGNED CAST（同 stats）。
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ecat_data::{Row, RdbmsClient};
use ecat_data_sqlx::SqlxClient;
use serde::Deserialize;
use serde_json::{json, Value};

use super::handlers::db_unavailable;
use super::line_handlers::{col_u64, valid_date};
use super::{err, AdminGuard, ApiResponse, AppState};

#[derive(Deserialize)]
pub(crate) struct ReportQuery {
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
}

fn col_str(row: &Row, col: &str) -> String {
    row.get(col).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn days_in_month(y: u32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// 格式 + 真实日历校验（valid_date 仅查 YYYY-MM-DD 字符格式）
fn valid_ymd(s: &str) -> bool {
    if !valid_date(s) {
        return false;
    }
    let mut it = s.split('-');
    let (y, m, d) = (
        it.next().and_then(|v| v.parse().ok()).unwrap_or(0),
        it.next().and_then(|v| v.parse().ok()).unwrap_or(0),
        it.next().and_then(|v| v.parse().ok()).unwrap_or(0),
    );
    (1..=12).contains(&m) && (1..=days_in_month(y, m)).contains(&d)
}

/// YYYY-MM-DD 字符串 +1 天（合法日期下 ISO 字符串序即日期序）
fn next_day(s: &str) -> String {
    let mut it = s.split('-');
    let (y, m, d) = (
        it.next().and_then(|v| v.parse().ok()).unwrap_or(0),
        it.next().and_then(|v| v.parse().ok()).unwrap_or(0),
        it.next().and_then(|v| v.parse().ok()).unwrap_or(0),
    );
    if d < days_in_month(y, m) {
        format!("{y:04}-{m:02}-{:02}", d + 1)
    } else if m == 12 {
        format!("{:04}-01-01", y + 1)
    } else {
        format!("{y:04}-{:02}-01", m + 1)
    }
}

/// 解析并校验日期区间：缺省近 30 天（含今日），from/to 取 MySQL CURDATE。
async fn resolve_range(db: &SqlxClient, q: &ReportQuery) -> Result<(String, String), Response> {
    let (from, to) = match (&q.from, &q.to) {
        (Some(f), Some(t)) => (f.clone(), t.clone()),
        (f, t) => {
            // 只缺一侧时该侧取默认，提供的另一侧仍走下方统一校验
            let sql = "SELECT DATE_FORMAT(DATE_SUB(CURDATE(), INTERVAL 29 DAY), '%Y-%m-%d') AS from_day, \
                DATE_FORMAT(CURDATE(), '%Y-%m-%d') AS to_day";
            let rows = match db.query(sql).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(error = %e, "report range query failed");
                    return Err(db_unavailable());
                }
            };
            match rows.first() {
                Some(r) => (
                    f.clone().unwrap_or_else(|| col_str(r, "from_day")),
                    t.clone().unwrap_or_else(|| col_str(r, "to_day")),
                ),
                None => {
                    return Err(
                        err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error")
                            .into_response()
                    )
                }
            }
        }
    };
    if !valid_ymd(&from) || !valid_ymd(&to) || from > to {
        return Err(err::<Value>(
            StatusCode::BAD_REQUEST,
            400,
            "from/to must be YYYY-MM-DD with from <= to",
        )
        .into_response());
    }
    Ok((from, to))
}

/// 销售报表：按日聚合订单数 / 已支付订单数（status>=1）/ GMV（cents），无单日补零。
pub(crate) async fn sales_report(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Query(q): Query<ReportQuery>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let (from, to) = match resolve_range(&db, &q).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let sql = "SELECT DATE_FORMAT(created_at, '%Y-%m-%d') AS day, COUNT(*) AS orders, \
        CAST(COALESCE(SUM(CASE WHEN status >= 1 THEN 1 ELSE 0 END), 0) AS SIGNED) AS paid_orders, \
        CAST(COALESCE(SUM(CASE WHEN status >= 1 THEN amount_cents ELSE 0 END), 0) AS SIGNED) AS gmv_cents \
        FROM travel_orders WHERE created_at >= ? AND created_at < DATE_ADD(?, INTERVAL 1 DAY) \
        GROUP BY DATE_FORMAT(created_at, '%Y-%m-%d')";
    let rows = match db.query_with(sql, &[json!(from.clone()), json!(to.clone())]).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "sales report query failed");
            return db_unavailable();
        }
    };
    let mut by_day: std::collections::HashMap<String, (u64, u64, u64)> = std::collections::HashMap::new();
    for r in &rows {
        by_day.insert(
            col_str(r, "day"),
            (col_u64(r, "orders"), col_u64(r, "paid_orders"), col_u64(r, "gmv_cents")),
        );
    }
    // 区间内逐日补零；上限 366 天防超长区间生成超大响应
    let mut items = Vec::new();
    let mut cur = from.clone();
    let mut days = 0u32;
    loop {
        let (o, p, g) = by_day.get(&cur).copied().unwrap_or((0, 0, 0));
        items.push(json!({ "day": cur.clone(), "orders": o, "paid_orders": p, "gmv_cents": g }));
        if cur == to {
            break;
        }
        cur = next_day(&cur);
        days += 1;
        if days > 366 {
            return err::<Value>(StatusCode::BAD_REQUEST, 400, "date range too large (max 366 days)")
                .into_response();
        }
    }
    ApiResponse::ok(json!({ "from": from, "to": to, "items": items })).into_response()
}

/// 支付渠道报表：按渠道聚合流水数与金额（cents），按金额降序。
pub(crate) async fn payments_report(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Query(q): Query<ReportQuery>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let (from, to) = match resolve_range(&db, &q).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let sql = "SELECT channel_code, COUNT(*) AS count, \
        CAST(COALESCE(SUM(amount_cents), 0) AS SIGNED) AS amount_cents \
        FROM travel_payments WHERE created_at >= ? AND created_at < DATE_ADD(?, INTERVAL 1 DAY) \
        GROUP BY channel_code ORDER BY amount_cents DESC";
    let rows = match db.query_with(sql, &[json!(from.clone()), json!(to.clone())]).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "payments report query failed");
            return db_unavailable();
        }
    };
    let items: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "channel": col_str(r, "channel_code"),
                "count": col_u64(r, "count"),
                "amount_cents": col_u64(r, "amount_cents"),
            })
        })
        .collect();
    ApiResponse::ok(json!({ "from": from, "to": to, "items": items })).into_response()
}
