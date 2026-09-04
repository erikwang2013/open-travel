// 评价体系（P5-01）：提交（JWT）/ 列表（公开）/ 景区详情聚合
use axum::body::to_bytes;
use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::Json;
use ecat_auth::claims_from_request;
use ecat_data::Row;
use ecat_data::RdbmsClient;
use ecat_data_sqlx::SqlxClient;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{col_f64, col_str, col_u64, err, ApiResponse, AppState};

pub(crate) const DETAIL_REVIEW_LIMIT: u64 = 20;
const COMMENT_MAX: usize = 500;
const PAGE_SIZE_MAX: u64 = 50;

#[derive(Serialize)]
pub(crate) struct ReviewRow {
    id: u64,
    attraction_id: u64,
    user_id: u64,
    rating: u64,
    comment: String,
    nickname: String,
    created_at: String,
}

#[derive(Deserialize)]
struct ReviewCreate {
    attraction_id: u64,
    rating: i64,
    #[serde(default)]
    comment: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ReviewsQuery {
    attraction_id: u64,
    #[serde(default = "page_one")]
    page: u64,
    #[serde(default = "page_size_ten")]
    page_size: u64,
}

fn page_one() -> u64 {
    1
}

fn page_size_ten() -> u64 {
    10
}

// TINYINT/DATETIME 经 sqlx Any 可能非字符串解码，统一 CAST AS CHAR 规避；
// content 为 TEXT 列同理，并重命名 comment 与响应字段对齐。
const REVIEW_COLS: &str = "r.id, r.attraction_id, r.user_id, \
    CAST(r.rating AS CHAR) AS rating, CAST(r.content AS CHAR) AS comment, \
    COALESCE(u.nickname, '') AS nickname, \
    CAST(r.created_at AS CHAR) AS created_at";

/// sqlx Any 将 TEXT 列按 Blob 解码并 base64 编码返回（与 description 列同陷阱），
/// 先尝试 base64 解码，非 base64 文本直接返回。
fn col_text(row: &Row, col: &str) -> String {
    use base64::Engine as _;
    let raw = row.get(col).and_then(|v| v.as_str()).unwrap_or("");
    base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_else(|| raw.to_string())
}

fn review_from_row(row: &Row) -> ReviewRow {
    ReviewRow {
        id: col_u64(row, "id"),
        attraction_id: col_u64(row, "attraction_id"),
        user_id: col_u64(row, "user_id"),
        rating: col_u64(row, "rating"),
        comment: col_text(row, "comment"),
        nickname: col_str(row, "nickname"),
        created_at: col_str(row, "created_at"),
    }
}

pub(crate) async fn fetch_reviews(
    db: &SqlxClient,
    attraction_id: u64,
    limit: u64,
    offset: u64,
) -> Vec<ReviewRow> {
    match db
        .query_with(
            &format!(
                "SELECT {REVIEW_COLS} FROM travel_reviews r \
                 LEFT JOIN travel_users u ON u.id = r.user_id \
                 WHERE r.attraction_id = ? ORDER BY r.id DESC LIMIT {limit} OFFSET {offset}"
            ),
            &[json!(attraction_id)],
        )
        .await
    {
        Ok(rows) => rows.iter().map(review_from_row).collect(),
        Err(e) => {
            tracing::warn!("reviews query failed: {e}");
            Vec::new()
        }
    }
}

/// 无评价（AVG 为 NULL）时返回 None，由调用方保留表内 rating_avg。
pub(crate) async fn fetch_rating_avg(db: &SqlxClient, attraction_id: u64) -> Option<f64> {
    match db
        .query_with(
            "SELECT CAST(AVG(rating) AS CHAR) AS avg_rating \
             FROM travel_reviews WHERE attraction_id = ?",
            &[json!(attraction_id)],
        )
        .await
    {
        Ok(rows) => rows.first().map(|row| col_f64(row, "avg_rating")).filter(|v| *v > 0.0),
        Err(e) => {
            tracing::warn!("rating avg query failed: {e}");
            None
        }
    }
}

pub(crate) async fn create_review(
    State(state): State<AppState>,
    req: Request,
) -> (StatusCode, Json<ApiResponse<ReviewRow>>) {
    // claims_from_request 需要整 Request，body 手动解析（与 user-service 一致）
    let Some(claims) = claims_from_request(&req) else {
        return err(StatusCode::UNAUTHORIZED, 401, "missing claims");
    };
    let Ok(user_id) = claims.sub.parse::<u64>() else {
        return err(StatusCode::BAD_REQUEST, 400, "invalid subject in token");
    };
    let bytes = match to_bytes(req.into_body(), 64 * 1024).await {
        Ok(b) => b,
        Err(_) => return err(StatusCode::BAD_REQUEST, 400, "invalid json body"),
    };
    let body: ReviewCreate = match serde_json::from_slice(&bytes) {
        Ok(b) => b,
        Err(_) => return err(StatusCode::BAD_REQUEST, 400, "invalid json body"),
    };
    if !(1..=5).contains(&body.rating) {
        return err(StatusCode::BAD_REQUEST, 400, "rating must be between 1 and 5");
    }
    let comment = body.comment.as_deref().map(str::trim).unwrap_or("");
    if comment.chars().count() > COMMENT_MAX {
        return err(StatusCode::BAD_REQUEST, 400, "comment too long");
    }
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };
    let dest_rows = match db
        .query_with(
            "SELECT destination_id FROM travel_attractions WHERE id = ? AND status = 1",
            &[json!(body.attraction_id)],
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("attraction check failed: {e}");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    let Some(dest_row) = dest_rows.first() else {
        return err(StatusCode::NOT_FOUND, 404, "attraction not found");
    };
    let comment_json = if comment.is_empty() {
        json!(None::<String>)
    } else {
        json!(comment)
    };
    // 主键去 AUTO_INCREMENT 后显式生成雪花 id
    let review_id = ecat::business::shared::snowflake_id().await;
    if let Err(e) = db
        .query_with(
            "INSERT INTO travel_reviews (id, user_id, attraction_id, destination_id, rating, content, lang) \
             VALUES (?, ?, ?, ?, ?, ?, 'en')",
            &[
                json!(review_id),
                json!(user_id),
                json!(body.attraction_id),
                json!(col_u64(dest_row, "destination_id")),
                json!(body.rating),
                comment_json,
            ],
        )
        .await
    {
        tracing::warn!("review insert failed: {e}");
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    }
    let rows = match db
        .query_with(
            &format!(
                "SELECT {REVIEW_COLS} FROM travel_reviews r \
                 LEFT JOIN travel_users u ON u.id = r.user_id \
                 WHERE r.id = ?"
            ),
            &[json!(review_id)],
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("review select failed: {e}");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    let Some(row) = rows.first() else {
        return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "inserted review not found");
    };
    (
        StatusCode::CREATED,
        Json(ApiResponse { code: 0, message: "ok".into(), data: Some(review_from_row(row)) }),
    )
}

pub(crate) async fn list_reviews(
    State(state): State<AppState>,
    Query(q): Query<ReviewsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<ReviewRow>>>) {
    if q.attraction_id == 0 {
        return err(StatusCode::BAD_REQUEST, 400, "attraction_id is required");
    }
    let page = q.page.max(1);
    let page_size = q.page_size.clamp(1, PAGE_SIZE_MAX);
    tracing::info!(event = "booking.reviews.listed", attraction_id = q.attraction_id, page, page_size);
    let offset = (page - 1) * page_size;
    let mut rows: Vec<ReviewRow> = Vec::new();
    for db in [state.replica.as_ref(), state.db.as_ref()].into_iter().flatten() {
        rows = fetch_reviews(db, q.attraction_id, page_size, offset).await;
        if !rows.is_empty() {
            break;
        }
    }
    (StatusCode::OK, Json(ApiResponse { code: 0, message: "ok".into(), data: Some(rows) }))
}
