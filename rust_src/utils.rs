use chrono::{DateTime, Utc};
use tracing::instrument;
use yral_ml_feed_cache::{
    types::PostItem,
    types_v2::MLFeedCacheHistoryItemV2,
    types_v3::MLFeedCacheHistoryItemV3,
};
use yral_types::post::{PostItemV2, PostItemV3};

use crate::canister::individual_user_template::SystemTime;

pub fn to_rfc3339_did_systemtime(dt: &SystemTime) -> String {
    let sys_time = to_system_time(dt.nanos_since_epoch, dt.secs_since_epoch);
    to_rfc3339(sys_time)
}

pub fn to_rfc3339(dt: std::time::SystemTime) -> String {
    let dt: DateTime<Utc> = dt.into();
    dt.to_rfc3339()
}

pub fn to_system_time(nanos_since_epoch: u32, secs_since_epoch: u64) -> std::time::SystemTime {
    let duration = std::time::Duration::new(secs_since_epoch, nanos_since_epoch);
    std::time::SystemTime::UNIX_EPOCH + duration
}

#[instrument(skip_all)]
pub fn remove_duplicates(posts: Vec<PostItem>) -> Vec<PostItem> {
    let mut seen = std::collections::HashSet::new();
    let posts_len = posts.len();

    let res_post = posts
        .into_iter()
        .filter(|post| seen.insert((post.canister_id.clone(), post.post_id)))
        .collect::<Vec<PostItem>>();

    if res_post.len() < posts_len {
        tracing::warn!(
            "Removed {} duplicate posts, original: {}, filtered: {}",
            posts_len - res_post.len(),
            posts_len,
            res_post.len()
        );
    }

    res_post
}

#[instrument(skip_all)]
pub fn remove_duplicates_v2(posts: Vec<PostItemV2>) -> Vec<PostItemV2> {
    let mut seen = std::collections::HashSet::new();
    let posts_len = posts.len();

    let res_post = posts
        .into_iter()
        .filter(|post| seen.insert(post.video_id.clone()))
        .collect::<Vec<PostItemV2>>();

    if res_post.len() < posts_len {
        tracing::warn!(
            "Removed {} duplicate posts2, original: {}, filtered: {}",
            posts_len - res_post.len(),
            posts_len,
            res_post.len()
        );
    }

    res_post
}

/// Converts a vector of PostItemV3 to PostItemV2, filtering out items with non-u64 post_ids
/// 
/// This function:
/// - Attempts to parse each PostItemV3's post_id (String) as u64
/// - Filters out items where the post_id cannot be parsed as u64
/// - Returns only the successfully converted items
#[instrument(skip_all)]
pub fn convert_post_items_v3_to_v2(items: Vec<PostItemV3>) -> Vec<PostItemV2> {
    let original_count = items.len();
    
    let converted: Vec<PostItemV2> = items
        .into_iter()
        .filter_map(|item| {
            // Try to parse post_id as u64
            match item.post_id.parse::<u64>() {
                Ok(post_id) => {
                    // Successfully parsed, create PostItemV2
                    Some(PostItemV2 {
                        publisher_user_id: item.publisher_user_id,
                        canister_id: item.canister_id,
                        post_id,
                        video_id: item.video_id,
                        is_nsfw: item.is_nsfw,
                    })
                }
                Err(_) => {
                    // Cannot parse as u64, filter out this item
                    tracing::debug!(
                        "Filtered out PostItemV3 with non-u64 post_id: {}",
                        item.post_id
                    );
                    None
                }
            }
        })
        .collect();
    
    let filtered_count = original_count - converted.len();
    if filtered_count > 0 {
        tracing::info!(
            "Converted PostItemV3 to PostItemV2: {} items converted, {} items filtered out (non-u64 post_ids)",
            converted.len(),
            filtered_count
        );
    }
    
    converted
}

