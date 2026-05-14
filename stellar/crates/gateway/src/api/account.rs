use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use soroban_client::{
    account::{Account, AccountBehavior},
    address::{Address, AddressTrait},
    contract::{self, ContractBehavior},
    network::{NetworkPassphrase, Networks},
    transaction::TransactionBuilder,
    transaction_builder::{TransactionBuilderBehavior, TIMEOUT_INFINITE},
    xdr::{int128_helpers::i128_from_pieces, Int128Parts, ScVal},
};

use crate::state::AppState;

#[derive(Serialize)]
struct AccountResponse {
    account_id: String,
    sequence_number: String,
}

#[derive(Serialize)]
struct BalanceResponse {
    balance_stroops: i128,
    balance_xlm: f64,
}

impl BalanceResponse {
    fn new(balance_stroops: i128) -> Self {
        Self {
            balance_stroops,
            balance_xlm: balance_stroops as f64 / 10_000_000.0,
        }
    }
}

pub async fn get_account(state: &AppState, address: &str) -> Account {
    let account_data = state.rpc_server.request_airdrop(address).await.unwrap();

    Account::new(address, &account_data.sequence_number()).expect("cannot get account data")
}

pub async fn account(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    let account = get_account(&state, &address).await;

    tracing::debug!("account {} sequence {}", address, account.sequence_number());

    (
        StatusCode::OK,
        Json(AccountResponse {
            account_id: account.account_id(),
            sequence_number: account.sequence_number(),
        }),
    )
}

pub async fn balance(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    let mut account = get_account(&state, &address).await;

    let native_address = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
    let contract = contract::Contracts::new(native_address).unwrap();

    let account_address = Address::new(&address).unwrap();
    let args = vec![account_address.to_sc_val().unwrap()];

    let tx = TransactionBuilder::new(&mut account, Networks::testnet(), None)
        .fee(1000000_u32)
        .add_operation(contract.call("balance", Some(args)))
        .set_timeout(TIMEOUT_INFINITE)
        .expect("could not set timeout")
        .build();

    let balance_stroops = match state.rpc_server.simulate_transaction(&tx, None).await {
        Ok(sim_result) => {
            tracing::debug!("latest ledger {}", sim_result.latest_ledger);
            
            if let Some((ScVal::I128(Int128Parts { hi, lo }), _)) = sim_result.to_result() {
                let stroops = i128_from_pieces(hi, lo);

                tracing::debug!(
                    "balance {} stroops ({} XLM)",
                    stroops,
                    stroops as f64 / 10_000_000.0
                );
                stroops
            } else {
                0
            }
        }
        Err(error) => {
            tracing::warn!("failed to get balance: {:?}", error);
            0
        }
    };

    (StatusCode::OK, Json(BalanceResponse::new(balance_stroops)))
}
