use std::collections::HashSet;

use candid::Principal;
use tracing::instrument;
use yral_metadata_client::MetadataClient;
use yral_ml_feed_cache::{
    consts::{
        MAX_WATCH_HISTORY_CACHE_LEN, USER_SUCCESS_HISTORY_CLEAN_SUFFIX_V2,
        USER_SUCCESS_HISTORY_NSFW_SUFFIX_V2, USER_WATCH_HISTORY_CLEAN_SUFFIX_V2,
        USER_WATCH_HISTORY_NSFW_SUFFIX_V2,
    },
    MLFeedCacheState,
};
use yral_types::post::PostItemV2;

use crate::{
    consts::ML_FEED_PY_SERVER,
    grpc_services::ml_feed_py::{self, ml_feed_client::MlFeedClient},
    utils::{convert_history_items_v3_to_v2, remove_duplicates_v2, to_rfc3339},
};

#[instrument(skip(ml_feed_cache, metadata_client, filter_results))]
pub async fn get_ml_feed_clean_v3_impl(
    ml_feed_cache: MLFeedCacheState,
    user_id: String,
    filter_results: Vec<PostItemV2>,
    num_results: u32,
    metadata_client: &MetadataClient<false>,
) -> Result<Vec<PostItemV2>, anyhow::Error> {
    let key = format!("{}{}", user_id, USER_WATCH_HISTORY_CLEAN_SUFFIX_V2);

    let watch_history_v3 = ml_feed_cache
        .get_watch_history_items_v3_resilient(&key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await?;

    // Convert MLFeedCacheHistoryItemV3 to V2, filtering out non-u64 post_ids
    let watch_history = convert_history_items_v3_to_v2(watch_history_v3);

    let watch_history_items = watch_history
        .iter()
        .map(|x| ml_feed_py::WatchHistoryItemV3 {
            post_id: x.post_id as u32,
            publisher_user_id: x.publisher_user_id.clone(),
            video_id: x.video_id.clone(),
            percent_watched: x.percent_watched,
            timestamp: to_rfc3339(x.timestamp),
        })
        .collect::<Vec<ml_feed_py::WatchHistoryItemV3>>();

    let key = format!("{}{}", user_id, USER_SUCCESS_HISTORY_CLEAN_SUFFIX_V2);

    let success_history_v3 = ml_feed_cache
        .get_watch_history_items_v3_resilient(&key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await?;

    // Convert MLFeedCacheHistoryItemV3 to V2, filtering out non-u64 post_ids
    let success_history = convert_history_items_v3_to_v2(success_history_v3);

    let success_history_items = success_history
        .iter()
        .map(|x| ml_feed_py::SuccessHistoryItemV3 {
            post_id: x.post_id as u32,
            publisher_user_id: x.publisher_user_id.clone(),
            video_id: x.video_id.clone(),
            item_type: x.item_type.clone(),
            percent_watched: x.percent_watched,
            timestamp: to_rfc3339(x.timestamp),
        })
        .collect::<Vec<ml_feed_py::SuccessHistoryItemV3>>();

    // Create a HashSet of video_ids from filter_results, watch history, and success history
    let mut filter_video_ids: HashSet<String> = filter_results
        .iter()
        .map(|item| item.video_id.clone())
        .collect();

    // Add watch history video_ids
    filter_video_ids.extend(watch_history.iter().map(|item| item.video_id.clone()));

    // Add success history video_ids
    filter_video_ids.extend(success_history.iter().map(|item| item.video_id.clone()));

    let filter_items = filter_results
        .iter()
        .map(|x| ml_feed_py::MlPostItemV3 {
            publisher_user_id: x.publisher_user_id.clone(),
            post_id: x.post_id as u32,
            video_id: x.video_id.clone(),
        })
        .collect::<Vec<ml_feed_py::MlPostItemV3>>();

    let request_data = ml_feed_py::MlFeedRequestV3 {
        user_principal_id: user_id,
        watch_history: watch_history_items,
        success_history: success_history_items,
        filter_posts: filter_items,
        num_results,
    };

    let request = tonic::Request::new(request_data.clone());

    let mut client = match MlFeedClient::connect(
        ML_FEED_PY_SERVER, // http://python_proc.process.yral-ml-feed-server.internal:50059"
    )
    .await
    {
        Ok(client) => client,
        Err(e) => {
            log::error!("Failed to connect to ml_feed_py server: {:?}", e);
            return Err(anyhow::anyhow!("Failed to connect to ml_feed_py server"));
        }
    };

    let response = client
        .get_ml_feed_clean_v3(request)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get get_ml_feed_clean_v3 response: {}", e))?;

    let response_obj = response.into_inner();
    let original_response_len = response_obj.feed.len();

    let response_items = response_obj
        .feed
        .iter()
        .filter(|x| !filter_video_ids.contains(&x.video_id))
        .map(|x| PostItemV2 {
            publisher_user_id: x.publisher_user_id.clone(),
            canister_id: String::new(),
            post_id: x.post_id as u64,
            video_id: x.video_id.clone(),
            is_nsfw: x.nsfw_probability > 0.4,
        })
        .collect::<Vec<PostItemV2>>();

    if response_items.len() < original_response_len {
        tracing::warn!(
            "Removed {} posts after explicit filtering, original: {}, filtered: {}",
            original_response_len - response_items.len(),
            original_response_len,
            response_items.len()
        );

        if original_response_len - response_items.len() > 20 {
            tracing::error!(
                "HIGH_FILTER_RATE_ALERT: Request details: {:?}",
                request_data
            );
        }
    }

    // Fill canister_ids and filter out posts without metadata
    let mut response_items = fill_canister_ids(response_items, metadata_client).await?;

    response_items = remove_duplicates_v2(response_items);

    Ok(response_items)
}

#[instrument(skip(ml_feed_cache, metadata_client, filter_results))]
pub async fn get_ml_feed_nsfw_v3_impl(
    ml_feed_cache: MLFeedCacheState,
    user_id: String,
    filter_results: Vec<PostItemV2>,
    num_results: u32,
    metadata_client: &MetadataClient<false>,
) -> Result<Vec<PostItemV2>, anyhow::Error> {
    let key = format!("{}{}", user_id, USER_WATCH_HISTORY_NSFW_SUFFIX_V2);

    let watch_history_v3 = ml_feed_cache
        .get_watch_history_items_v3_resilient(&key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await?;

    // Convert MLFeedCacheHistoryItemV3 to V2, filtering out non-u64 post_ids
    let watch_history = convert_history_items_v3_to_v2(watch_history_v3);

    let watch_history_items = watch_history
        .iter()
        .map(|x| ml_feed_py::WatchHistoryItemV3 {
            post_id: x.post_id as u32,
            publisher_user_id: x.publisher_user_id.clone(),
            video_id: x.video_id.clone(),
            percent_watched: x.percent_watched,
            timestamp: to_rfc3339(x.timestamp),
        })
        .collect::<Vec<ml_feed_py::WatchHistoryItemV3>>();

    let key = format!("{}{}", user_id, USER_SUCCESS_HISTORY_NSFW_SUFFIX_V2);

    let success_history_v3 = ml_feed_cache
        .get_watch_history_items_v3_resilient(&key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await?;

    // Convert MLFeedCacheHistoryItemV3 to V2, filtering out non-u64 post_ids
    let success_history = convert_history_items_v3_to_v2(success_history_v3);

    let success_history_items = success_history
        .iter()
        .map(|x| ml_feed_py::SuccessHistoryItemV3 {
            post_id: x.post_id as u32,
            publisher_user_id: x.publisher_user_id.clone(),
            video_id: x.video_id.clone(),
            item_type: x.item_type.clone(),
            percent_watched: x.percent_watched,
            timestamp: to_rfc3339(x.timestamp),
        })
        .collect::<Vec<ml_feed_py::SuccessHistoryItemV3>>();

    // Create a HashSet of video_ids from filter_results, watch history, and success history
    let mut filter_video_ids: HashSet<String> = filter_results
        .iter()
        .map(|item| item.video_id.clone())
        .collect();

    // Add watch history video_ids
    filter_video_ids.extend(watch_history.iter().map(|item| item.video_id.clone()));

    // Add success history video_ids
    filter_video_ids.extend(success_history.iter().map(|item| item.video_id.clone()));

    let filter_items = filter_results
        .iter()
        .map(|x| ml_feed_py::MlPostItemV3 {
            publisher_user_id: x.publisher_user_id.clone(),
            post_id: x.post_id as u32,
            video_id: x.video_id.clone(),
        })
        .collect::<Vec<ml_feed_py::MlPostItemV3>>();

    let request_data = ml_feed_py::MlFeedRequestV3 {
        user_principal_id: user_id,
        watch_history: watch_history_items,
        success_history: success_history_items,
        filter_posts: filter_items,
        num_results,
    };

    let request = tonic::Request::new(request_data.clone());

    let mut client = match MlFeedClient::connect(
        ML_FEED_PY_SERVER, // http://python_proc.process.yral-ml-feed-server.internal:50059"
    )
    .await
    {
        Ok(client) => client,
        Err(e) => {
            log::error!("Failed to connect to ml_feed_py server: {:?}", e);
            return Err(anyhow::anyhow!("Failed to connect to ml_feed_py server"));
        }
    };

    let response = client
        .get_ml_feed_nsfw_v3(request)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get get_ml_feed_nsfw_v3 response: {}", e))?;

    let response_obj = response.into_inner();
    let original_response_len = response_obj.feed.len();

    let response_items = response_obj
        .feed
        .iter()
        .filter(|x| !filter_video_ids.contains(&x.video_id))
        .map(|x| PostItemV2 {
            publisher_user_id: x.publisher_user_id.clone(),
            canister_id: String::new(),
            post_id: x.post_id as u64,
            video_id: x.video_id.clone(),
            is_nsfw: x.nsfw_probability > 0.4,
        })
        .collect::<Vec<PostItemV2>>();

    if response_items.len() < original_response_len {
        tracing::warn!(
            "Removed {} posts after explicit filtering, original: {}, filtered: {}",
            original_response_len - response_items.len(),
            original_response_len,
            response_items.len()
        );

        if original_response_len - response_items.len() > 20 {
            tracing::error!(
                "HIGH_FILTER_RATE_ALERT: Request details: {:?}",
                request_data
            );
        }
    }

    // Fill canister_ids and filter out posts without metadata
    let mut response_items = fill_canister_ids(response_items, metadata_client).await?;

    response_items = remove_duplicates_v2(response_items);

    Ok(response_items)
}

#[instrument(skip(metadata_client, posts))]
pub async fn fill_canister_ids(
    posts: Vec<PostItemV2>,
    metadata_client: &MetadataClient<false>,
) -> Result<Vec<PostItemV2>, anyhow::Error> {
    if posts.is_empty() {
        return Ok(posts);
    }

    // Extract unique publisher_user_ids
    let publisher_user_ids: Vec<Principal> = posts
        .iter()
        .map(|post| Principal::from_text(&post.publisher_user_id).unwrap())
        .collect::<std::collections::HashSet<Principal>>()
        .into_iter()
        .collect();

    // Get metadata for all users
    let user_metadata_map = metadata_client
        .get_user_metadata_bulk(publisher_user_ids)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get user metadata bulk: {}", e))?;

    // Filter posts and fill canister_ids for posts with metadata
    let original_posts_len = posts.len();
    let filtered_posts: Vec<PostItemV2> = posts
        .into_iter()
        .filter_map(|mut post| {
            if let Some(Some(metadata)) =
                user_metadata_map.get(&Principal::from_text(&post.publisher_user_id).unwrap())
            {
                post.canister_id = metadata.user_canister_id.to_string();
                return Some(post);
            }
            None
        })
        .collect();

    if filtered_posts.len() < original_posts_len {
        tracing::warn!(
            "Removed {} posts without metadata, original: {}, filtered: {}",
            original_posts_len - filtered_posts.len(),
            original_posts_len,
            filtered_posts.len()
        );
    }

    Ok(filtered_posts)
}
