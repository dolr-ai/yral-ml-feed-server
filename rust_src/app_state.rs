use ic_agent::Agent;
use yral_metadata_client::MetadataClient;
use yral_ml_feed_cache::MLFeedCacheState;

use crate::canister::init_agent;

pub struct AppState {
    pub agent: Agent,
    pub ml_feed_cache: MLFeedCacheState,
    pub yral_metadata_client: MetadataClient<false>,
}

impl AppState {
    pub async fn new() -> Self {
        AppState {
            agent: init_agent().await,
            ml_feed_cache: MLFeedCacheState::new().await,
            yral_metadata_client: MetadataClient::default(),
        }
    }
}
