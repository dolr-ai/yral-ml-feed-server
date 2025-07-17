use rand::seq::SliceRandom;
use yral_ml_feed_cache::types::PostItem;
use yral_types::post::PostItemV2;

pub mod coldstart_clean_cache;
pub mod coldstart_clean_cache_v3;
pub mod coldstart_mixed_cache;
pub mod coldstart_nsfw_cache;
pub mod coldstart_nsfw_cache_v3;
pub mod global_cache;
pub mod global_cache_v3;
pub mod ml_feed;
pub mod ml_feed_v3;

pub async fn get_shuffled_limit_list(list: Vec<PostItem>, limit: usize) -> Vec<PostItem> {
    let mut rng = rand::rng();
    let mut indices = (0..list.len()).collect::<Vec<_>>();
    indices.shuffle(&mut rng);
    indices
        .into_iter()
        .take(limit)
        .map(|i| list[i].clone())
        .collect()
}

pub async fn get_shuffled_limit_list_v3(list: Vec<PostItemV2>, limit: usize) -> Vec<PostItemV2> {
    let mut rng = rand::rng();
    let mut indices = (0..list.len()).collect::<Vec<_>>();
    indices.shuffle(&mut rng);
    indices
        .into_iter()
        .take(limit)
        .map(|i| list[i].clone())
        .collect()
}
