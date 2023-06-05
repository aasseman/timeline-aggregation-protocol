// manager_server.rs
use anyhow::Result;
use tap_core::tap_manager::RAVRequest;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use jsonrpsee::core::async_trait;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::rpc_params;
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use jsonrpsee::{proc_macros::rpc, http_client::HttpClientBuilder};
use tap_core::eip_712_signed_message::EIP712SignedMessage;
use tap_core::receipt_aggregate_voucher::ReceiptAggregateVoucher;
use tap_core::Error;
use tap_core::{tap_manager::{Manager, SignedReceipt}, adapters::{ receipt_checks_adapter::ReceiptChecksAdapter, rav_storage_adapter::RAVStorageAdapter, receipt_storage_adapter::ReceiptStorageAdapter, collateral_adapter::CollateralAdapter}, tap_receipt::ReceiptCheck};

#[rpc(server)]
pub trait Rpc {
    #[method(name = "request")]
    async fn request(&self, request_id: u64, receipt: SignedReceipt) -> Result<(), jsonrpsee::types::ErrorObjectOwned>;
}

pub struct RpcManager
    <CA: CollateralAdapter+Sync+Send+'static,
    RCA: ReceiptChecksAdapter+Sync+Send+'static,
    RSA: ReceiptStorageAdapter+Sync+Send+'static,
    RAVSA: RAVStorageAdapter+Sync+Send+'static> {
    manager: Arc<Mutex<Manager<CA, RCA, RSA, RAVSA>>>,
    initial_checks: Vec<ReceiptCheck>,
    receipt_count: Arc<AtomicU64>,
    threshold: u64,
    aggregate_server_address: String,
}

impl < CA: CollateralAdapter+Sync+Send+'static,
    RCA: ReceiptChecksAdapter+Sync+Send+'static,
    RSA: ReceiptStorageAdapter+Sync+Send+'static,
    RAVSA: RAVStorageAdapter+Sync+Send+'static>
    RpcManager<CA, RCA, RSA, RAVSA> {
    pub fn new(
        collateral_adapter: CA, 
        receipt_checks_adapter: RCA, 
        receipt_storage_adapter: RSA, 
        rav_storage_adapter: RAVSA, 
        initial_checks: Vec<ReceiptCheck>,
        required_checks: Vec<ReceiptCheck>,
        threshold: u64, 
        aggregate_server_address: String) -> Self {
        Self {
            manager: Arc::new(Mutex::new(Manager::<CA, RCA, RSA, RAVSA>::new(collateral_adapter, receipt_checks_adapter, rav_storage_adapter, receipt_storage_adapter, required_checks, get_current_timestamp_u64_ns().unwrap()))),
            initial_checks,
            receipt_count: Arc::new(AtomicU64::new(0)),
            threshold,
            aggregate_server_address,
        }
    }
}
#[async_trait]
impl<CA: CollateralAdapter+Sync+Send+'static,
RCA: ReceiptChecksAdapter+Sync+Send+'static,
RSA: ReceiptStorageAdapter+Sync+Send+'static,
RAVSA: RAVStorageAdapter+Sync+Send+'static>
RpcServer for RpcManager<CA, RCA, RSA, RAVSA> {

async fn request(&self, 
    request_id: u64, 
    receipt: SignedReceipt) -> Result<(), jsonrpsee::types::ErrorObjectOwned> {
    // clone before moving to closure
    let manager_clone = Arc::clone(&self.manager);
    let manager_clone_rav = Arc::clone(&self.manager);
    let manager_clone_rav_check = Arc::clone(&self.manager);
    let receipt_count_clone = Arc::clone(&self.receipt_count);
    let initial_checks = self.initial_checks.clone();
    let result = tokio::task::spawn_blocking(move || {
        manager_clone.lock().unwrap().verify_and_store_receipt(receipt, request_id, initial_checks)
    }).await.map_err(|e| jsonrpsee::types::ErrorObject::owned(
        -32000,
        format!("Failed to spawn task: {}", e),
        None::<()>))?;

    match result {
        Ok(result) => {
            // Increment the receipt count
            receipt_count_clone.fetch_add(1, Ordering::Relaxed);

            if self.receipt_count.load(Ordering::SeqCst) >= self.threshold {
                // Reset the counter after reaching the threshold
                self.receipt_count.store(0, Ordering::SeqCst);

                // Connect to the other server
                let client = HttpClientBuilder::default()
                    .build(format!("{}", self.aggregate_server_address))
                    .unwrap();

                println!("Requesting RAV...");
                // Create the aggregate_receipts request params
                let time_stamp_buffer = 0;
                let rav_result = request_rav(manager_clone_rav, time_stamp_buffer).await;

                match rav_result {
                    Ok(rav) => {
                        let params = rpc_params!(&rav.valid_receipts, None::<()>);
                        // Call the aggregate_receipts method on the other server
                        let remote_rav_result: Result<EIP712SignedMessage<ReceiptAggregateVoucher>, _> = client.request("aggregate_receipts", params).await;
                        
                        match remote_rav_result {
                            Ok(remote_rav) => {
                                // Spawn a new OS thread to run the blocking function
                                let result = tokio::task::spawn_blocking(move || {
                                    manager_clone_rav_check.lock().unwrap().verify_and_store_rav(rav.expected_rav, remote_rav)
                                }).await.map_err(|e| jsonrpsee::types::ErrorObject::owned(
                                    -32000,
                                    format!("Failed to spawn task: {}", e),
                                    None::<()>))?;

                                let _ = match result {
                                    Ok(_) => Ok(()),
                                    Err(e) => Err(jsonrpsee::types::ErrorObject::owned(
                                        -32000,
                                        e.to_string(),
                                        None::<()>,
                                    )),
                                };
                            },
                            Err(e) => {
                                return Err(jsonrpsee::types::ErrorObject::owned(
                                    -32000,
                                    e.to_string(),
                                    None::<()>,
                                ))
                            }
                        }
                    },
                    Err(e) => {
                        return Err(jsonrpsee::types::ErrorObject::owned(
                            -32000,
                            e.to_string(),
                            None::<()>,
                        ))
                    }
                }
            }
            Ok(result)
        },
        Err(e) => Err(jsonrpsee::types::ErrorObject::owned(
            -32000,
            e.to_string(),
            None::<()>,
        )),
    }
}
}

