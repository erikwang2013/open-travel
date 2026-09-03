// payment-service 业务 handlers：发起支付 / 模拟收银台 / 渠道回调 / 流水列表 / 渠道列表。
use super::*;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Html;
use axum::Json;
use ecat_data::RdbmsClient;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;
use std::sync::atomic::{AtomicU64, Ordering};

// DATETIME/TINYINT 列 CAST AS CHAR：sqlx Any 对 MySQL 时间类型及 TINYINT 解码失败
const PAYMENT_COLS: &str = "id, order_id, channel_code, amount_cents, \
    CAST(status AS CHAR) AS status, txn_no, \
    CAST(paid_at AS CHAR) AS paid_at, CAST(created_at AS CHAR) AS created_at";

fn col_u64(row: &ecat_data::Row, col: &str) -> u64 {
    row.get(col)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0)
}

fn col_str(row: &ecat_data::Row, col: &str) -> String {
    row.get(col).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

/// TEXT 列（渠道 name）sqlx Any 可能按 base64 返回，先解码；非 base64 原文使用
/// （同订单 snapshot_value 模式）。
fn decode_text(raw: &str) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_else(|| raw.to_string())
}

/// 渠道 name 为多语种 JSON（{"en":"Alipay","zh":"支付宝"}）：按 lang 取，回退 en/zh。
fn pick_name(raw: &str, lang: &str) -> String {
    let decoded = decode_text(raw);
    if let Ok(map) = serde_json::from_str::<serde_json::Map<String, Value>>(&decoded) {
        for k in [lang, "en", "zh"] {
            if let Some(v) = map.get(k).and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                return v.to_string();
            }
        }
    }
    decoded
}

/// 开发环境模拟验签：HMAC-SHA256(原始请求体, sandbox-secret)，输出小写 hex。
pub(crate) const SANDBOX_SECRET: &str = "sandbox-secret";

pub(crate) fn hmac_hex(body: &[u8]) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(SANDBOX_SECRET.as_bytes()).expect("hmac key ok");
    mac.update(body);
    mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Deserialize)]
pub(crate) struct CreatePaymentReq {
    pub(crate) order_id: u64,
    pub(crate) channel_code: String,
}

#[derive(Deserialize)]
pub(crate) struct CallbackReq {
    pub(crate) txn_no: String,
    pub(crate) status: u8,
}

#[derive(Deserialize)]
pub(crate) struct OrderIdQuery {
    pub(crate) order_id: Option<u64>,
}

#[derive(Deserialize)]
pub(crate) struct LangQuery {
    #[serde(default)]
    pub(crate) lang: String,
}

#[derive(Serialize)]
pub(crate) struct PaymentOut {
    pub(crate) id: u64,
    pub(crate) order_id: u64,
    pub(crate) channel_code: String,
    pub(crate) amount_cents: u64,
    pub(crate) status: u8,
    pub(crate) txn_no: String,
    pub(crate) created_at: String,
    pub(crate) checkout_url: String,
}

#[derive(Serialize)]
pub(crate) struct ChannelOut {
    pub(crate) channel_code: String,
    pub(crate) name: String,
    pub(crate) r#type: u8,
    pub(crate) priority: u64,
}

fn payment_from_row(row: &ecat_data::Row) -> PaymentOut {
    let txn_no = col_str(row, "txn_no");
    PaymentOut {
        id: col_u64(row, "id"),
        order_id: col_u64(row, "order_id"),
        channel_code: col_str(row, "channel_code"),
        amount_cents: col_u64(row, "amount_cents"),
        status: col_u64(row, "status") as u8,
        txn_no: txn_no.clone(),
        created_at: col_str(row, "created_at"),
        // 模拟收银台经网关（8082）访问；真实渠道接入点见 create_payment 注释
        checkout_url: format!("http://localhost:8082/api/v1/payments/sandbox/{txn_no}"),
    }
}

