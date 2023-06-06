use ethers::signers::coins_bip39::English;
use ethers::signers::{LocalWallet, MnemonicBuilder, Signer};
use ethers::types::{Address, H160};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
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
fn http_port_indexer_1() -> u16 {
    8080
}

#[fixture]
fn http_port_indexer_2() -> u16 {
    8081
}

#[fixture]
fn http_port_tap_aggregator() -> u16 {
    3030
}


#[fixture]
fn collateral_adapter() -> (CollateralAdapterMock, CollateralAdapterMock) {
    (CollateralAdapterMock::new(Arc::new(RwLock::new(HashMap::new()))),
    CollateralAdapterMock::new(Arc::new(RwLock::new(HashMap::new()))))
}

#[fixture]
fn receipt_storage_adapter() -> (ReceiptStorageAdapterMock, ReceiptStorageAdapterMock) {
    (ReceiptStorageAdapterMock::new(Arc::new(RwLock::new(HashMap::new()))),
    ReceiptStorageAdapterMock::new(Arc::new(RwLock::new(HashMap::new()))))
}

#[fixture]
fn num_queries() -> usize {
    16
}

#[fixture]
fn query_price() -> Vec<u128> {
    let seed: Vec<u8> = (0..32u8).collect(); // A seed of your choice
    let mut rng: StdRng = SeedableRng::from_seed(seed.try_into().unwrap());
    let mut v = Vec::new();

    for _ in 0..num_queries() {
        v.push(rng.gen::<u128>() % 100);
    }
    v
}

#[fixture]
fn receipt_checks_adapter() -> (ReceiptChecksAdapterMock, ReceiptChecksAdapterMock) {
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
    (ReceiptChecksAdapterMock::new(
        receipt_storage.clone(),
        query_appraisals_storage.clone(),
        allocation_ids.clone(),
        gateway_ids.clone(),
    ),
    ReceiptChecksAdapterMock::new(
        receipt_storage,
        query_appraisals_storage,
        allocation_ids,
        gateway_ids,
    ))
}

#[fixture]
fn rav_storage_adapter() -> (RAVStorageAdapterMock, RAVStorageAdapterMock) {
    (RAVStorageAdapterMock::new(Arc::new(RwLock::new(HashMap::new()))),
    RAVStorageAdapterMock::new(Arc::new(RwLock::new(HashMap::new()))))
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

#[fixture]
fn receipt_threshold_1() -> u64 {
    11
}

#[fixture]
fn receipt_threshold_2() -> u64 {
    15
}

#[fixture]
fn num_batches() -> u64 {
    1
}

#[rstest]
#[tokio::test]
async fn test_manager_one_servers(
    collateral_adapter: (CollateralAdapterMock, CollateralAdapterMock),
    receipt_storage_adapter: (ReceiptStorageAdapterMock, ReceiptStorageAdapterMock),
    receipt_checks_adapter: (ReceiptChecksAdapterMock, ReceiptChecksAdapterMock),
    rav_storage_adapter: (RAVStorageAdapterMock, RAVStorageAdapterMock),
    keys: (LocalWallet, Address),
    query_price: Vec<u128>,
    initial_checks: Vec<ReceiptCheck>,
    required_checks: Vec<ReceiptCheck>,
    receipt_threshold_1: u64,
    num_batches: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    use jsonrpsee::core::client::ClientT;
    use jsonrpsee::http_client::HttpClientBuilder;
    use tap_aggregator::server as agg_server;
    use tap_core::eip_712_signed_message::EIP712SignedMessage;
    use tap_core::tap_receipt::Receipt;
  
    let mut collateral_adapter_1 = collateral_adapter.0;
    let receipt_checks_adapter_1 = receipt_checks_adapter.0;
    let receipt_storage_adapter_1 = receipt_storage_adapter.0;
    let rav_storage_adapter_1 = rav_storage_adapter.0;

    let gateway_id = keys.1;
    let value: u128 = query_price.clone().into_iter().sum();
    collateral_adapter_1.increase_collateral(gateway_id, value);
    let threshold_1 = receipt_threshold_1;
    let aggregate_server_address = "http://127.0.0.1:".to_string() + &http_port_tap_aggregator().to_string();
    let server_1_address = "http://127.0.0.1:".to_string() + &http_port_indexer_1().to_string();
    let (_server_handle, _) = server::run_server(
        http_port_indexer_1(),
        collateral_adapter_1,
        receipt_checks_adapter_1,
        receipt_storage_adapter_1,
        rav_storage_adapter_1,
        initial_checks.clone(),
        required_checks.clone(),
        threshold_1,
        aggregate_server_address.clone(),
    )
    .await
    .expect("Failed to start server");

    // Start tap_aggregate server
    let (_handle, _local_addr) = agg_server::run_server(
        http_port_tap_aggregator(),
        keys.clone().0,
        http_request_size_limit(),
        http_response_size_limit(),
        http_max_concurrent_connections(),
    )
    .await
    .expect("Failed to start server");

    // Setup client
    let client_1 = HttpClientBuilder::default().build(server_1_address).unwrap();

    for _ in 0..num_batches {
        // Create your Receipt here
        let values = query_price.clone();
        let request_id = 0..query_price.len() as u64;
        let mut receipts = Vec::new();

        // Sign receipt
        for value in values {
            receipts.push(EIP712SignedMessage::new(Receipt::new(allocation_ids()[0], value).unwrap(), &keys.clone().0)
                    .await
                    .expect("Failed to sign receipt")
        );
        }

        let req = receipts.iter().zip(request_id.clone()).collect::<Vec<_>>();
        
        // let start_time = std::time::Instant::now();
        for (receipt_1, id) in req.clone() {
            let result  = client_1.request("request", (id, receipt_1)).await;
            
            match result {
                Ok(()) => {}
                Err(e) => panic!("Error making receipt request: {:?}", e),
            }

        }

        }
    Ok(())
}

