use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::AppState;

pub mod mixed;

// Data structures for the recommendation service
#[derive(Debug, Serialize, Deserialize)]
pub struct WatchHistoryItem {
    pub video_id: String,
    pub last_watched_timestamp: String,
    pub mean_percentage_watched: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendationRequest {
    pub user_id: String,
    pub watch_history: Vec<WatchHistoryItem>,
    pub exclude_watched_items: Vec<String>, // IDs of videos to exclude from recommendations
}

// Re-export the mixed endpoint handler and its path macro
pub use mixed::__path_get_feed_mixed_v3;
pub use mixed::get_feed_mixed_v3;

pub fn feed_router_v2(state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(get_feed_mixed_v3))
        .with_state(state)
}