/// P4-06 发起支付（用户）：校验订单归属/待支付状态与渠道启用，幂等（同订单已有
/// 流水直接返回），生成 txn_no 插入待支付流水。
/// 渠道抽象（P4-15）：渠道侧接入点。真实接入时按 channel_code 分发到各渠道 SDK
/// 创建支付单并返回各自 checkout_url；本期全部渠道统一走模拟收银台。
pub(crate) async fn create_payment(
    State(state): State<AppState>,
    UserGuard(user_id): UserGuard,
    Json(body): Json<CreatePaymentReq>,
) -> (StatusCode, Json<ApiResponse<PaymentOut>>) {
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };
    // 1. 订单校验：存在 + 归属当前用户 + 待支付
    let orders = match db
        .query_with(
            "SELECT id, CAST(status AS CHAR) AS status, amount_cents \
             FROM travel_orders WHERE id = ? AND user_id = ?",
            &[json!(body.order_id), json!(user_id)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "order query failed");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    let Some(order_row) = orders.first() else {
        return err(StatusCode::NOT_FOUND, 404, "order not found");
    };
    if col_u64(order_row, "status") != 0 {
        return err(StatusCode::BAD_REQUEST, 400, "only pending orders can be paid");
    }
    let amount_cents = col_u64(order_row, "amount_cents");
    // 2. 渠道校验：存在 + 启用
    let channels = match db
        .query_with(
            "SELECT channel_code, CAST(enabled AS CHAR) AS enabled \
             FROM travel_payment_channels WHERE channel_code = ?",
            &[json!(body.channel_code)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "channel query failed");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    let Some(ch_row) = channels.first() else {
        return err(StatusCode::NOT_FOUND, 404, "channel not found");
    };
    if col_u64(ch_row, "enabled") != 1 {
        return err(StatusCode::BAD_REQUEST, 400, "channel disabled");
    }
    // 3. 幂等：同订单已有流水（含已终态，供前端轮询）直接返回，不重复建
    let existing = db
        .query_with(
            &format!(
                "SELECT {PAYMENT_COLS} FROM travel_payments WHERE order_id = ? \
                 ORDER BY id DESC LIMIT 1"
            ),
            &[json!(body.order_id)],
        )
        .await;
    if let Some(row) = existing.ok().and_then(|rows| rows.first().cloned()) {
        return (StatusCode::OK, ApiResponse::ok(payment_from_row(&row)));
    }
    // 4. 生成 txn_no（渠道_时间戳_自增，足够唯一）并插入
    static TXN_SEQ: AtomicU64 = AtomicU64::new(0);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let txn_no = format!(
        "{}_{}_{}",
        body.channel_code,
        ts,
        TXN_SEQ.fetch_add(1, Ordering::Relaxed)
    );
    // 主键去 AUTO_INCREMENT 后显式生成雪花 id
    let payment_id = idgen_rs::id_helper::next_id();
    if let Err(e) = db
        .execute_with(
            "INSERT INTO travel_payments (id, order_id, channel_code, amount_cents, status, txn_no) \
             VALUES (?, ?, ?, ?, 0, ?)",
            &[json!(payment_id), json!(body.order_id), json!(body.channel_code), json!(amount_cents), json!(txn_no)],
        )
        .await
    {
        tracing::warn!(error = %e, "payment insert failed");
        return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
    }
    // 5. 读回新流水（按生成的 id 精确回查，无竞态）
    let rows = match db
        .query_with(
            &format!("SELECT {PAYMENT_COLS} FROM travel_payments WHERE id = ?"),
            &[json!(payment_id)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "payment readback failed");
            return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
        }
    };
    match rows.first() {
        Some(row) => (StatusCode::OK, ApiResponse::ok(payment_from_row(row))),
        None => err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error"),
    }
}

