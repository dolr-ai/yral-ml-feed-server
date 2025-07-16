use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use candid::Principal;
use http::StatusCode;
use tracing::instrument;
use utoipa_axum::{router::OpenApiRouter, routes};
use yral_ml_feed_cache::consts::{
    GLOBAL_CACHE_CLEAN_KEY_V2, GLOBAL_CACHE_NSFW_KEY_V2, USER_CACHE_CLEAN_SUFFIX_V2,
    USER_CACHE_NSFW_SUFFIX_V2,
};
use yral_types::post::{FeedRequestV2, FeedResponseV2};

use crate::{
    feed::utils::{
        coldstart_clean_cache_v3::{
            get_coldstart_clean_cache_input_user_impl, get_coldstart_clean_cache_noinput_impl,
            get_coldstart_clean_cache_noinput_user_impl,
        },
        coldstart_nsfw_cache_v3::{
            get_coldstart_nsfw_cache_input_user_impl, get_coldstart_nsfw_cache_noinput_impl,
            get_coldstart_nsfw_cache_noinput_user_impl,
        },
    },
    AppState,
};

use super::utils::{
    global_cache_v3::{get_global_cache_clean_v3, get_global_cache_nsfw_v3},
    ml_feed_v3::{get_ml_feed_clean_v3_impl, get_ml_feed_nsfw_v3_impl},
};

#[instrument(skip(state))]
pub fn feed_v3_router(state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(get_feed_clean_v3))
        .routes(routes!(get_feed_nsfw_v3))
        .routes(routes!(update_global_cache_clean_v3))
        .routes(routes!(update_global_cache_nsfw_v3))
        .routes(routes!(get_feed_coldstart_clean_v3))
        .routes(routes!(get_feed_coldstart_nsfw_v3))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/coldstart/clean",
    request_body = FeedRequestV2,
    tag = "feed-v3",
    responses(
        (status = 200, description = "Feed sent successfully", body = FeedResponseV2),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(state, payload), fields(canister_id = %payload.user_id))]
