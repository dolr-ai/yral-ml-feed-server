use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use app_state::AppState;
use axum::{routing::get, Router};
use grpc_services::ml_feed::ml_feed_server::MlFeedServer;
use http::{header::CONTENT_TYPE, StatusCode};
use ml_feed_impl::MLFeedService;
use sentry_tower::{NewSentryLayer, SentryHttpLayer};
use tonic::service::Routes;
use tower::ServiceBuilder;
use tower::{make::Shared, steer::Steer};
use tower_http::cors::CorsLayer;
use tracing::instrument;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

pub mod app_state;
pub mod canister;
pub mod consts;
pub mod error;
pub mod feed;
pub mod grpc_services;
pub mod ml_feed_impl;
pub mod utils;

async fn main_impl() -> Result<()> {
    #[derive(OpenApi)]
    #[openapi(
        tags(
            (name = "ML_FEED", description = "ML Feed API")
        )
    )]
    struct ApiDoc;

    let app_state = Arc::new(AppState::new().await);
    let sentry_tower_layer = ServiceBuilder::new()
        .layer(NewSentryLayer::new_from_top())
        .layer(SentryHttpLayer::with_transaction());

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1/feed", feed::feed_router(app_state.clone()))
        .nest("/api/v2/feed", feed::v2::feed_router_v2(app_state.clone()))
        .split_for_parts();

    let router =
        router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()));

    let mlfeed_service = MLFeedService {
        shared_state: app_state.clone(),
    };
    let mlfeed_server = MlFeedServer::new(mlfeed_service);

    let http = Router::new()
        .route("/healthz", get(health_handler))
        .fallback_service(router)
        .layer(CorsLayer::permissive())
        .layer(sentry_tower_layer)
        .with_state(app_state.clone());

    let grpc_axum = Routes::builder()
        .routes()
        .add_service(mlfeed_server)
        .into_axum_router()
        .layer(NewSentryLayer::new_from_top());

    let http_grpc = Steer::new(
        vec![http, grpc_axum],
        |req: &axum::extract::Request, _svcs: &[_]| {
            let content_type = req.headers().get(CONTENT_TYPE).map(|v| v.as_bytes());
            if content_type == Some(b"application/grpc")
                || content_type == Some(b"application/grpc-web")
                || content_type == Some(b"application/grpc-web+proto")
                || content_type == Some(b"application/grpc-web-text")
                || content_type == Some(b"application/grpc-web-text+proto")
            {
                1
            } else {
                0
            }
        },
    );

    let addr = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], 50051));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    log::info!("listening on {}", addr);

    axum::serve(listener, Shared::new(http_grpc)).await.unwrap();

    Ok(())
}

fn main() {
    let _guard = sentry::init((
        "https://55fb42771c224c7c8ba356ff547744c9@sentry.yral.com/5",
        sentry::ClientOptions {
            release: sentry::release_name!(),
            // debug: true,
            traces_sample_rate: 0.3,
            ..Default::default()
        },
    ));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                format!(
                    "{}=debug,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(sentry_tracing::layer())
        .init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            main_impl().await.unwrap();
        });
}

async fn health_handler() -> (StatusCode, &'static str) {
    log::info!("Health check");
    log::warn!("Health check");
    log::error!("Health check");

    (StatusCode::OK, "OK")
}
