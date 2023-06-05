
use ethers::signers::coins_bip39::English;
use ethers::signers::{LocalWallet, MnemonicBuilder, Signer};
use ethers::types::{Address, H160};

use rstest::*;
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::iter::FromIterator;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use tap_core::{
    adapters::{
        collateral_adapter_mock::CollateralAdapterMock,
        rav_storage_adapter_mock::RAVStorageAdapterMock,
        receipt_checks_adapter_mock::ReceiptChecksAdapterMock,
        receipt_storage_adapter_mock::ReceiptStorageAdapterMock,
    },
    tap_receipt::ReceiptCheck,
};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

pub mod server;

#[fixture]
fn keys() -> (LocalWallet, Address) {
    let wallet: LocalWallet = MnemonicBuilder::<English>::default()
     .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
     .build()
     .unwrap();
    let address = wallet.address();
    (wallet, address)
}

#[fixture]
fn allocation_ids() -> Vec<Address> {
    vec![
        Address::from_str("0xabababababababababababababababababababab").unwrap(),
        Address::from_str("0xdeaddeaddeaddeaddeaddeaddeaddeaddeaddead").unwrap(),
        Address::from_str("0xbeefbeefbeefbeefbeefbeefbeefbeefbeefbeef").unwrap(),
        Address::from_str("0x1234567890abcdef1234567890abcdef12345678").unwrap(),
    ]
}

#[fixture]
fn http_request_size_limit() -> u32 {
    100 * 1024
}

#[fixture]
fn http_response_size_limit() -> u32 {
    100 * 1024
}

#[fixture]
fn http_max_concurrent_connections() -> u32 {
    1
}

#[fixture]
fn http_port() -> u16 {
    8080
}

#[fixture]
fn collateral_adapter() -> CollateralAdapterMock {
    CollateralAdapterMock::new(Arc::new(RwLock::new(HashMap::new())))
}

#[fixture]
fn receipt_storage_adapter() -> ReceiptStorageAdapterMock {
    ReceiptStorageAdapterMock::new(Arc::new(RwLock::new(HashMap::new())))
}

#[fixture]
fn num_queries() -> usize {
    100
}

#[fixture]
fn query_price() -> Vec<u128> {
    let seed: Vec<u8> = (0..32u8).collect(); // A seed of your choice
    let mut rng: StdRng = SeedableRng::from_seed(seed.try_into().unwrap());
    let mut v = Vec::new();

    for _ in 0..num_queries() {
        v.push(rng.gen::<u128>() % 100);
        // v.push(100u128);
    }
    v
}

#[fixture]
fn receipt_checks_adapter() -> ReceiptChecksAdapterMock {
    // Setup receipt storage
    let receipt_storage = Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));

    let query_prices = query_price();
    // Setup query appraisals
    let query_appraisals = (0..num_queries() as u64)
        .zip(query_prices)
        .into_iter()
        .map(|(id, price)| (id, price))
        .collect::<HashMap<_, _>>();

    let query_appraisals_storage = Arc::new(RwLock::new(query_appraisals));

    // Setup receipt checks adapter
    let allocation_ids: Arc<RwLock<HashSet<H160>>> =
        Arc::new(RwLock::new(HashSet::from_iter(allocation_ids())));
    let gateway_ids: Arc<RwLock<HashSet<H160>>> = Arc::new(RwLock::new(HashSet::from([keys().1])));
    ReceiptChecksAdapterMock::new(
        receipt_storage.clone(),
        query_appraisals_storage,
        allocation_ids,
        gateway_ids,
    )
}

#[fixture]
fn rav_storage_adapter() -> RAVStorageAdapterMock {
    RAVStorageAdapterMock::new(Arc::new(RwLock::new(HashMap::new())))
}

#[fixture]
fn required_checks() -> Vec<ReceiptCheck> {
    vec![
        ReceiptCheck::CheckAllocationId,
        ReceiptCheck::CheckSignature,
        ReceiptCheck::CheckTimestamp,
        ReceiptCheck::CheckUnique,
        ReceiptCheck::CheckValue,
        ReceiptCheck::CheckAndReserveCollateral,
    ]
}

#[fixture]
fn initial_checks() -> Vec<ReceiptCheck> {
    vec![
        ReceiptCheck::CheckAllocationId,
        ReceiptCheck::CheckSignature,
        ReceiptCheck::CheckTimestamp,
        ReceiptCheck::CheckUnique,
        ReceiptCheck::CheckValue,
        ReceiptCheck::CheckAndReserveCollateral,
    ]
}

#[tokio::test]
async fn test_manager_server() -> Result<(), Box<dyn std::error::Error>> {
    use jsonrpsee::core::client::ClientT;
    use jsonrpsee::http_client::HttpClientBuilder;
    use tap_core::eip_712_signed_message::EIP712SignedMessage;
    use tap_core::tap_receipt::Receipt;
    use tap_aggregator::server as agg_server;

    let mut collateral_adapter = collateral_adapter();
    let receipt_checks_adapter = receipt_checks_adapter();
    let receipt_storage_adapter = receipt_storage_adapter();
    let rav_storage_adapter = rav_storage_adapter();
    let initial_checks = initial_checks();
    let required_checks = required_checks();

    let gateway_id = keys().1;
    let value = query_price().into_iter().sum();
    collateral_adapter.increase_collateral(gateway_id, value);
    let threshold = 50;
    let aggregate_server_address = "http://127.0.0.1:8080".to_string();
    let (_server_handle, _) = server::run_server(
        3030,
        collateral_adapter,
        receipt_checks_adapter,
        receipt_storage_adapter,
        rav_storage_adapter,
        initial_checks,
        required_checks,
        threshold,
        aggregate_server_address,
    )
    .await
    .expect("Failed to start server");

    // Start tap_aggregate server
    let (_handle, _local_addr) = agg_server::run_server(
        http_port(),
        keys().0,
        http_request_size_limit(),
        http_response_size_limit(),
        http_max_concurrent_connections(),
    )
    .await
    .expect("Failed to start server");

    // Give the server a moment to start up
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Run receipt client code here
    let server_url = "http://127.0.0.1:3030";
    // Setup client
    let client = HttpClientBuilder::default().build(server_url).unwrap();

    // Create your Receipt here
    let values = query_price();
    let request_id = 0..query_price().len() as u64;
    let mut receipts = Vec::new();

    // Sign receipt
    for value in values {
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids()[0], value).unwrap(), &keys().0)
                .await
                .expect("Failed to sign receipt"),
        );
    }

    let req = receipts
        .iter()
        .zip(request_id.clone())
        .collect::<Vec<_>>();
    
    for (receipt, id) in req {
        let result: Result<(), _> = client.request("request", (id, receipt)).await;
        match result {
            Ok(()) => {}
            Err(e) => println!("Error making request: {:?}", e),
        }
    }

    Ok(())
}
