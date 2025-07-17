use std::collections::HashSet;

use tracing::instrument;
use yral_ml_feed_cache::{
    consts::{
        MAX_WATCH_HISTORY_CACHE_LEN, USER_SUCCESS_HISTORY_CLEAN_SUFFIX,
        USER_SUCCESS_HISTORY_NSFW_SUFFIX, USER_WATCH_HISTORY_CLEAN_SUFFIX,
        USER_WATCH_HISTORY_NSFW_SUFFIX,
    },
    types::{MLFeedCacheHistoryItem, PostItem},
    MLFeedCacheState,
};

use crate::{
    consts::ML_FEED_PY_SERVER,
    grpc_services::ml_feed_py::{self, ml_feed_client::MlFeedClient},
    utils::to_rfc3339,
};

#[instrument(skip(ml_feed_cache, filter_results))]
pub async fn get_ml_feed_clean_impl(
    ml_feed_cache: MLFeedCacheState,
    canister_id: String,
    filter_results: Vec<PostItem>,
    num_results: u32,
) -> Result<Vec<PostItem>, anyhow::Error> {
    let key = format!("{}{}", canister_id, USER_WATCH_HISTORY_CLEAN_SUFFIX);

    let watch_history = ml_feed_cache
        .get_history_items(&key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await?;

    let watch_history_items = watch_history
        .iter()
        .map(|x| ml_feed_py::WatchHistoryItem {
            post_id: x.post_id as u32,
            canister_id: x.canister_id.clone(),
            video_id: x.video_id.clone(),
            percent_watched: x.percent_watched,
            timestamp: to_rfc3339(x.timestamp),
        })
        .collect::<Vec<ml_feed_py::WatchHistoryItem>>();

    let key = format!("{}{}", canister_id, USER_SUCCESS_HISTORY_CLEAN_SUFFIX);

    let success_history = ml_feed_cache
        .get_history_items(&key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await?;

    let success_history_items = success_history
        .iter()
        .map(|x| ml_feed_py::SuccessHistoryItem {
            post_id: x.post_id as u32,
            canister_id: x.canister_id.clone(),
            video_id: x.video_id.clone(),
            item_type: x.item_type.clone(),
            percent_watched: x.percent_watched,
            timestamp: to_rfc3339(x.timestamp),
        })
        .collect::<Vec<ml_feed_py::SuccessHistoryItem>>();

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
        .map(|x| ml_feed_py::MlPostItem {
            canister_id: x.canister_id.clone(),
            post_id: x.post_id as u32,
            video_id: x.video_id.clone(),
        })
        .collect::<Vec<ml_feed_py::MlPostItem>>();

    let request = tonic::Request::new(ml_feed_py::MlFeedRequest {
        canister_id,
        watch_history: watch_history_items,
        success_history: success_history_items,
        filter_posts: filter_items,
        num_results,
    });

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
        .get_ml_feed_clean_v2_deduped(request)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get ml_feed_py response: {}", e))?;

    let response_obj = response.into_inner();

    let original_response_len = response_obj.feed.len();

    let response_items = response_obj
        .feed
        .iter()
        .filter(|x| !filter_video_ids.contains(&x.video_id))
        .map(|x| PostItem {
            canister_id: x.canister_id.clone(),
            post_id: x.post_id as u64,
            video_id: x.video_id.clone(),
            nsfw_probability: x.nsfw_probability,
        })
        .collect::<Vec<PostItem>>();

    if response_items.len() < original_response_len {
        tracing::warn!(
            "Removed {} duplicate posts, original: {}, filtered: {}",
            original_response_len - response_items.len(),
            original_response_len,
            response_items.len()
        );
    }

    Ok(response_items)
}

#[instrument(skip(ml_feed_cache, filter_results))]
pub async fn get_ml_feed_nsfw_impl(
    ml_feed_cache: MLFeedCacheState,
    canister_id: String,
    filter_results: Vec<PostItem>,
    num_results: u32,
) -> Result<Vec<PostItem>, anyhow::Error> {
    let key = format!("{}{}", canister_id, USER_WATCH_HISTORY_NSFW_SUFFIX);

    let watch_history = ml_feed_cache
        .get_history_items(&key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await?;

    let watch_history_items = watch_history
        .iter()
        .map(|x| ml_feed_py::WatchHistoryItem {
            post_id: x.post_id as u32,
            canister_id: x.canister_id.clone(),
            video_id: x.video_id.clone(),
            percent_watched: x.percent_watched,
            timestamp: to_rfc3339(x.timestamp),
        })
        .collect::<Vec<ml_feed_py::WatchHistoryItem>>();

    let key = format!("{}{}", canister_id, USER_SUCCESS_HISTORY_NSFW_SUFFIX);

    let success_history = ml_feed_cache
        .get_history_items(&key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await?;

    let success_history_items = success_history
        .iter()
        .map(|x| ml_feed_py::SuccessHistoryItem {
            post_id: x.post_id as u32,
            canister_id: x.canister_id.clone(),
            video_id: x.video_id.clone(),
            item_type: x.item_type.clone(),
            percent_watched: x.percent_watched,
            timestamp: to_rfc3339(x.timestamp),
        })
        .collect::<Vec<ml_feed_py::SuccessHistoryItem>>();

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
        .map(|x| ml_feed_py::MlPostItem {
            canister_id: x.canister_id.clone(),
            post_id: x.post_id as u32,
            video_id: x.video_id.clone(),
        })
        .collect::<Vec<ml_feed_py::MlPostItem>>();

    let request = tonic::Request::new(ml_feed_py::MlFeedRequest {
        canister_id,
        watch_history: watch_history_items,
        success_history: success_history_items,
        filter_posts: filter_items,
        num_results,
    });

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
        .get_ml_feed_nsfw_v2_deduped(request)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get ml_feed_py response: {}", e))?;

    let response_obj = response.into_inner();
    let original_response_len = response_obj.feed.len();

    let response_items = response_obj
        .feed
        .iter()
        .filter(|x| !filter_video_ids.contains(&x.video_id))
        .map(|x| PostItem {
            canister_id: x.canister_id.clone(),
            post_id: x.post_id as u64,
            video_id: x.video_id.clone(),
            nsfw_probability: x.nsfw_probability,
        })
        .collect::<Vec<PostItem>>();

    if response_items.len() < original_response_len {
        tracing::warn!(
            "Removed {} duplicate posts, original: {}, filtered: {}",
            original_response_len - response_items.len(),
            original_response_len,
            response_items.len()
        );
    }

    Ok(response_items)
}

#[instrument(skip(ml_feed_cache, filter_results))]
pub async fn get_ml_feed_mixed_impl(
    ml_feed_cache: MLFeedCacheState,
    canister_id: String,
    filter_results: Vec<PostItem>,
    num_results: u32,
) -> Result<Vec<PostItem>, anyhow::Error> {
    let key = format!("{}{}", canister_id, USER_WATCH_HISTORY_CLEAN_SUFFIX);

    let watch_history_clean = ml_feed_cache
        .get_history_items(&key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await?;

    let key = format!("{}{}", canister_id, USER_WATCH_HISTORY_NSFW_SUFFIX);

    let watch_history_nsfw = ml_feed_cache
        .get_history_items(&key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await?;

    let watch_history = watch_history_clean
        .iter()
        .chain(watch_history_nsfw.iter())
        .collect::<Vec<&MLFeedCacheHistoryItem>>();

    let watch_history_items = watch_history
        .iter()
        .map(|x| ml_feed_py::WatchHistoryItem {
            post_id: x.post_id as u32,
            canister_id: x.canister_id.clone(),
            video_id: x.video_id.clone(),
            percent_watched: x.percent_watched,
            timestamp: to_rfc3339(x.timestamp),
        })
        .collect::<Vec<ml_feed_py::WatchHistoryItem>>();

    let key = format!("{}{}", canister_id, USER_SUCCESS_HISTORY_CLEAN_SUFFIX);

    let success_history_clean = ml_feed_cache
        .get_history_items(&key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await?;

    let key = format!("{}{}", canister_id, USER_SUCCESS_HISTORY_NSFW_SUFFIX);

    let success_history_nsfw = ml_feed_cache
        .get_history_items(&key, 0, MAX_WATCH_HISTORY_CACHE_LEN)
        .await?;

    let success_history = success_history_clean
        .iter()
        .chain(success_history_nsfw.iter())
        .collect::<Vec<&MLFeedCacheHistoryItem>>();

    let success_history_items = success_history
        .iter()
        .map(|x| ml_feed_py::SuccessHistoryItem {
            post_id: x.post_id as u32,
            canister_id: x.canister_id.clone(),
            video_id: x.video_id.clone(),
            item_type: x.item_type.clone(),
            percent_watched: x.percent_watched,
            timestamp: to_rfc3339(x.timestamp),
        })
        .collect::<Vec<ml_feed_py::SuccessHistoryItem>>();

    // Create a HashSet of video_ids from filter_results and all history data
    let mut filter_video_ids: HashSet<String> = filter_results
        .iter()
        .map(|item| item.video_id.clone())
        .collect();

    // Add watch history video_ids (both clean and nsfw)
    filter_video_ids.extend(watch_history_clean.iter().map(|item| item.video_id.clone()));
    filter_video_ids.extend(watch_history_nsfw.iter().map(|item| item.video_id.clone()));

    // Add success history video_ids (both clean and nsfw)
    filter_video_ids.extend(
        success_history_clean
            .iter()
            .map(|item| item.video_id.clone()),
    );
    filter_video_ids.extend(
        success_history_nsfw
            .iter()
            .map(|item| item.video_id.clone()),
    );

    let filter_items = filter_results
        .iter()
        .map(|x| ml_feed_py::MlPostItem {
            canister_id: x.canister_id.clone(),
            post_id: x.post_id as u32,
            video_id: x.video_id.clone(),
        })
        .collect::<Vec<ml_feed_py::MlPostItem>>();

    let request = tonic::Request::new(ml_feed_py::MlFeedRequest {
        canister_id,
        watch_history: watch_history_items,
        success_history: success_history_items,
        filter_posts: filter_items,
        num_results,
    });

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
        .get_ml_feed_combined_deduped(request)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get ml_feed_py response: {}", e))?;

    let response_obj = response.into_inner();
    let original_response_len = response_obj.feed.len();

    let response_items = response_obj
        .feed
        .iter()
        .filter(|x| !filter_video_ids.contains(&x.video_id))
        .map(|x| PostItem {
            canister_id: x.canister_id.clone(),
            post_id: x.post_id as u64,
            video_id: x.video_id.clone(),
            nsfw_probability: x.nsfw_probability,
        })
        .collect::<Vec<PostItem>>();

    if response_items.len() < original_response_len {
        tracing::warn!(
            "Removed {} duplicate posts, original: {}, filtered: {}",
            original_response_len - response_items.len(),
            original_response_len,
            response_items.len()
        );
    }

    Ok(response_items)
}
