use futures::{SinkExt, StreamExt};
use warp::ws::{Message, WebSocket, Ws};
use warp::Filter;

use dashmap::DashMap;
pub use portal_lib::*;
use std::sync::{Arc, OnceLock};

use tokio::net::TcpListener;

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::stream::{SplitSink, SplitStream};

mod connected_clients;
mod client_manager;

use self::connected_clients::*;
mod active_stream;
use self::active_stream::*;

mod auth;
pub use self::auth::client_auth;

// pub use self::auth_db::AuthDbService;

mod control_server;
mod control_server_2;
mod remote;

mod config;
pub use self::config::Config;
mod network;

mod observability;

mod cli;
use clap::Parser;
use cli::Cli;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry;

use tracing::{error, info, Instrument};

static CLI: OnceLock<Cli> = OnceLock::new();
static CONNECTIONS: OnceLock<Connections> = OnceLock::new();
static ACTIVE_STREAMS: OnceLock<ActiveStreams> = OnceLock::new();
static CONFIG: OnceLock<Config> = OnceLock::new();
static AUTH_DB_SERVICE: OnceLock<crate::auth::NoAuth> = OnceLock::new();

pub fn get_cli() -> &'static Cli {
    CLI.get_or_init(Cli::parse)
}

pub fn get_connections() -> &'static Connections {
    CONNECTIONS.get_or_init(Connections::new)
}

pub fn get_active_streams() -> &'static ActiveStreams {
    ACTIVE_STREAMS.get_or_init(|| Arc::new(DashMap::new()))
}

pub fn get_config() -> &'static Config {
    CONFIG.get_or_init(|| match get_cli().config {
        Some(ref config_path) => Config::load_from_file(config_path.to_str().unwrap()).unwrap(),
        None => Config::load_from_env(),
    })
}

pub fn get_auth_db_service() -> &'static crate::auth::NoAuth {
    AUTH_DB_SERVICE.get_or_init(|| crate::auth::NoAuth)
}

#[tokio::main]
async fn main() {
    // if let Some(config_path) = &CLI.config {
    //     println!("Value for config: {}", config_path.display());
    // };

    // setup observability
    let subscriber = registry::Registry::default()
        .with(LevelFilter::DEBUG)
        .with(tracing_subscriber::fmt::Layer::default());
    tracing::subscriber::set_global_default(subscriber).expect("setting global default failed");

    info!("starting server!");

    let config = get_config();
    #[cfg(feature = "tcp_tunnel")]
    {
        control_server_2::spawn(([0, 0, 0, 0, 0, 0, 0, 0], config.control_port)).await;
    }

    #[cfg(feature = "ws_tunnel")]
    {
        control_server::spawn(([0, 0, 0, 0, 0, 0, 0, 0], config.control_port));
    }

    info!(
        "started portal control server on [::]:{}",
        config.control_port
    );

    network::spawn(([0, 0, 0, 0, 0, 0, 0, 0], config.internal_network_port));
    info!(
        "start network service on [::]:{}",
        config.internal_network_port
    );

    let listen_addr = format!("[::]:{}", config.remote_port);
    info!("listening on: {}", &listen_addr);
    info!("portal server with hostname: {}", config.portal_host);

    // create our accept any server
    let listener = TcpListener::bind(listen_addr)
        .await
        .expect("failed to bind");

    loop {
        let socket = match listener.accept().await {
            Ok((socket, _)) => socket,
            Err(e) => {
                error!("failed to accept socket: {:?}", e);
                continue;
            }
        };

        info!("accepted connection from: {}", socket.peer_addr().unwrap());

        tokio::spawn(
            async move {
                remote::accept_connection(socket).await;
            }
            .instrument(observability::remote_trace("remote_connect")),
        );
    }
}
