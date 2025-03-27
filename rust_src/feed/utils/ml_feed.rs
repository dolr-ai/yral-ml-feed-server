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
        .get_ml_feed_clean_v2(request)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get ml_feed_py response: {}", e))?;

    let response_obj = response.into_inner();

    let response_items = response_obj
        .feed
        .iter()
        .map(|x| PostItem {
            canister_id: x.canister_id.clone(),
            post_id: x.post_id as u64,
            video_id: x.video_id.clone(),
            nsfw_probability: x.nsfw_probability,
        })
        .collect::<Vec<PostItem>>();

    Ok(response_items)
}

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
        .get_ml_feed_nsfw_v2(request)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get ml_feed_py response: {}", e))?;

    let response_obj = response.into_inner();

    let response_items = response_obj
        .feed
        .iter()
        .map(|x| PostItem {
            canister_id: x.canister_id.clone(),
            post_id: x.post_id as u64,
            video_id: x.video_id.clone(),
            nsfw_probability: x.nsfw_probability,
        })
        .collect::<Vec<PostItem>>();

    Ok(response_items)
}

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
        .get_ml_feed_combined(request)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get ml_feed_py response: {}", e))?;

    let response_obj = response.into_inner();

    let response_items = response_obj
        .feed
        .iter()
        .map(|x| PostItem {
            canister_id: x.canister_id.clone(),
            post_id: x.post_id as u64,
            video_id: x.video_id.clone(),
            nsfw_probability: x.nsfw_probability,
        })
        .collect::<Vec<PostItem>>();

    Ok(response_items)
}
