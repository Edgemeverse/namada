use color_eyre::eyre::Result;
use namada::types::address::wnam;
use namada::types::ethereum_events::testing::DAI_ERC20_ETH_ADDRESS_CHECKSUMMED;
use namada_apps::config::ethereum_bridge;

use super::setup::set_ethereum_bridge_mode;
use crate::e2e::helpers::get_actor_rpc;
use crate::e2e::setup;
use crate::e2e::setup::constants::{
    wasm_abs_path, ALBERT, BERTHA, NAM, TX_WRITE_STORAGE_KEY_WASM,
};
use crate::e2e::setup::{Bin, Who};
use crate::{run, run_as};

const ETH_BRIDGE_ADDRESS: &str = "atest1v9hx7w36g42ysgzzwf5kgem9ypqkgerjv4ehxgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpq8f99ew";

/// # Examples
///
/// ```
/// let storage_key = storage_key("queue");
/// assert_eq!(storage_key, "#atest1v9hx7w36g42ysgzzwf5kgem9ypqkgerjv4ehxgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpq8f99ew/queue");
/// ```
fn storage_key(path: &str) -> String {
    format!("#{ETH_BRIDGE_ADDRESS}/{}", path)
}

#[test]
#[ignore]
// this test is outdated, so it is ignored
fn everything() {
    const LEDGER_STARTUP_TIMEOUT_SECONDS: u64 = 30;
    const CLIENT_COMMAND_TIMEOUT_SECONDS: u64 = 30;
    const SOLE_VALIDATOR: Who = Who::Validator(0);

    let test = setup::single_node_net().unwrap();

    let mut anoman_ledger = run_as!(
        test,
        SOLE_VALIDATOR,
        Bin::Node,
        &["ledger"],
        Some(LEDGER_STARTUP_TIMEOUT_SECONDS)
    )
    .unwrap();
    anoman_ledger
        .exp_string("Anoma ledger node started")
        .unwrap();
    anoman_ledger.exp_string("Tendermint node started").unwrap();
    anoman_ledger.exp_string("Committed block hash").unwrap();
    let _bg_ledger = anoman_ledger.background();

    let tx_data_path = test.test_dir.path().join("queue_storage_key.txt");
    std::fs::write(&tx_data_path, &storage_key("queue")[..]).unwrap();

    let tx_code_path = wasm_abs_path(TX_WRITE_STORAGE_KEY_WASM);

    let tx_data_path = tx_data_path.to_string_lossy().to_string();
    let tx_code_path = tx_code_path.to_string_lossy().to_string();
    let ledger_addr = get_actor_rpc(&test, &SOLE_VALIDATOR);
    let tx_args = vec![
        "tx",
        "--signer",
        ALBERT,
        "--code-path",
        &tx_code_path,
        "--data-path",
        &tx_data_path,
        "--ledger-address",
        &ledger_addr,
    ];

    for &dry_run in &[true, false] {
        let tx_args = if dry_run {
            vec![tx_args.clone(), vec!["--dry-run"]].concat()
        } else {
            tx_args.clone()
        };
        let mut anomac_tx = run!(
            test,
            Bin::Client,
            tx_args,
            Some(CLIENT_COMMAND_TIMEOUT_SECONDS)
        )
        .unwrap();

        if !dry_run {
            anomac_tx.exp_string("Transaction accepted").unwrap();
            anomac_tx.exp_string("Transaction applied").unwrap();
        }
        // TODO: we should check here explicitly with the ledger via a
        //  Tendermint RPC call that the path `value/#EthBridge/queue`
        //  is unchanged rather than relying solely  on looking at anomac
        //  stdout.
        anomac_tx.exp_string("Transaction is invalid").unwrap();
        anomac_tx
            .exp_string(&format!("Rejected: {}", ETH_BRIDGE_ADDRESS))
            .unwrap();
        anomac_tx.assert_success();
    }
}

/// Tests that we can start the ledger with an endpoint for submitting Ethereum
/// events. This mode can be used in further end-to-end tests.
#[test]
fn run_ledger_with_ethereum_events_endpoint() -> Result<()> {
    let test = setup::single_node_net()?;

    set_ethereum_bridge_mode(
        &test,
        &test.net.chain_id,
        &Who::Validator(0),
        ethereum_bridge::ledger::Mode::EventsEndpoint,
    );

    // Start the ledger as a validator
    let mut ledger =
        run_as!(test, Who::Validator(0), Bin::Node, vec!["ledger"], Some(40))?;
    ledger.exp_string(
        "Starting to listen for Borsh-serialized Ethereum events",
    )?;
    ledger.exp_string("Anoma ledger node started")?;

    ledger.send_control('c')?;
    ledger.exp_string(
        "Stopping listening for Borsh-serialized Ethereum events",
    )?;

    Ok(())
}

#[test]
fn test_add_to_bridge_pool() {
    const LEDGER_STARTUP_TIMEOUT_SECONDS: u64 = 40;
    const CLIENT_COMMAND_TIMEOUT_SECONDS: u64 = 60;
    const SOLE_VALIDATOR: Who = Who::Validator(0);
    let wnam_address = wnam().to_canonical();
    let test = setup::single_node_net().unwrap();
    set_ethereum_bridge_mode(
        &test,
        &test.net.chain_id,
        &Who::Validator(0),
        ethereum_bridge::ledger::Mode::EventsEndpoint,
    );

    let mut anoman_ledger = run_as!(
        test,
        SOLE_VALIDATOR,
        Bin::Node,
        &["ledger"],
        Some(LEDGER_STARTUP_TIMEOUT_SECONDS)
    )
    .unwrap();
    anoman_ledger
        .exp_string("Anoma ledger node started")
        .unwrap();
    anoman_ledger.exp_string("Tendermint node started").unwrap();
    anoman_ledger.exp_string("Committed block hash").unwrap();
    let _bg_ledger = anoman_ledger.background();

    let ledger_addr = get_actor_rpc(&test, &SOLE_VALIDATOR);
    let tx_args = vec![
        "add-erc20-transfer",
        "--address",
        BERTHA,
        "--signer",
        BERTHA,
        "--amount",
        "100",
        "--erc20",
        &wnam_address,
        "--ethereum-address",
        DAI_ERC20_ETH_ADDRESS_CHECKSUMMED,
        "--fee-amount",
        "10",
        "--fee-payer",
        BERTHA,
        "--gas-amount",
        "0",
        "--gas-limit",
        "0",
        "--gas-token",
        NAM,
        "--ledger-address",
        &ledger_addr,
    ];

    let mut anomac_tx = run!(
        test,
        Bin::Client,
        tx_args,
        Some(CLIENT_COMMAND_TIMEOUT_SECONDS)
    )
    .unwrap();
    anomac_tx.exp_string("Transaction applied").unwrap();
}
