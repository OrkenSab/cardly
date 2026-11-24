// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html

use std::env;
use std::net::SocketAddr;
use std::path::Path;
use std::process;

use cardly_server::build_router;
use cardly_server::config::load_dotenv;
use cardly_server::config::ServerConfig;
use cardly_server::AppState;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    load_dotenv(Path::new(".env"));
    load_dotenv(Path::new("../.env"));

    if env::args().nth(1).as_deref() == Some("--healthcheck") {
        run_healthcheck();
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cardly_server=info,anki=info,tower_http=info".into()),
        )
        .init();

    let config = ServerConfig::from_env()?;
    let addr = SocketAddr::new(config.host, config.port);
    let user_count = config.users.len();
    let state = AppState::new(config);
    let rpc_count = state.rpc.len();

    if state.config.demo_mode {
        let demo_user = state
            .config
            .demo_user
            .as_deref()
            .unwrap_or(cardly_server::demo::DEFAULT_DEMO_USER);
        info!(user = %demo_user, "Demo mode active; resetting and seeding sample decks on startup...");
        if let Err(err) = cardly_server::demo::reset_demo_collection(&state).await {
            tracing::warn!(error = ?err, "Failed to reset demo collection at startup");
        }
        let interval = state
            .config
            .demo_reset_interval_secs
            .unwrap_or(cardly_server::demo::DEFAULT_RESET_INTERVAL_SECS);
        cardly_server::demo::start_demo_reset_task(state.clone(), interval);
    }

    let app = build_router(state);
    let listener = TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;

    info!(
        %bound,
        users = user_count,
        rpc_methods = rpc_count,
        "Cardly server listening"
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

fn run_healthcheck() -> ! {
    let port = env::var("CARDLY_PORT")
        .or_else(|_| env::var("SYNC_PORT"))
        .unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{port}");
    match std::net::TcpStream::connect(&addr) {
        Ok(_) => process::exit(0),
        Err(_) => process::exit(1),
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("Shutting down Cardly server...");
}
