// Copyright (c) Mysten Labs, Inc.
// Copyright (c) The Social Proof Foundation, LLC.
// SPDX-License-Identifier: Apache-2.0

use axum::{
    routing::{get, post},
    Router,
};
use clap::{Parser, ValueEnum};
use fastcrypto_zkp::bn254::zk_login::{fetch_jwks, OIDCProvider};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower_http::cors::{CorsLayer, AllowOrigin};
use tracing::{info, warn};
use zklogin_verifier::{verify, AppState};

#[derive(Parser, Debug)]
#[command(name = "zklogin-verifier")]
#[command(about = "A zkLogin verifier for MySocial blockchain")]
#[command(version)]
struct Cli {
    /// Network to connect to
    #[arg(short, long, value_enum, default_value_t = Network::Testnet)]
    network: Network,

    /// Port to bind the server to
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// JWK refresh interval in seconds
    #[arg(long, default_value_t = 3600)]
    jwk_refresh_interval: u64,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Network {
    /// MainNet - Production network
    Mainnet,
    /// TestNet - Test network
    Testnet,
    /// DevNet - Development network
    Devnet,
    /// LocalNet - Local development network
    Localnet,
}

impl Network {
    fn get_config(&self) -> NetworkConfig {
        match self {
            Network::Mainnet => NetworkConfig {
                name: "mainnet".to_string(),
                rpc_url: "https://mainnet.mysocial.network/rpc".to_string(),
                faucet_url: None,
                jwk_providers: vec![
                    OIDCProvider::Google,
                    OIDCProvider::Facebook,
                    OIDCProvider::Apple,
                    OIDCProvider::Slack,
                    OIDCProvider::Twitch,
                    OIDCProvider::Kakao,
                ],
            },
            Network::Testnet => NetworkConfig {
                name: "testnet".to_string(),
                rpc_url: "https://testnet.mysocial.network/rpc".to_string(),
                faucet_url: Some("https://faucet.mysocial.network".to_string()),
                jwk_providers: vec![
                    OIDCProvider::Google,
                    OIDCProvider::Facebook,
                    OIDCProvider::Apple,
                    OIDCProvider::Slack,
                    OIDCProvider::Twitch,
                    OIDCProvider::Kakao,
                ],
            },
            Network::Devnet => NetworkConfig {
                name: "devnet".to_string(),
                rpc_url: "https://devnet.mysocial.network/rpc".to_string(),
                faucet_url: Some("https://devnet.mysocial.network/faucet".to_string()),
                jwk_providers: vec![
                    OIDCProvider::Google,
                    OIDCProvider::Facebook,
                ],
            },
            Network::Localnet => NetworkConfig {
                name: "localnet".to_string(),
                rpc_url: "http://localhost:9000/rpc".to_string(),
                faucet_url: Some("http://localhost:9123/gas".to_string()),
                jwk_providers: vec![
                    OIDCProvider::Google, // For local testing
                ],
            },
        }
    }
}

#[derive(Debug, Clone)]
struct NetworkConfig {
    name: String,
    rpc_url: String,
    faucet_url: Option<String>,
    jwk_providers: Vec<OIDCProvider>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct SerializableNetworkConfig {
    name: String,
    rpc_url: String,
    faucet_url: Option<String>,
    jwk_providers: Vec<String>,
}

impl From<&NetworkConfig> for SerializableNetworkConfig {
    fn from(config: &NetworkConfig) -> Self {
        Self {
            name: config.name.clone(),
            rpc_url: config.rpc_url.clone(),
            faucet_url: config.faucet_url.clone(),
            jwk_providers: config.jwk_providers.iter().map(|p| format!("{:?}", p)).collect(),
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    
    tracing_subscriber::FmtSubscriber::builder()
        .try_init()
        .expect("setting default subscriber failed");

    let network_config = cli.network.get_config();
    
    info!("🚀 Starting zkLogin Verifier");
    info!("📡 Network: {} ({})", network_config.name, network_config.rpc_url);
    info!("🔐 OAuth Providers: {:?}", network_config.jwk_providers);
    if let Some(faucet) = &network_config.faucet_url {
        info!("💰 Faucet: {}", faucet);
    }
    info!("🌐 Server will listen on port: {}", cli.port);

    let state = Arc::new(AppState {
        jwks: Default::default(),
    });

    let state_clone = state.clone();
    let network_config_clone = network_config.clone();
    let jwk_refresh_interval = cli.jwk_refresh_interval;

    tokio::task::spawn(async move {
        info!("🔄 Starting JWK updater task (refresh every {}s)", jwk_refresh_interval);
        loop {
            let client = reqwest::Client::new();
            for provider in &network_config_clone.jwk_providers {
                match fetch_jwks(provider, &client).await {
                    Err(e) => {
                        warn!("❌ Error when fetching JWK with provider {:?}: {:?}", provider, e);
                        tokio::time::sleep(Duration::from_secs(30)).await;
                    }
                    Ok(keys) => {
                        for (jwk_id, jwk) in keys {
                            let mut oauth_provider_jwk = state_clone.jwks.write();
                            if oauth_provider_jwk.contains_key(&jwk_id) {
                                continue;
                            }
                            info!("✅ {:?} JWK updated: {:?}", &jwk_id, jwk);
                            // todo(joyqvq): prune old jwks.
                            oauth_provider_jwk.insert(jwk_id, jwk.clone());
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(jwk_refresh_interval)).await;
        }
    });

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<axum::http::HeaderValue>().unwrap())
        .allow_methods([axum::http::Method::POST, axum::http::Method::OPTIONS])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/", get(ping))
        .route("/verify", post(verify))
        .route("/health", get(health))
        .route("/config", get(move || async move { 
            let serializable_config = SerializableNetworkConfig::from(&network_config);
            serde_json::to_string_pretty(&serializable_config).unwrap() 
        }))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    info!("🎯 Listening on {}", addr);
    info!("📋 Available endpoints:");
    info!("   GET  /           - Ping endpoint");
    info!("   POST /verify     - Verify zkLogin signature");
    info!("   GET  /health     - Health check");
    info!("   GET  /config     - Show current configuration");
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn ping() -> &'static str {
    "Pong! zkLogin Verifier is running 🚀"
}

async fn health() -> &'static str {
    "OK"
}