/// Converts a vector of MLFeedCacheHistoryItemV3 to MLFeedCacheHistoryItemV2, filtering out items with non-u64 post_ids
/// 
/// This function:
/// - Attempts to parse each MLFeedCacheHistoryItemV3's post_id (String) as u64
/// - Filters out items where the post_id cannot be parsed as u64
/// - Returns only the successfully converted items
#[instrument(skip_all)]
pub fn convert_history_items_v3_to_v2(items: Vec<MLFeedCacheHistoryItemV3>) -> Vec<MLFeedCacheHistoryItemV2> {
    let original_count = items.len();
    
    let converted: Vec<MLFeedCacheHistoryItemV2> = items
        .into_iter()
        .filter_map(|item| {
            // Try to parse post_id as u64
            match item.post_id.parse::<u64>() {
                Ok(post_id) => {
                    // Successfully parsed, create MLFeedCacheHistoryItemV2
                    Some(MLFeedCacheHistoryItemV2 {
                        publisher_user_id: item.publisher_user_id,
                        canister_id: item.canister_id,
                        post_id,
                        video_id: item.video_id,
                        item_type: item.item_type,
                        timestamp: item.timestamp,
                        percent_watched: item.percent_watched,
                    })
                }
                Err(_) => {
                    // Cannot parse as u64, filter out this item
                    tracing::debug!(
                        "Filtered out MLFeedCacheHistoryItemV3 with non-u64 post_id: {}",
                        item.post_id
                    );
                    None
                }
            }
        })
        .collect();
    
    let filtered_count = original_count - converted.len();
    if filtered_count > 0 {
        tracing::info!(
            "Converted MLFeedCacheHistoryItemV3 to V2: {} items converted, {} items filtered out (non-u64 post_ids)",
            converted.len(),
            filtered_count
        );
    }
    
    converted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_post_items_v3_to_v2() {
        // Create test PostItemV3 items with different post_id formats
        let items_v3 = vec![
            PostItemV3 {
                publisher_user_id: "user1".to_string(),
                canister_id: "canister1".to_string(),
                post_id: "12345".to_string(), // Valid u64
                video_id: "video1".to_string(),
                is_nsfw: false,
            },
            PostItemV3 {
                publisher_user_id: "user2".to_string(),
                canister_id: "canister2".to_string(),
                post_id: "xyz789".to_string(), // Invalid - non-numeric
                video_id: "video2".to_string(),
                is_nsfw: true,
            },
            PostItemV3 {
                publisher_user_id: "user3".to_string(),
                canister_id: "canister3".to_string(),
                post_id: "99999999999999999999999".to_string(), // Invalid - too large for u64
                video_id: "video3".to_string(),
                is_nsfw: false,
            },
            PostItemV3 {
                publisher_user_id: "user4".to_string(),
                canister_id: "canister4".to_string(),
                post_id: "67890".to_string(), // Valid u64
                video_id: "video4".to_string(),
                is_nsfw: true,
            },
        ];

        // Convert
        let items_v2 = convert_post_items_v3_to_v2(items_v3);

        // Verify only valid items were converted
        assert_eq!(items_v2.len(), 2, "Should have 2 valid items after conversion");
        
        // Check first valid item
        assert_eq!(items_v2[0].post_id, 12345);
        assert_eq!(items_v2[0].publisher_user_id, "user1");
        assert_eq!(items_v2[0].video_id, "video1");
        assert_eq!(items_v2[0].is_nsfw, false);
        
        // Check second valid item
        assert_eq!(items_v2[1].post_id, 67890);
        assert_eq!(items_v2[1].publisher_user_id, "user4");
        assert_eq!(items_v2[1].video_id, "video4");
        assert_eq!(items_v2[1].is_nsfw, true);
    }

    #[test]
    fn test_convert_post_items_v3_to_v2_empty() {
        let items_v3 = vec![];
        let items_v2 = convert_post_items_v3_to_v2(items_v3);
        assert_eq!(items_v2.len(), 0, "Empty input should return empty output");
    }

    #[test]
    fn test_convert_post_items_v3_to_v2_all_invalid() {
        let items_v3 = vec![
            PostItemV3 {
                publisher_user_id: "user1".to_string(),
                canister_id: "canister1".to_string(),
                post_id: "not-a-number".to_string(),
                video_id: "video1".to_string(),
                is_nsfw: false,
            },
            PostItemV3 {
                publisher_user_id: "user2".to_string(),
                canister_id: "canister2".to_string(),
                post_id: "abc123".to_string(),
                video_id: "video2".to_string(),
                is_nsfw: true,
            },
        ];
        
        let items_v2 = convert_post_items_v3_to_v2(items_v3);
        assert_eq!(items_v2.len(), 0, "All invalid items should be filtered out");
    }

    #[test]
    fn test_convert_history_items_v3_to_v2() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let base_time = SystemTime::now();
        
        // Create test MLFeedCacheHistoryItemV3 items with different post_id formats
        let items_v3 = vec![
            MLFeedCacheHistoryItemV3 {
                publisher_user_id: "user1".to_string(),
                canister_id: "canister1".to_string(),
                post_id: "12345".to_string(), // Valid u64
                video_id: "video1".to_string(),
                item_type: "like_video".to_string(),
                timestamp: base_time,
                percent_watched: 75.5,
            },
            MLFeedCacheHistoryItemV3 {
                publisher_user_id: "user2".to_string(),
                canister_id: "canister2".to_string(),
                post_id: "abc123".to_string(), // Invalid - non-numeric
                video_id: "video2".to_string(),
                item_type: "video_watched".to_string(),
                timestamp: base_time,
                percent_watched: 50.0,
            },
            MLFeedCacheHistoryItemV3 {
                publisher_user_id: "user3".to_string(),
                canister_id: "canister3".to_string(),
                post_id: "67890".to_string(), // Valid u64
                video_id: "video3".to_string(),
                item_type: "share_video".to_string(),
                timestamp: base_time,
                percent_watched: 100.0,
            },
        ];

        // Convert
        let items_v2 = convert_history_items_v3_to_v2(items_v3);

        // Verify only valid items were converted
        assert_eq!(items_v2.len(), 2, "Should have 2 valid items after conversion");
        
        // Check first valid item
        assert_eq!(items_v2[0].post_id, 12345);
        assert_eq!(items_v2[0].publisher_user_id, "user1");
        assert_eq!(items_v2[0].video_id, "video1");
        assert_eq!(items_v2[0].item_type, "like_video");
        assert_eq!(items_v2[0].percent_watched, 75.5);
        
        // Check second valid item
        assert_eq!(items_v2[1].post_id, 67890);
        assert_eq!(items_v2[1].publisher_user_id, "user3");
        assert_eq!(items_v2[1].video_id, "video3");
        assert_eq!(items_v2[1].item_type, "share_video");
        assert_eq!(items_v2[1].percent_watched, 100.0);
    }

    #[test]
    fn test_convert_history_items_v3_to_v2_empty() {
        let items_v3 = vec![];
        let items_v2 = convert_history_items_v3_to_v2(items_v3);
        assert_eq!(items_v2.len(), 0, "Empty input should return empty output");
    }
}