pub async fn request_rav<CA: CollateralAdapter+Sync+Send+'static,
RCA: ReceiptChecksAdapter+Sync+Send+'static,
RSA: ReceiptStorageAdapter+Sync+Send+'static,
RAVSA: RAVStorageAdapter+Sync+Send+'static> (
    manager: Arc<Mutex<Manager<CA, RCA, RSA, RAVSA>>>,
    time_stamp_buffer: u64,
) -> Result<RAVRequest, jsonrpsee::types::ErrorObjectOwned> {
    let result = tokio::task::spawn_blocking(move || {
        manager.lock().unwrap().create_rav_request(time_stamp_buffer)
    }).await.map_err(|e| jsonrpsee::types::ErrorObject::owned(
        -32000,
        format!("Failed to spawn task: {}", e),
        None::<()>))?;

    match result {
        Ok(rav) => Ok(rav),
        Err(e) => Err(jsonrpsee::types::ErrorObject::owned(
            -32000,
            e.to_string(),
            None::<()>,
        )),
    }
}

pub async fn run_server<CA: CollateralAdapter+Sync+Send+'static,
RCA: ReceiptChecksAdapter+Sync+Send+'static,
RSA: ReceiptStorageAdapter+Sync+Send+'static,
RAVSA: RAVStorageAdapter+Sync+Send+'static> (
    port: u16,
    collateral_adapter: CA,
    receipt_checks_adapter: RCA,
    receipt_storage_adapter: RSA,
    rav_storage_adapter: RAVSA,
    initial_checks: Vec<ReceiptCheck>,
    required_checks: Vec<ReceiptCheck>,
    threshold: u64,
    aggregate_server_address: String,
) -> Result<(ServerHandle, std::net::SocketAddr)> {
// Setting up the JSON RPC server
println!("Starting server...");
let server = ServerBuilder::new()
    .http_only()
    .build(format!("127.0.0.1:{}", port))
    .await?;
let addr = server.local_addr()?;
println!("Listening on: {}", addr);
let rpc_manager = 
RpcManager::new(
    collateral_adapter, 
    receipt_checks_adapter, 
    receipt_storage_adapter, 
    rav_storage_adapter, 
    initial_checks,
    required_checks, 
    threshold,
    aggregate_server_address);



let handle = server.start(rpc_manager.into_rpc())?;
Ok((handle, addr))
}

pub fn get_current_timestamp_u64_ns() -> Result<u64, Error> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| Error::InvalidSystemTime {
            source_error_message: err.to_string(),
        })?
        .as_nanos() as u64)
}
