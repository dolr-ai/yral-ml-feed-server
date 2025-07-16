use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use candid::Principal;
use http::StatusCode;
use tracing::instrument;
use utils::{
    coldstart_clean_cache::{
        get_coldstart_clean_cache_input_user_impl, get_coldstart_clean_cache_noinput_impl,
        get_coldstart_clean_cache_noinput_user_impl,
    },
    coldstart_mixed_cache::{
        get_coldstart_mixed_cache_input_user_impl, get_coldstart_mixed_cache_noinput_impl,
        get_coldstart_mixed_cache_noinput_user_impl,
    },
    coldstart_nsfw_cache::{
        get_coldstart_nsfw_cache_input_user_impl, get_coldstart_nsfw_cache_noinput_impl,
        get_coldstart_nsfw_cache_noinput_user_impl,
    },
    global_cache::*,
    ml_feed::*,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use yral_ml_feed_cache::{
    consts::{
        GLOBAL_CACHE_CLEAN_KEY, GLOBAL_CACHE_MIXED_KEY, GLOBAL_CACHE_NSFW_KEY,
        USER_CACHE_CLEAN_SUFFIX, USER_CACHE_MIXED_SUFFIX, USER_CACHE_NSFW_SUFFIX,
    },
    types::{FeedRequest, FeedResponse},
};

pub mod feed_v3;
pub mod utils;

use crate::{utils::remove_duplicates, AppState};

#[instrument(skip(state))]
pub fn feed_router(state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(get_feed_coldstart_clean_v2))
        .routes(routes!(get_feed_coldstart_nsfw_v2))
        .routes(routes!(get_feed_coldstart_mixed_v2))
        .routes(routes!(get_feed_clean_v2))
        .routes(routes!(get_feed_nsfw_v2))
        .routes(routes!(get_feed_mixed_v2))
        .routes(routes!(update_global_cache_clean))
        .routes(routes!(update_global_cache_nsfw))
        .routes(routes!(update_global_cache_mixed))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/coldstart/clean",
    request_body = FeedRequest,
    tag = "feed",
    responses(
        (status = 200, description = "Feed sent successfully", body = FeedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[deprecated]
#[instrument(skip(state, payload), fields(canister_id = %payload.canister_id))]
async fn get_feed_coldstart_clean_v2(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FeedRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    log::info!("get_feed_coldstart_clean_v2");

    let canister_id = payload.canister_id.clone();
    let num_results = payload.num_results;
    let filter_results = payload.filter_results;
    let ml_feed_cache = state.ml_feed_cache.clone();

    if num_results > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            "num_results must be less than 500".to_string(),
        ));
    }

    if num_results == 1 && canister_id == Principal::anonymous().to_string() {
        let feed = get_coldstart_clean_cache_noinput_impl(ml_feed_cache)
            .await
            .map_err(|e| {
                log::error!("Failed to get coldstart clean cache: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;

        return Ok(Json(FeedResponse {
            posts: vec![feed[0].clone()],
        }));
    } else if num_results == 1 && canister_id != Principal::anonymous().to_string() {
        let feed = get_coldstart_clean_cache_noinput_user_impl(ml_feed_cache, canister_id)
            .await
            .map_err(|e| {
                log::error!("Failed to get coldstart clean cache: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;

        return Ok(Json(FeedResponse {
            posts: vec![feed[0].clone()],
        }));
    }

    let feed = get_coldstart_clean_cache_input_user_impl(
        ml_feed_cache,
        canister_id,
        num_results,
        filter_results,
    )
    .await
    .map_err(|e| {
        log::error!("Failed to get coldstart clean cache: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(FeedResponse {
        posts: remove_duplicates(feed),
    }))
}

#[utoipa::path(
    post,
    path = "/coldstart/nsfw",
    request_body = FeedRequest,
    tag = "feed",
    responses(
        (status = 200, description = "Feed sent successfully", body = FeedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[deprecated]
#[instrument(skip(state, payload), fields(canister_id = %payload.canister_id))]
async fn get_feed_coldstart_nsfw_v2(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FeedRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    log::info!("get_feed_coldstart_nsfw_v2");

    let canister_id = payload.canister_id.clone();
    let num_results = payload.num_results;
    let filter_results = payload.filter_results;
    let ml_feed_cache = state.ml_feed_cache.clone();

    if num_results > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            "num_results must be less than 500".to_string(),
        ));
    }

    if num_results == 1 && canister_id == Principal::anonymous().to_string() {
        let feed = get_coldstart_nsfw_cache_noinput_impl(ml_feed_cache)
            .await
            .map_err(|e| {
                log::error!("Failed to get coldstart nsfw cache: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;

        return Ok(Json(FeedResponse {
            posts: vec![feed[0].clone()],
        }));
    } else if num_results == 1 && canister_id != Principal::anonymous().to_string() {
        let feed = get_coldstart_nsfw_cache_noinput_user_impl(ml_feed_cache, canister_id)
            .await
            .map_err(|e| {
                log::error!("Failed to get coldstart nsfw cache: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;

        return Ok(Json(FeedResponse {
            posts: vec![feed[0].clone()],
        }));
    }

    let feed = get_coldstart_nsfw_cache_input_user_impl(
        ml_feed_cache,
        canister_id,
        num_results,
        filter_results,
    )
    .await
    .map_err(|e| {
        log::error!("Failed to get coldstart nsfw cache: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(FeedResponse {
        posts: remove_duplicates(feed),
    }))
}

#[utoipa::path(
    post,
    path = "/coldstart/mixed",
    request_body = FeedRequest,
    tag = "feed",
    responses(
        (status = 200, description = "Feed sent successfully", body = FeedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[deprecated]
#[instrument(skip(state, payload), fields(canister_id = %payload.canister_id))]
async fn get_feed_coldstart_mixed_v2(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FeedRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let canister_id = payload.canister_id.clone();
    let num_results = payload.num_results;
    let filter_results = payload.filter_results;
    let ml_feed_cache = state.ml_feed_cache.clone();

    if num_results > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            "num_results must be less than 500".to_string(),
        ));
    }

    if num_results == 1 && canister_id == Principal::anonymous().to_string() {
        let feed = get_coldstart_mixed_cache_noinput_impl(ml_feed_cache)
            .await
            .map_err(|e| {
                log::error!("Failed to get coldstart mixed cache: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;

        return Ok(Json(FeedResponse {
            posts: vec![feed[0].clone()],
        }));
    } else if num_results == 1 && canister_id != Principal::anonymous().to_string() {
        let feed = get_coldstart_mixed_cache_noinput_user_impl(ml_feed_cache, canister_id)
            .await
            .map_err(|e| {
                log::error!("Failed to get coldstart mixed cache: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;

        return Ok(Json(FeedResponse {
            posts: vec![feed[0].clone()],
        }));
    }

    let feed = get_coldstart_mixed_cache_input_user_impl(
        ml_feed_cache,
        canister_id,
        num_results,
        filter_results,
    )
    .await
    .map_err(|e| {
        log::error!("Failed to get coldstart mixed cache: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(FeedResponse {
        posts: remove_duplicates(feed),
    }))
}

#[utoipa::path(
    post,
    path = "/clean",
    request_body = FeedRequest,
    tag = "feed",
    responses(
        (status = 200, description = "Feed sent successfully", body = FeedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[deprecated]
#[instrument(skip(state, payload), fields(canister_id = %payload.canister_id))]
async fn get_feed_clean_v2(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FeedRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let canister_id = payload.canister_id.clone();
    let feed = get_ml_feed_clean_impl(
        state.ml_feed_cache.clone(),
        canister_id.clone(),
        payload.filter_results,
        payload.num_results + 100,
    )
    .await
    .map_err(|e| {
        log::error!("Failed to get ml_feed_clean_impl: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let feed = remove_duplicates(feed);

    if feed.len() > payload.num_results as usize {
        let (first_part, rest) = feed.split_at(payload.num_results as usize);

        let rest = rest.to_vec();
        tokio::spawn(async move {
            let res = state
                .ml_feed_cache
                .add_user_cache_items(&format!("{}{}", canister_id, USER_CACHE_CLEAN_SUFFIX), rest)
                .await;
            if res.is_err() {
                log::error!(
                    "Failed to add user cache _watch_clean items: {}",
                    res.err().unwrap()
                );
            }
        });

        return Ok(Json(FeedResponse {
            posts: first_part.to_vec(),
        }));
    }

    Ok(Json(FeedResponse { posts: feed }))
}

#[utoipa::path(
    post,
    path = "/nsfw",
    request_body = FeedRequest,
    tag = "feed",
    responses(
        (status = 200, description = "Feed sent successfully", body = FeedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[deprecated]
#[instrument(skip(state, payload), fields(canister_id = %payload.canister_id))]
async fn get_feed_nsfw_v2(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FeedRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let canister_id = payload.canister_id.clone();
    let feed = get_ml_feed_nsfw_impl(
        state.ml_feed_cache.clone(),
        canister_id.clone(),
        payload.filter_results,
        payload.num_results + 100,
    )
    .await
    .map_err(|e| {
        log::error!("Failed to get ml_feed_nsfw_impl: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let feed = remove_duplicates(feed);

    if feed.len() > payload.num_results as usize {
        let (first_part, rest) = feed.split_at(payload.num_results as usize);

        let rest = rest.to_vec();
        tokio::spawn(async move {
            let res = state
                .ml_feed_cache
                .add_user_cache_items(&format!("{}{}", canister_id, USER_CACHE_NSFW_SUFFIX), rest)
                .await;
            if res.is_err() {
                log::error!(
                    "Failed to add user cache _watch_nsfw items: {}",
                    res.err().unwrap()
                );
            }
        });

        return Ok(Json(FeedResponse {
            posts: first_part.to_vec(),
        }));
    }

    Ok(Json(FeedResponse { posts: feed }))
}

#[utoipa::path(
    post,
    path = "/mixed",
    request_body = FeedRequest,
    tag = "feed",
    responses(
        (status = 200, description = "Feed sent successfully", body = FeedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[deprecated]
#[instrument(skip(state, payload), fields(canister_id = %payload.canister_id))]
async fn get_feed_mixed_v2(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FeedRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let canister_id = payload.canister_id.clone();
    let feed = get_ml_feed_mixed_impl(
        state.ml_feed_cache.clone(),
        canister_id.clone(),
        payload.filter_results,
        payload.num_results + 100,
    )
    .await
    .map_err(|e| {
        log::error!("Failed to get ml_feed_mixed_impl: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let feed = remove_duplicates(feed);

    if feed.len() > payload.num_results as usize {
        let (first_part, rest) = feed.split_at(payload.num_results as usize);

        let rest = rest.to_vec();
        tokio::spawn(async move {
            let res = state
                .ml_feed_cache
                .add_user_cache_items(&format!("{}{}", canister_id, USER_CACHE_MIXED_SUFFIX), rest)
                .await;
            if res.is_err() {
                log::error!(
                    "Failed to add user cache _watch_mixed items: {}",
                    res.err().unwrap()
                );
            }
        });

        return Ok(Json(FeedResponse {
            posts: first_part.to_vec(),
        }));
    }

    Ok(Json(FeedResponse { posts: feed }))
}

#[utoipa::path(
    post,
    path = "/global-cache/clean",
    tag = "update-global-cache",
    responses(
        (status = 200, description = "Feed sent successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[deprecated]
#[instrument(skip(state))]
async fn update_global_cache_clean(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let feed = get_global_cache_clean().await.map_err(|e| {
        log::error!("Failed to get global cache clean: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let feed = remove_duplicates(feed);

    state
        .ml_feed_cache
        .add_global_cache_items(GLOBAL_CACHE_CLEAN_KEY, feed)
        .await
        .map_err(|e| {
            log::error!("Failed to update global cache: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/global-cache/nsfw",
    tag = "update-global-cache",
    responses(
        (status = 200, description = "Feed sent successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[deprecated]
#[instrument(skip(state))]
async fn update_global_cache_nsfw(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let feed = get_global_cache_nsfw().await.map_err(|e| {
        log::error!("Failed to get global cache clean: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let feed = remove_duplicates(feed);

    state
        .ml_feed_cache
        .add_global_cache_items(GLOBAL_CACHE_NSFW_KEY, feed)
        .await
        .map_err(|e| {
            log::error!("Failed to update global cache: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/global-cache/mixed",
    tag = "update-global-cache",
    responses(
        (status = 200, description = "Feed sent successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[deprecated]
#[instrument(skip(state))]
async fn update_global_cache_mixed(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let feed = get_global_cache_mixed().await.map_err(|e| {
        log::error!("Failed to get global cache clean: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let feed = remove_duplicates(feed);

    state
        .ml_feed_cache
        .add_global_cache_items(GLOBAL_CACHE_MIXED_KEY, feed)
        .await
        .map_err(|e| {
            log::error!("Failed to update global cache: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    Ok(())
}

