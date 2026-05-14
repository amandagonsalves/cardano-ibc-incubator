use std::sync::Arc;

use soroban_client::{Options, Server};

pub struct AppState {
    pub rpc_server: Arc<Server>,
    pub signing_key: Arc<String>,
}

impl AppState {
    pub fn new() -> Self {
        let server_url = std::env::var("STELLAR_RPC_URL").expect("STELLAR_RPC_URL must be set");
        let server = Server::new(&server_url, Options::default()).expect("Cannot create server");
        let signing_key =
            std::env::var("STELLAR_SIGNING_KEY").expect("STELLAR_SIGNING_KEY must be set");

        Self {
            rpc_server: Arc::new(server),
            signing_key: Arc::new(signing_key),
        }
    }
}
