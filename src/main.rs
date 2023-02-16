use axum::Extension;
use axum::Router;
use orderbook_api_rs::actor;
use orderbook_api_rs::database;
use orderbook_api_rs::endpoints;
use orderbook_api_rs::AppContext;
use orderbook_api_rs::Config;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let config = Config::parse()?;

    let db = database::connect(&config).await?;
    database::run_migrations(&db).await?;

    let (client, actor) = actor::build(db.clone(), "vibranium", 8);

    let app_state = AppContext {
        db,
        config: Arc::new(config),
        actor_client: client,
    };

    let app = Router::new()
        .nest("/", endpoints::routes())
        .layer(Extension(app_state))
        .layer(TraceLayer::new_for_http());

    let address = SocketAddr::from(([0, 0, 0, 0], 3000));
    let http_server = axum::Server::bind(&address)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal());

    tracing::info!("Server running, listening on {}", address);
    tokio::select! {
        _ = actor.run() => {},
        _ = http_server => {},
    }

    Ok(())
}

async fn shutdown_signal() {
    use std::io;
    use tokio::signal::unix::SignalKind;

    async fn terminate() -> io::Result<()> {
        tokio::signal::unix::signal(SignalKind::terminate())?
            .recv()
            .await;
        Ok(())
    }

    tokio::select! {
        _ = terminate() => {},
        _ = tokio::signal::ctrl_c() => {},
    }
    tracing::debug!("signal received, starting graceful shutdown")
}
