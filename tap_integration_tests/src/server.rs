// manager_server.rs
use anyhow::Result;
use jsonrpsee::core::async_trait;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClient;
use jsonrpsee::rpc_params;
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use jsonrpsee::{http_client::HttpClientBuilder, proc_macros::rpc};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tap_core::Error;
use tap_core::{
    adapters::{
        collateral_adapter::CollateralAdapter, rav_storage_adapter::RAVStorageAdapter,
        receipt_checks_adapter::ReceiptChecksAdapter,
        receipt_storage_adapter::ReceiptStorageAdapter,
    },
    tap_manager::{Manager, SignedReceipt},
    tap_receipt::ReceiptCheck,
};

#[rpc(server)]
pub trait Rpc {
    #[method(name = "request")]
    async fn request(
        &self,
        request_id: u64,
        receipt: SignedReceipt,
    ) -> Result<(), jsonrpsee::types::ErrorObjectOwned>;
}

pub struct RpcManager<
    CA: CollateralAdapter + Send + 'static,
    RCA: ReceiptChecksAdapter + Send + 'static,
    RSA: ReceiptStorageAdapter + Send + 'static,
    RAVSA: RAVStorageAdapter + Send + 'static,
> {
    manager: Arc<Mutex<Manager<CA, RCA, RSA, RAVSA>>>,
    initial_checks: Vec<ReceiptCheck>,
    receipt_count: Arc<AtomicU64>,
    threshold: u64,
    aggregator_client: HttpClient,
}

impl<
        CA: CollateralAdapter + Send + 'static,
        RCA: ReceiptChecksAdapter + Send + 'static,
        RSA: ReceiptStorageAdapter + Send + 'static,
        RAVSA: RAVStorageAdapter + Send + 'static,
    > RpcManager<CA, RCA, RSA, RAVSA>
{
    pub fn new(
        collateral_adapter: CA,
        receipt_checks_adapter: RCA,
        receipt_storage_adapter: RSA,
        rav_storage_adapter: RAVSA,
        initial_checks: Vec<ReceiptCheck>,
        required_checks: Vec<ReceiptCheck>,
        threshold: u64,
        aggregate_server_address: String,
    ) -> Self {
        Self {
            manager: Arc::new(Mutex::new(Manager::<CA, RCA, RSA, RAVSA>::new(
                collateral_adapter,
                receipt_checks_adapter,
                rav_storage_adapter,
                receipt_storage_adapter,
                required_checks,
                get_current_timestamp_u64_ns().unwrap(),
            ))),
            initial_checks,
            receipt_count: Arc::new(AtomicU64::new(0)),
            threshold,
            aggregator_client: HttpClientBuilder::default()
                .build(format!("{}", aggregate_server_address))
                .unwrap(),
        }
    }
}
#[async_trait]
impl<
        CA: CollateralAdapter + Send + 'static,
        RCA: ReceiptChecksAdapter + Send + 'static,
        RSA: ReceiptStorageAdapter + Send + 'static,
        RAVSA: RAVStorageAdapter + Send + 'static,
    > RpcServer for RpcManager<CA, RCA, RSA, RAVSA>
{
    async fn request(
        &self,
        request_id: u64,
        receipt: SignedReceipt,
    ) -> Result<(), jsonrpsee::types::ErrorObjectOwned> {
        let result = Arc::clone(&self.manager)
            .lock()
            .unwrap()
            .verify_and_store_receipt(receipt, request_id, self.initial_checks.clone());

        match result {
            Ok(result) => {
                // Increment the receipt count
                self.receipt_count.fetch_add(1, Ordering::Relaxed);

                if self.receipt_count.load(Ordering::SeqCst) >= self.threshold {
                    // Reset the counter after reaching the threshold
                    self.receipt_count.store(0, Ordering::SeqCst);
                    println!("Requesting RAV...");

                    // Create the aggregate_receipts request params
                    let time_stamp_buffer = 0;
                    match request_rav(
                        Arc::clone(&self.manager),
                        time_stamp_buffer,
                        self.aggregator_client.clone(),
                    )
                    .await
                    {
                        Ok(value) => value,
                        Err(e) => {
                            return Err(jsonrpsee::types::ErrorObject::owned(
                                -32000,
                                e.to_string() + " - Rav request failed",
                                None::<()>,
                            ))
                        }
                    };
                }
                Ok(result)
            }
            Err(e) => Err(jsonrpsee::types::ErrorObject::owned(
                -32000,
                e.to_string() + " - Verify and store receipt failed",
                None::<()>,
            )),
        }
    }
}

async fn request_rav<
    CA: CollateralAdapter + Send + 'static,
    RCA: ReceiptChecksAdapter + Send + 'static,
    RSA: ReceiptStorageAdapter + Send + 'static,
    RAVSA: RAVStorageAdapter + Send + 'static,
>(
    manager: Arc<Mutex<Manager<CA, RCA, RSA, RAVSA>>>,
    time_stamp_buffer: u64,
    aggregator_client: HttpClient,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create the aggregate_receipts request params
    let rav = manager
        .lock()
        .unwrap()
        .create_rav_request(time_stamp_buffer)?;
    let params = rpc_params!(&rav.valid_receipts, None::<()>);
    // Call the aggregate_receipts method on the other server
    let remote_rav_result = aggregator_client
        .request("aggregate_receipts", params)
        .await?;
    let _result = manager
        .clone()
        .lock()
        .unwrap()
        .verify_and_store_rav(rav.expected_rav, remote_rav_result)?;
    Ok(())
}

pub async fn run_server<
    CA: CollateralAdapter + Send + 'static,
    RCA: ReceiptChecksAdapter + Send + 'static,
    RSA: ReceiptStorageAdapter + Send + 'static,
    RAVSA: RAVStorageAdapter + Send + 'static,
>(
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
    let rpc_manager = RpcManager::new(
        collateral_adapter,
        receipt_checks_adapter,
        receipt_storage_adapter,
        rav_storage_adapter,
        initial_checks,
        required_checks,
        threshold,
        aggregate_server_address,
    );

    let handle = server.start(rpc_manager.into_rpc())?;
    Ok((handle, addr))
}

fn get_current_timestamp_u64_ns() -> Result<u64, Error> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| Error::InvalidSystemTime {
            source_error_message: err.to_string(),
        })?
        .as_nanos() as u64)
}
