use std::fs;
use std::path::Path;

use super::config;
use crate::chains::hermes_support::{self, HermesStellarChainProfile};
use crate::process::hermes::HermesCli;

pub(super) fn sync_testnet_chain_with_hermes(
    project_root: &Path,
    stellar_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if hermes_support::hermes_config_path().is_none() {
        return Ok(());
    }

    hermes_support::ensure_hermes_docker_wrapper()?;

    ensure_testnet_chain_in_hermes_config()?;
    ensure_testnet_key_in_hermes_keyring(project_root, stellar_dir)?;
    Ok(())
}

fn ensure_testnet_chain_in_hermes_config() -> Result<(), Box<dyn std::error::Error>> {
    let profile = HermesStellarChainProfile {
        id: config::TESTNET_CHAIN_ID.to_string(),
        rpc_addr: config::TESTNET_RPC_URL.to_string(),
        grpc_addr: format!("http://127.0.0.1:{}", config::TESTNET_GRPC_PORT),
        key_name: config::TESTNET_KEY_NAME.to_string(),
        max_block_time: "6s".to_string(),
        clock_drift: "5s".to_string(),
    };
    hermes_support::ensure_stellar_chain_in_hermes_config(&profile)
}

fn ensure_testnet_key_in_hermes_keyring(
    project_root: &Path,
    stellar_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let hermes_bin = resolve_hermes_binary(project_root, stellar_dir)?;
    let mnemonic = resolve_mnemonic(project_root)?;
    let temp_file = hermes_support::write_temp_mnemonic_file("stellar-testnet-relayer", mnemonic)?;
    let temp_arg = temp_file.to_string_lossy().to_string();

    let result = HermesCli::new(&hermes_bin).output(
        None,
        &[
            "keys",
            "add",
            "--chain",
            config::TESTNET_CHAIN_ID,
            "--mnemonic-file",
            temp_arg.as_str(),
            "--key-name",
            config::TESTNET_KEY_NAME,
            "--overwrite",
        ],
    );

    let _ = fs::remove_file(&temp_file);

    match result? {
        out if out.status.success() => Ok(()),
        out => Err(format!(
            "hermes keys add failed for chain '{}': {}",
            config::TESTNET_CHAIN_ID,
            String::from_utf8_lossy(&out.stderr)
        )
        .into()),
    }
}

fn resolve_mnemonic(project_root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(secret) = std::env::var("STELLAR_SECRET_KEY") {
        let trimmed = secret.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    let caribic_dir = project_root.join("caribic");
    let mnemonic_path = caribic_dir.join(config::TESTNET_MNEMONIC_FILE);

    if !mnemonic_path.exists() {
        return Err(format!(
            "No Stellar mnemonic available. Set STELLAR_SECRET_KEY env var or create {} with a 24-word BIP-39 mnemonic.",
            mnemonic_path.display()
        )
        .into());
    }

    let content = fs::read_to_string(&mnemonic_path)
        .map_err(|e| format!("Failed to read Stellar mnemonic file: {}", e))?;

    content
        .lines()
        .find(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .ok_or_else(|| {
            format!(
                "Stellar mnemonic file at {} contains only comments. Set STELLAR_SECRET_KEY env var or add a 24-word BIP-39 mnemonic.",
                mnemonic_path.display()
            )
            .into()
        })
}

fn resolve_hermes_binary(
    project_root: &Path,
    stellar_dir: &Path,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    if let Some(local) = hermes_support::resolve_local_hermes_binary(project_root, stellar_dir) {
        return Ok(local);
    }
    hermes_support::ensure_hermes_docker_wrapper()
}
