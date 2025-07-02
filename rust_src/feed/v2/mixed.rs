use super::{RecommendationRequest, WatchHistoryItem};
use crate::AppState;
use axum::{extract::State, http::StatusCode, response::Json};
use chrono::DateTime;
use std::{sync::Arc, time::SystemTime};
use yral_ml_feed_cache::{
    consts::{
        MAX_WATCH_HISTORY_CACHE_LEN, USER_WATCH_HISTORY_CLEAN_SUFFIX,
        USER_WATCH_HISTORY_NSFW_SUFFIX,
    },
    types::{FeedRequest, FeedResponse, FeedResponseV2, PostItem, PostItemV2},
};

const RECOMMENDATION_SERVICE_URL: &str =
    "https://recommendation-service-749244211103.us-central1.run.app/recommendations";

#[utoipa::path(
    post,
    path = "/mixed",
    request_body = FeedRequest,
    responses(
        (status = 200, description = "Successfully retrieved mixed feed", body = FeedResponseV2),
        (status = 502, description = "Bad gateway - recommendation service error"),
        (status = 500, description = "Internal server error")
    ),
    tag = "ML_FEED"
)]
pub async fn get_feed_mixed_v2(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FeedRequest>,
) -> Result<Json<FeedResponseV2>, StatusCode> {
    // Collect watch history from both clean and nsfw caches
    let watch_history = collect_watch_history(&state, &req.canister_id).await?;

    let excluded_videos: Vec<String> = req
        .filter_results
        .iter()
        .map(|post| post.video_id.clone())
        .collect();

    // Transform to recommendation service format
    let recommendation_request = RecommendationRequest {
        user_id: req.canister_id.clone(),
        watch_history,
        exclude_watched_items: excluded_videos,
    };

    // Call recommendation service
    let client = reqwest::Client::new();
    let response = client
        .post(RECOMMENDATION_SERVICE_URL)
        .json(&recommendation_request)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }

    // Parse response as FeedResponse
    let feed_response: FeedResponseV2 = response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(feed_response))
}

async fn collect_watch_history(
    state: &Arc<AppState>,
    canister_id: &str,
) -> Result<Vec<WatchHistoryItem>, StatusCode> {
    let ml_feed_cache = &state.ml_feed_cache;
    let mut history_items = Vec::new();

    // Collect from both clean and nsfw watch history
    let clean_key = format!("{}{}", canister_id, USER_WATCH_HISTORY_CLEAN_SUFFIX);
    let nsfw_key = format!("{}{}", canister_id, USER_WATCH_HISTORY_NSFW_SUFFIX);

    // Get clean history - get all available items
    if let Ok(clean_history) = ml_feed_cache
        .get_history_items(&clean_key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await
    {
        history_items.extend(clean_history);
    }

    // Get nsfw history - get all available items
    if let Ok(nsfw_history) = ml_feed_cache
        .get_history_items(&nsfw_key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await
    {
        history_items.extend(nsfw_history);
    }

    // Transform to recommendation service format
    let watch_history: Vec<WatchHistoryItem> = history_items
        .into_iter()
        .map(|item| {
            let timestamp = format_timestamp(item.timestamp);
            let percentage = format!("{}", item.percent_watched);

            WatchHistoryItem {
                video_id: item.video_id.clone(),
                last_watched_timestamp: timestamp,
                mean_percentage_watched: percentage,
            }
        })
        .collect();

    Ok(watch_history)
}

fn format_timestamp(timestamp: SystemTime) -> String {
    // Convert SystemTime to Unix timestamp and then to RFC3339 format with timezone
    let unix_timestamp = timestamp
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let dt = DateTime::from_timestamp(unix_timestamp, 0)
        .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());
    dt.format("%Y-%m-%d %H:%M:%S%.6f+00:00").to_string()
}
