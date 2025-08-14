use yral_metadata_client::MetadataClient;
use yral_types::post::PostItemV2;

use crate::{
    consts::ML_FEED_PY_SERVER,
    grpc_services::ml_feed_py::{self, ml_feed_client::MlFeedClient},
    utils::remove_duplicates_v2,
};

use super::ml_feed_v3::fill_canister_ids;

pub async fn get_global_cache_clean_v3(
    metadata_client: &MetadataClient<false>,
) -> Result<Vec<PostItemV2>, anyhow::Error> {
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

    let request = tonic::Request::new(ml_feed_py::MlFeedRequestV3 {
        user_principal_id: "".to_string(),
        watch_history: vec![],
        success_history: vec![],
        filter_posts: vec![],
        num_results: 3000,
    });

    let response = client.get_ml_feed_clean_v3(request).await.map_err(|e| {
        log::error!("Failed to get get_ml_feed_clean_v3 response: {}", e);
        anyhow::anyhow!("Failed to get get_ml_feed_clean_v3 response: {}", e)
    })?;

    let response_obj = response.into_inner();

    let response_items = response_obj
        .feed
        .iter()
        .map(|x| PostItemV2 {
            publisher_user_id: x.publisher_user_id.clone(),
            canister_id: "".to_string(),
            post_id: x.post_id as u64,
            video_id: x.video_id.clone(),
            is_nsfw: x.nsfw_probability > 0.4,
        })
        .collect::<Vec<PostItemV2>>();

    // Fill canister_ids and filter out posts without metadata
    let mut response_items = fill_canister_ids(response_items, metadata_client).await?;

    response_items = remove_duplicates_v2(response_items);

    Ok(response_items)
}

pub async fn get_global_cache_nsfw_v3(
    metadata_client: &MetadataClient<false>,
) -> Result<Vec<PostItemV2>, anyhow::Error> {
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

    let request = tonic::Request::new(ml_feed_py::MlFeedRequestV3 {
        user_principal_id: "".to_string(),
        watch_history: vec![],
        success_history: vec![],
        filter_posts: vec![],
        num_results: 3000,
    });

    let response = client.get_ml_feed_nsfw_v3(request).await.map_err(|e| {
        log::error!("Failed to get get_ml_feed_nsfw_v3 response: {}", e);
        anyhow::anyhow!("Failed to get get_ml_feed_nsfw_v3 response: {}", e)
    })?;

    let response_obj = response.into_inner();

    let response_items = response_obj
        .feed
        .iter()
        .map(|x| PostItemV2 {
            publisher_user_id: x.publisher_user_id.clone(),
            canister_id: "".to_string(),
            post_id: x.post_id as u64,
            video_id: x.video_id.clone(),
            is_nsfw: x.nsfw_probability > 0.4,
        })
        .collect::<Vec<PostItemV2>>();

    // Fill canister_ids and filter out posts without metadata
    let mut response_items = fill_canister_ids(response_items, metadata_client).await?;

    response_items = remove_duplicates_v2(response_items);

    Ok(response_items)
}