async fn get_feed_coldstart_clean_v3(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FeedRequestV2>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    log::info!("get_feed_coldstart_clean_v3");

    let user_id = payload.user_id.clone();
    let user_id_str = user_id.to_text();
    let num_results = payload.num_results;
    let filter_results = payload.filter_results;
    let ml_feed_cache = state.ml_feed_cache.clone();

    if num_results > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            "num_results must be less than 500".to_string(),
        ));
    }

    if num_results == 1 && user_id == Principal::anonymous() {
        let feed = get_coldstart_clean_cache_noinput_impl(ml_feed_cache)
            .await
            .map_err(|e| {
                log::error!("Failed to get coldstart clean cache: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;

        return Ok(Json(FeedResponseV2 {
            posts: vec![feed[0].clone()],
        }));
    } else if num_results == 1 && user_id != Principal::anonymous() {
        let feed = get_coldstart_clean_cache_noinput_user_impl(ml_feed_cache, user_id_str)
            .await
            .map_err(|e| {
                log::error!("Failed to get coldstart clean cache: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;

        return Ok(Json(FeedResponseV2 {
            posts: vec![feed[0].clone()],
        }));
    }

    let feed = get_coldstart_clean_cache_input_user_impl(
        ml_feed_cache,
        user_id_str,
        num_results,
        filter_results,
    )
    .await
    .map_err(|e| {
        log::error!("Failed to get coldstart clean cache: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(FeedResponseV2 { posts: feed }))
}

#[utoipa::path(
    post,
    path = "/coldstart/nsfw",
    request_body = FeedRequestV2,
    tag = "feed-v3",
    responses(
        (status = 200, description = "Feed sent successfully", body = FeedResponseV2),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(state, payload), fields(user_id = %payload.user_id))]
async fn get_feed_coldstart_nsfw_v3(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FeedRequestV2>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    log::info!("get_feed_coldstart_nsfw_v3");

    let user_id = payload.user_id.clone();
    let user_id_str = user_id.to_text();
    let num_results = payload.num_results;
    let filter_results = payload.filter_results;
    let ml_feed_cache = state.ml_feed_cache.clone();

    if num_results > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            "num_results must be less than 500".to_string(),
        ));
    }

    if num_results == 1 && user_id == Principal::anonymous() {
        let feed = get_coldstart_nsfw_cache_noinput_impl(ml_feed_cache)
            .await
            .map_err(|e| {
                log::error!("Failed to get coldstart nsfw cache: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;

        return Ok(Json(FeedResponseV2 {
            posts: vec![feed[0].clone()],
        }));
    } else if num_results == 1 && user_id != Principal::anonymous() {
        let feed = get_coldstart_nsfw_cache_noinput_user_impl(ml_feed_cache, user_id_str)
            .await
            .map_err(|e| {
                log::error!("Failed to get coldstart nsfw cache: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;

        return Ok(Json(FeedResponseV2 {
            posts: vec![feed[0].clone()],
        }));
    }

    let feed = get_coldstart_nsfw_cache_input_user_impl(
        ml_feed_cache,
        user_id_str,
        num_results,
        filter_results,
    )
    .await
    .map_err(|e| {
        log::error!("Failed to get coldstart nsfw cache: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Json(FeedResponseV2 { posts: feed }))
}

#[utoipa::path(
    post,
    path = "/clean",
    request_body = FeedRequestV2,
    tag = "feed-v3",
    responses(
        (status = 200, description = "Feed sent successfully", body = FeedResponseV2),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(state, payload), fields(user_id = %payload.user_id))]
async fn get_feed_clean_v3(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FeedRequestV2>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_id = payload.user_id.clone();
    let feed = get_ml_feed_clean_v3_impl(
        state.ml_feed_cache.clone(),
        user_id.to_text(),
        payload.filter_results,
        payload.num_results + 100,
        &state.yral_metadata_client,
    )
    .await
    .map_err(|e| {
        log::error!("Failed to get ml_feed_clean_v3_impl: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    if feed.len() > payload.num_results as usize {
        let (first_part, rest) = feed.split_at(payload.num_results as usize);

        let rest = rest.to_vec();
        let user_id_for_cache = user_id.to_string();
        tokio::spawn(async move {
            let res = state
                .ml_feed_cache
                .add_user_cache_items_v2(
                    &format!("{}{}", user_id_for_cache, USER_CACHE_CLEAN_SUFFIX_V2),
                    rest,
                )
                .await;
            if res.is_err() {
                log::error!(
                    "Failed to add user cache clean v3 items: {}",
                    res.err().unwrap()
                );
            }
        });

        return Ok(Json(FeedResponseV2 {
            posts: first_part.to_vec(),
        }));
    }

    Ok(Json(FeedResponseV2 { posts: feed }))
}

#[utoipa::path(
    post,
    path = "/nsfw",
    request_body = FeedRequestV2,
    tag = "feed-v3",
    responses(
        (status = 200, description = "Feed sent successfully", body = FeedResponseV2),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(state, payload), fields(user_id = %payload.user_id))]
async fn get_feed_nsfw_v3(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FeedRequestV2>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_id = payload.user_id.clone();
    let feed = get_ml_feed_nsfw_v3_impl(
        state.ml_feed_cache.clone(),
        user_id.to_string(),
        payload.filter_results,
        payload.num_results + 100,
        &state.yral_metadata_client,
    )
    .await
    .map_err(|e| {
        log::error!("Failed to get ml_feed_nsfw_v3_impl: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    if feed.len() > payload.num_results as usize {
        let (first_part, rest) = feed.split_at(payload.num_results as usize);

        let rest = rest.to_vec();
        let user_id_for_cache = user_id.to_string();
        tokio::spawn(async move {
            let res = state
                .ml_feed_cache
                .add_user_cache_items_v2(
                    &format!("{}{}", user_id_for_cache, USER_CACHE_NSFW_SUFFIX_V2),
                    rest,
                )
                .await;
            if res.is_err() {
                log::error!(
                    "Failed to add user cache nsfw v3 items: {}",
                    res.err().unwrap()
                );
            }
        });

        return Ok(Json(FeedResponseV2 {
            posts: first_part.to_vec(),
        }));
    }

    Ok(Json(FeedResponseV2 { posts: feed }))
}

#[utoipa::path(
    post,
    path = "/global-cache/clean",
    tag = "update-global-cache-v3",
    responses(
        (status = 200, description = "Feed sent successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(state))]
async fn update_global_cache_clean_v3(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let feed = get_global_cache_clean_v3(&state.yral_metadata_client)
        .await
        .map_err(|e| {
            log::error!("Failed to get global cache clean v3: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    state
        .ml_feed_cache
        .add_global_cache_items_v2(GLOBAL_CACHE_CLEAN_KEY_V2, feed)
        .await
        .map_err(|e| {
            log::error!("Failed to update global cache v3: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/global-cache/nsfw",
    tag = "update-global-cache-v3",
    responses(
        (status = 200, description = "Feed sent successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(state))]
async fn update_global_cache_nsfw_v3(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let feed = get_global_cache_nsfw_v3(&state.yral_metadata_client)
        .await
        .map_err(|e| {
            log::error!("Failed to get global cache nsfw v3: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    state
        .ml_feed_cache
        .add_global_cache_items_v2(GLOBAL_CACHE_NSFW_KEY_V2, feed)
        .await
        .map_err(|e| {
            log::error!("Failed to update global cache v3: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    Ok(())
}
