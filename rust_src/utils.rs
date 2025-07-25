use chrono::{DateTime, Utc};
use tracing::instrument;
use yral_ml_feed_cache::types::PostItem;
use yral_types::post::PostItemV2;

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
