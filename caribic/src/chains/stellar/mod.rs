use std::path::{Path, PathBuf};

use async_trait::async_trait;
use dirs::home_dir;

use crate::chains::{
    check_host_port_health, check_port_health, ChainAdapter, ChainFlags, ChainHealthStatus,
    ChainNetwork, ChainStartRequest,
};
use crate::logger::verbose;

mod config;
mod hermes;
mod lifecycle;

pub struct StellarChainAdapter;

pub static STELLAR_CHAIN_ADAPTER: StellarChainAdapter = StellarChainAdapter;

const STELLAR_NETWORKS: [ChainNetwork; 1] = [ChainNetwork {
    name: "testnet",
    description: "Stellar public testnet (soroban-testnet.stellar.org) — gateway runs locally",
    managed_by_caribic: false,
}];

#[async_trait]
impl ChainAdapter for StellarChainAdapter {
    fn id(&self) -> &'static str {
        config::DISPLAY_NAME
    }

    fn display_name(&self) -> &'static str {
        config::DISPLAY_NAME
    }

    fn default_network(&self) -> &'static str {
        "testnet"
    }

    fn supported_networks(&self) -> &'static [ChainNetwork] {
        &STELLAR_NETWORKS
    }

    async fn start(
        &self,
        project_root: &Path,
        request: &ChainStartRequest<'_>,
    ) -> Result<(), String> {
        self.validate_flags(request.network, request.flags)?;

        let stellar_dir = workspace_dir(project_root);

        lifecycle::ensure_stellar_ibc_available(project_root)
            .map_err(|e| format!("Failed to initialise stellar-ibc submodule: {}", e))?;

        lifecycle::start_gateway(
            project_root,
            config::TESTNET_GRPC_PORT,
            config::TESTNET_HTTP_PORT,
            config::TESTNET_RPC_URL,
            config::TESTNET_NETWORK_PASSPHRASE,
        )
        .await
        .map_err(|e| format!("Failed to start Stellar gateway: {}", e))?;

        if let Err(e) = hermes::sync_testnet_chain_with_hermes(project_root, &stellar_dir) {
            verbose(&format!("Stellar Hermes sync skipped: {}", e));
        }

        println!(
            "Stellar gateway running on :{grpc} (HTTP :{http}). Run `caribic chain health --chain stellar` to verify.",
            grpc = config::TESTNET_GRPC_PORT,
            http = config::TESTNET_HTTP_PORT,
        );

        Ok(())
    }

    fn stop(
        &self,
        _project_root: &Path,
        _network: &str,
        _flags: &ChainFlags,
    ) -> Result<(), String> {
        lifecycle::stop_gateway()
    }

    fn health(
        &self,
        _project_root: &Path,
        _network: &str,
        flags: &ChainFlags,
    ) -> Result<Vec<ChainHealthStatus>, String> {
        self.validate_flags("testnet", flags)?;

        let grpc_status = check_port_health(
            "stellar-gateway",
            config::TESTNET_GRPC_PORT,
            "Stellar gateway gRPC",
        );
        let http_status = check_host_port_health(
            "stellar-gateway-http",
            "127.0.0.1",
            config::TESTNET_HTTP_PORT,
            "Stellar gateway HTTP",
        );
        let rpc_status = check_host_port_health(
            "stellar-rpc",
            "soroban-testnet.stellar.org",
            443,
            "Stellar Soroban RPC",
        );

        Ok(vec![
            ChainHealthStatus {
                id: "stellar-gateway",
                label: "Stellar gateway",
                healthy: grpc_status.healthy && http_status.healthy,
                status: format!(
                    "gRPC ({}): {}; HTTP ({}): {}",
                    config::TESTNET_GRPC_PORT,
                    if grpc_status.healthy {
                        "reachable"
                    } else {
                        "not reachable"
                    },
                    config::TESTNET_HTTP_PORT,
                    if http_status.healthy {
                        "reachable"
                    } else {
                        "not reachable"
                    },
                ),
            },
            ChainHealthStatus {
                id: "stellar-rpc",
                label: "Stellar Soroban RPC",
                healthy: rpc_status.healthy,
                status: format!(
                    "soroban-testnet.stellar.org:443 {}",
                    if rpc_status.healthy {
                        "reachable"
                    } else {
                        "not reachable"
                    },
                ),
            },
        ])
    }
}

/// Returns the local runtime workspace used by the Stellar adapter.
pub fn workspace_dir(project_root: &Path) -> PathBuf {
    if let Some(home) = home_dir() {
        return home
            .join(".caribic")
            .join("stellar")
            .join("workspace")
            .join("stellar");
    }

    project_root
        .join(".caribic")
        .join("stellar")
        .join("workspace")
        .join("stellar")
}
