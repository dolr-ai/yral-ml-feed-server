#[cfg(feature = "local-bin")]
pub const ML_FEED_PY_SERVER: &str = "http://localhost:50059"; //"http://python_proc.process.yral-ml-feed-server.internal:50059";

#[cfg(feature = "staging")]
pub const ML_FEED_PY_SERVER: &str =
    "http://python_proc.process.yral-ml-feed-server-staging.internal:50059";

#[cfg(not(any(feature = "local-bin", feature = "staging")))]
pub const ML_FEED_PY_SERVER: &str = "http://python_proc.process.yral-ml-feed-server.internal:50059";

#[cfg(feature = "local-bin")]
pub const OFF_CHAIN_AGENT: &str = "http://localhost:50051";

#[cfg(not(feature = "local-bin"))]
pub const OFF_CHAIN_AGENT: &str = "https://offchain.yral.com:443";

pub const CLOUDFLARE_ML_FEED_CACHE_WORKER_URL: &str =
    "https://yral-ml-feed-cache.go-bazzinga.workers.dev";