/// P4-06 模拟收银台（公开）：展示渠道名/金额，「模拟支付成功/失败」两个按钮，
/// 点击后用 WebCrypto 计算 X-Signature 并 POST 到回调接口。开发调试用。
pub(crate) async fn sandbox_page(
    State(state): State<AppState>,
    Path(txn_no): Path<String>,
) -> Response {
    let Some(db) = state.db.clone() else {
        return err::<Value>(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable")
            .into_response();
    };
    let rows = match db
        .query_with(
            "SELECT p.channel_code, p.amount_cents, CAST(p.status AS CHAR) AS status, \
             CAST(c.name AS CHAR) AS name \
             FROM travel_payments p \
             LEFT JOIN travel_payment_channels c ON c.channel_code = p.channel_code \
             WHERE p.txn_no = ?",
            &[json!(txn_no)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "sandbox query failed");
            return err::<Value>(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable")
                .into_response();
        }
    };
    let Some(row) = rows.first() else {
        return err::<Value>(StatusCode::NOT_FOUND, 404, "payment not found").into_response();
    };
    let channel_code = col_str(row, "channel_code");
    let name = pick_name(&col_str(row, "name"), "zh");
    let amount_yuan = col_u64(row, "amount_cents") as f64 / 100.0;
    let status = col_u64(row, "status");
    let state_html = match status {
        1 => "<p style=\"color:#16a34a\">已支付成功</p>",
        2 => "<p style=\"color:#dc2626\">已支付失败</p>",
        _ => "",
    };
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh"><head><meta charset="utf-8"><title>模拟收银台</title>
<style>body{{font-family:system-ui;max-width:420px;margin:40px auto;padding:0 16px;background:#f7f7f8}}
.card{{background:#fff;border:1px solid #ddd;border-radius:12px;padding:24px}}
.btn{{display:block;width:100%;padding:12px;margin:8px 0;border:0;border-radius:8px;color:#fff;font-size:16px;cursor:pointer}}
.ok{{background:#16a34a}}.fail{{background:#dc2626}}.hint{{color:#666;font-size:13px}}</style></head>
<body><div class="card">
<h2>{name} · 模拟收银台</h2>
<p>金额：¥{amount_yuan:.2}</p>
<p>单号：{txn_no}</p>
{state_html}
<button class="btn ok" onclick="pay(1)">模拟支付成功</button>
<button class="btn fail" onclick="pay(2)">模拟支付失败</button>
<p class="hint" id="msg"></p></div>
<script>
async function hmac(body) {{
  const key = await crypto.subtle.importKey('raw', new TextEncoder().encode('{secret}'),
    {{name:'HMAC', hash:'SHA-256'}}, false, ['sign']);
  const sig = await crypto.subtle.sign('HMAC', key, new TextEncoder().encode(body));
  return [...new Uint8Array(sig)].map(b => b.toString(16).padStart(2, '0')).join('');
}}
async function pay(status) {{
  const body = JSON.stringify({{txn_no:'{txn_no}', status}});
  const resp = await fetch('/api/v1/payments/callback/{channel_code}', {{
    method:'POST',
    headers:{{'content-type':'application/json','x-signature': await hmac(body)}},
    body}});
  const out = await resp.json();
  document.getElementById('msg').textContent = '回调结果: ' + (out.message || resp.status);
  setTimeout(() => location.reload(), 800);
}}
</script></body></html>"#,
        name = name,
        amount_yuan = amount_yuan,
        txn_no = txn_no,
        state_html = state_html,
        secret = SANDBOX_SECRET,
        channel_code = channel_code,
    );
    (StatusCode::OK, Html(html)).into_response()
}

/// 通知 order-service 确认订单已支付（P4-07 闭环）。独立成函数便于集成测试注入
/// mock；失败返回 Err（渠道可重试）。
pub(crate) async fn confirm_order_paid(
    state: &AppState,
    order_id: u64,
    txn_no: &str,
) -> Result<(), String> {
    state.confirm.confirm(order_id, txn_no).await
}

/// P4-06 渠道异步回调（公开）：验签（模拟 HMAC）→ 幂等（已终态直接成功）→
/// 入账（仅 status=0 可更新）→ 成功回调后调 order-service 确认（失败 500，渠道可重试）。
pub(crate) async fn payment_callback(
    State(state): State<AppState>,
    Path(_channel_code): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, Json<ApiResponse<Value>>) {
    // 1. 验签：HMAC-SHA256(原始请求体)；失败 401 且不入账
    let got = headers.get("x-signature").and_then(|v| v.to_str().ok()).unwrap_or("");
    if got != hmac_hex(&body) {
        return err(StatusCode::UNAUTHORIZED, 401, "invalid signature");
    }
    let parsed: CallbackReq = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(_) => return err(StatusCode::BAD_REQUEST, 400, "invalid body"),
    };
    if parsed.status != 1 && parsed.status != 2 {
        return err(StatusCode::BAD_REQUEST, 400, "status must be 1 or 2");
    }
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };
    // 2. 幂等：查 txn_no；不存在 404；已终态（1/2/3）直接返回成功不重复入账
    let rows = match db
        .query_with(
            "SELECT order_id, CAST(status AS CHAR) AS status \
             FROM travel_payments WHERE txn_no = ?",
            &[json!(parsed.txn_no)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "payment query failed");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    let Some(row) = rows.first() else {
        return err(StatusCode::NOT_FOUND, 404, "payment not found");
    };
    let order_id = col_u64(row, "order_id");
    let cur = col_u64(row, "status");
    if cur == 1 || cur == 2 || cur == 3 {
        return (StatusCode::OK, ApiResponse::ok(Value::Null));
    }
    // 3. 入账：仅 status=0 可更新（并发重复回调只有一个生效），成功置 paid_at
    let affected = match db
        .execute_with(
            "UPDATE travel_payments SET status = ?, \
             paid_at = IF(? = 1, NOW(), paid_at) \
             WHERE txn_no = ? AND status = 0",
            &[json!(parsed.status), json!(parsed.status), json!(parsed.txn_no)],
        )
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, "payment update failed");
            return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
        }
    };
    if affected == 0 {
        // 已被并发回调入账（终态），幂等放行
        return (StatusCode::OK, ApiResponse::ok(Value::Null));
    }
    // 4. 成功 → 通知 order-service 确认订单；失败返回 500（渠道会重试，幂等保证安全）
    if parsed.status == 1 {
        if let Err(e) = confirm_order_paid(&state, order_id, &parsed.txn_no).await {
            tracing::error!(error = %e, order_id, "order confirm failed");
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "order confirm failed, retry later",
            );
        }
    }
    (StatusCode::OK, ApiResponse::ok(Value::Null))
}

/// P4-06 订单支付流水（用户，供前端轮询支付结果）。
pub(crate) async fn payment_list(
    State(state): State<AppState>,
    UserGuard(user_id): UserGuard,
    Query(q): Query<OrderIdQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<PaymentOut>>>) {
    let Some(order_id) = q.order_id else {
        return err(StatusCode::BAD_REQUEST, 400, "order_id required");
    };
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };
    // 归属校验：非本人订单返回 404（不泄露存在性）
    let own = match db
        .query_with(
            "SELECT id FROM travel_orders WHERE id = ? AND user_id = ?",
            &[json!(order_id), json!(user_id)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "order query failed");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    if own.first().is_none() {
        return err(StatusCode::NOT_FOUND, 404, "order not found");
    }
    let rows = match db
        .query_with(
            &format!(
                "SELECT {PAYMENT_COLS} FROM travel_payments WHERE order_id = ? ORDER BY id"
            ),
            &[json!(order_id)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "payments query failed");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    (StatusCode::OK, ApiResponse::ok(rows.iter().map(payment_from_row).collect()))
}

/// P4-15 启用渠道列表（公开）：本国渠道（languages 含 lang）按 priority DESC 排前，
/// 全语言渠道（languages=''）兜底排后；停用渠道不返回。
pub(crate) async fn channel_list(
    State(state): State<AppState>,
    Query(q): Query<LangQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<ChannelOut>>>) {
    let lang = {
        let l = q.lang.trim().to_lowercase();
        if l.is_empty() { "en".into() } else { l }
    };
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };
    let rows = match db
        .query_with(
            "SELECT channel_code, CAST(name AS CHAR) AS name, CAST(type AS CHAR) AS type, \
             priority, languages \
             FROM travel_payment_channels WHERE enabled = 1",
            &[],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "channels query failed");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    let mut out: Vec<(ChannelOut, String)> = rows
        .iter()
        .map(|row| {
            (
                ChannelOut {
                    channel_code: col_str(row, "channel_code"),
                    name: pick_name(&col_str(row, "name"), &lang),
                    r#type: col_u64(row, "type") as u8,
                    priority: col_u64(row, "priority"),
                },
                col_str(row, "languages"),
            )
        })
        .collect();
    out.sort_by(|a, b| {
        let a_local = a.1.split(',').any(|l| l.trim() == lang);
        let b_local = b.1.split(',').any(|l| l.trim() == lang);
        b_local.cmp(&a_local).then_with(|| b.0.priority.cmp(&a.0.priority))
    });
    (StatusCode::OK, ApiResponse::ok(out.into_iter().map(|(c, _)| c).collect()))
}
