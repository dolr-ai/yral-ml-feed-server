use std::collections::HashSet;

use anyhow::Ok;
use rand::Rng;
use yral_ml_feed_cache::{
    consts::{
        GLOBAL_CACHE_NSFW_KEY_V2, MAX_GLOBAL_CACHE_LEN, MAX_WATCH_HISTORY_CACHE_LEN,
        USER_CACHE_NSFW_SUFFIX_V2, USER_WATCH_HISTORY_NSFW_SUFFIX_V2,
    },
    MLFeedCacheState,
};
use yral_types::post::PostItemV2;

use crate::{feed::utils::get_shuffled_limit_list_v3, utils::remove_duplicates_v2};

pub async fn get_coldstart_nsfw_cache_noinput_impl(
    ml_feed_cache: MLFeedCacheState,
) -> Result<Vec<PostItemV2>, anyhow::Error> {
    let num_posts_in_cache = ml_feed_cache
        .get_cache_items_len(GLOBAL_CACHE_NSFW_KEY_V2)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get num posts in cache: {}", e))?;

    let post_index = rand::rng().random_range(0..num_posts_in_cache);
    let feed = ml_feed_cache
        .get_cache_items_v2(GLOBAL_CACHE_NSFW_KEY_V2, post_index, post_index + 1)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get post from cache: {}", e))?;

    Ok(remove_duplicates_v2(feed))
}

pub async fn get_coldstart_nsfw_cache_noinput_user_impl(
    ml_feed_cache: MLFeedCacheState,
    user_id: String,
) -> Result<Vec<PostItemV2>, anyhow::Error> {
    let num_posts_in_cache = ml_feed_cache
        .get_cache_items_len(&format!("{}{}", user_id, USER_CACHE_NSFW_SUFFIX_V2))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get num posts in cache: {}", e))?;

    if num_posts_in_cache == 0 {
        return get_coldstart_nsfw_cache_noinput_impl(ml_feed_cache).await;
    }

    let feed = get_coldstart_nsfw_cache_input_user_impl(ml_feed_cache, user_id, 1, vec![]).await?;

    Ok(remove_duplicates_v2(feed))
}

pub async fn get_coldstart_nsfw_cache_input_impl(
    ml_feed_cache: MLFeedCacheState,
    num_results: u32,
) -> Result<Vec<PostItemV2>, anyhow::Error> {
    let global_cache_nsfw_feed = ml_feed_cache
        .get_cache_items_v2(GLOBAL_CACHE_NSFW_KEY_V2, 0, MAX_GLOBAL_CACHE_LEN)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get global cache nsfw feed: {}", e))?;

    Ok(get_shuffled_limit_list_v3(
        remove_duplicates_v2(global_cache_nsfw_feed),
        num_results as usize,
    )
    .await)
}

pub async fn get_coldstart_nsfw_cache_input_user_impl(
    ml_feed_cache: MLFeedCacheState,
    user_id: String,
    num_results: u32,
    filter_results: Vec<PostItemV2>,
) -> Result<Vec<PostItemV2>, anyhow::Error> {
    let num_posts_in_cache = ml_feed_cache
        .get_cache_items_len(&format!("{}{}", user_id, USER_CACHE_NSFW_SUFFIX_V2))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get num posts in cache: {}", e))?;

    if num_posts_in_cache == 0 {
        return get_coldstart_nsfw_cache_input_impl(ml_feed_cache, num_results).await;
    }

    let watch_history = ml_feed_cache
        .get_history_items_v2(
            &format!("{}{}", user_id, USER_WATCH_HISTORY_NSFW_SUFFIX_V2),
            0,
            MAX_WATCH_HISTORY_CACHE_LEN,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get watch history: {}", e))?;

    // create a set of video_ids from watch_history
    let mut watch_history_video_ids = watch_history
        .iter()
        .map(|item| item.video_id.clone())
        .collect::<HashSet<String>>();

    for item in filter_results {
        watch_history_video_ids.insert(item.video_id.clone());
    }

    let user_cache_items = ml_feed_cache
        .get_cache_items_v2(
            &format!("{}{}", user_id, USER_CACHE_NSFW_SUFFIX_V2),
            0,
            num_posts_in_cache,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get user cache items: {}", e))?;

    let mut feed = Vec::new();
    for item in user_cache_items {
        if !watch_history_video_ids.contains(&item.video_id) {
            feed.push(item);
        }
    }

    Ok(get_shuffled_limit_list_v3(remove_duplicates_v2(feed), num_results as usize).await)
}
