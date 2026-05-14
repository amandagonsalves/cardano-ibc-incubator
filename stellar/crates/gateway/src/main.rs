use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use soroban_client::{
    account::AccountBehavior,
    keypair::{Keypair, KeypairBehavior},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    api::account::{account, balance, get_account},
    state::AppState,
};

mod api;
mod state;

async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let keypair =
        Keypair::from_secret(&state.signing_key).expect("could not get keypair from secret key");

    let public_key = keypair.public_key().to_string();

    let account = get_account(&state, &public_key).await;

    (
        StatusCode::OK,
        format!(
            "Stellar Gateway is up and the signer {} is ready.",
            account.account_id()
        ),
    )
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        .route("/health", get(health))
        .route("/account/{address}", get(account))
        .route("/balance/{address}", get(balance))
        .with_state(Arc::new(AppState::new()));

    let port = std::env::var("STELLAR_GATEWAY_PORT").expect("STELLAR_GATEWAY_PORT must be set");

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();

    tracing::debug!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
