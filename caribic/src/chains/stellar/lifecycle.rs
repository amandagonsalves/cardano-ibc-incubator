use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crate::chains::stellar::config;
use crate::logger::verbose;
use crate::process::system::SystemChecks;

const STARTUP_TIMEOUT_SECS: u64 = 30;

pub(super) fn ensure_stellar_ibc_available(_project_root: &std::path::Path) -> Result<(), String> {
    Ok(())
}

pub(super) async fn start_gateway(
    _project_root: &std::path::Path,
    grpc_port: u16,
    http_port: u16,
    rpc_url: &str,
    network_passphrase: &str,
) -> Result<(), String> {
    if container_is_running() {
        verbose(&format!(
            "Stellar gateway container '{}' already running",
            config::GATEWAY_CONTAINER_NAME
        ));
        return Ok(());
    }

    remove_stopped_container();

    verbose(&format!(
        "Starting Stellar gateway from image {}",
        config::GATEWAY_IMAGE
    ));

    let mut args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        config::GATEWAY_CONTAINER_NAME.to_string(),
        "--restart".to_string(),
        "unless-stopped".to_string(),
        "-p".to_string(),
        format!("{}:50052", grpc_port),
        "-p".to_string(),
        format!("{}:8001", http_port),
        "-e".to_string(),
        format!("STELLAR_RPC_URL={}", rpc_url),
        "-e".to_string(),
        format!("NETWORK_PASSPHRASE={}", network_passphrase),
        "-e".to_string(),
        "STELLAR_GATEWAY_HOST=0.0.0.0".to_string(),
        "-e".to_string(),
        format!("STELLAR_GATEWAY_GRPC_PORT={}", grpc_port),
        "-e".to_string(),
        format!("STELLAR_GATEWAY_HTTP_PORT={}", http_port),
    ];

    for var in &[
        "STELLAR_SIGNING_KEY",
        "IBC_CONTRACT_ID",
        "TRANSFER_CONTRACT_ID",
    ] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                args.push("-e".to_string());
                args.push(format!("{}={}", var, val));
            }
        }
    }

    args.push(config::GATEWAY_IMAGE.to_string());

    let status = Command::new("docker")
        .args(&args)
        .status()
        .map_err(|e| format!("Failed to run 'docker run': {}", e))?;

    if !status.success() {
        return Err(format!(
            "docker run failed for image '{}'. Is Docker running and the image available?",
            config::GATEWAY_IMAGE
        ));
    }

    // Wait for ports to open.
    let deadline = Instant::now() + Duration::from_secs(STARTUP_TIMEOUT_SECS);
    while Instant::now() < deadline {
        if gateway_is_running(grpc_port) {
            verbose(&format!(
                "Stellar gateway container up on gRPC :{} HTTP :{}",
                grpc_port, http_port
            ));
            return Ok(());
        }
        thread::sleep(Duration::from_millis(500));
    }

    Err(format!(
        "Stellar gateway container did not open gRPC port {} within {} seconds.",
        grpc_port, STARTUP_TIMEOUT_SECS
    ))
}

pub(super) fn stop_gateway() -> Result<(), String> {
    if !container_exists() {
        verbose("No Stellar gateway container found — nothing to stop");
        return Ok(());
    }

    verbose(&format!(
        "Stopping Stellar gateway container '{}'",
        config::GATEWAY_CONTAINER_NAME
    ));

    let stop = Command::new("docker")
        .args(["stop", config::GATEWAY_CONTAINER_NAME])
        .status()
        .map_err(|e| format!("Failed to run 'docker stop': {}", e))?;

    if !stop.success() {
        return Err(format!(
            "docker stop failed for container '{}'",
            config::GATEWAY_CONTAINER_NAME
        ));
    }

    // Remove the container so the next `start` can use the same name cleanly.
    let _ = Command::new("docker")
        .args(["rm", config::GATEWAY_CONTAINER_NAME])
        .status();

    verbose("Stellar gateway container stopped and removed");
    Ok(())
}

pub(super) fn gateway_is_running(grpc_port: u16) -> bool {
    SystemChecks::tcp_port_open("localhost", grpc_port)
}

// ── Docker helpers ─────────────────────────────────────────────────────────────

/// Returns true if the container exists in any state (running or stopped).
fn container_exists() -> bool {
    Command::new("docker")
        .args([
            "ps",
            "-aq",
            "--filter",
            &format!("name=^{}$", config::GATEWAY_CONTAINER_NAME),
        ])
        .output()
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false)
}

/// Returns true only when the container is in the running state.
fn container_is_running() -> bool {
    Command::new("docker")
        .args([
            "ps",
            "-q",
            "--filter",
            &format!("name=^{}$", config::GATEWAY_CONTAINER_NAME),
            "--filter",
            "status=running",
        ])
        .output()
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false)
}

/// Removes a stopped container so `docker run --name` can reuse the name.
fn remove_stopped_container() {
    if container_exists() && !container_is_running() {
        verbose(&format!(
            "Removing stopped container '{}'",
            config::GATEWAY_CONTAINER_NAME
        ));
        let _ = Command::new("docker")
            .args(["rm", config::GATEWAY_CONTAINER_NAME])
            .status();
    }
}
