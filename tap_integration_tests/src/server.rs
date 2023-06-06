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

/// Rpc trait represents a JSON-RPC server that has a single async method `request`.
/// This method is designed to handle incoming JSON-RPC requests.
#[rpc(server)]
pub trait Rpc {
    // This async method is designed to handle incoming JSON-RPC requests.
    #[method(name = "request")]
    async fn request(
        &self,
        request_id: u64, // Unique identifier for the request
        receipt: SignedReceipt, // Signed receipt associated with the request
    ) -> Result<(), jsonrpsee::types::ErrorObjectOwned>; // The result of the request, a JSON-RPC error if it fails
}

/// RpcManager is a struct that implements the `Rpc` trait and it represents a JSON-RPC server manager.
/// It includes a manager, initial_checks, receipt_count, threshold and aggregator_client.
/// Manager holds a Mutex-protected instance of a generic `Manager` object which is shared and can be accessed by multiple threads.
/// initial_checks is a list of checks that needs to be performed for every incoming request.
/// receipt_count is a thread-safe counter that increments with each receipt verified and stored.
/// threshold is a limit to which receipt_count can increment, after reaching which RAV request is triggered.
/// aggregator_client is an HTTP client used for making JSON-RPC requests to another server.
pub struct RpcManager<
    CA: CollateralAdapter + Send + 'static,  // An instance of CollateralAdapter, marked as thread-safe with Send and given 'static lifetime
    RCA: ReceiptChecksAdapter + Send + 'static, // An instance of ReceiptChecksAdapter
    RSA: ReceiptStorageAdapter + Send + 'static, // An instance of ReceiptStorageAdapter
    RAVSA: RAVStorageAdapter + Send + 'static, // An instance of RAVStorageAdapter
> {
    manager: Arc<Mutex<Manager<CA, RCA, RSA, RAVSA>>>, // Manager object in a mutex for thread safety, reference counted with an Arc
    initial_checks: Vec<ReceiptCheck>, // Vector of initial checks to be performed on each request
    receipt_count: Arc<AtomicU64>, // Thread-safe atomic counter for receipts
    threshold: u64, // The count at which a RAV request will be triggered
    aggregator_client: HttpClient, // HTTP client for sending requests to the aggregator server
}


/// Implementation for `RpcManager`, includes the constructor and the `request` method.
/// Constructor initializes a new instance of `RpcManager`.
/// `request` method handles incoming JSON-RPC requests and it verifies and stores the receipt from the request.
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

/// request_rav function creates a request for aggregate receipts (RAV), sends it to another server and verifies the result.
async fn request_rav<
    CA: CollateralAdapter + Send + 'static,
    RCA: ReceiptChecksAdapter + Send + 'static,
    RSA: ReceiptStorageAdapter + Send + 'static,
    RAVSA: RAVStorageAdapter + Send + 'static,
>(
    manager: Arc<Mutex<Manager<CA, RCA, RSA, RAVSA>>>, // Mutex-protected manager object for thread safety
    time_stamp_buffer: u64, // Buffer for timestamping, see tap_core for details
    aggregator_client: HttpClient, // HttpClient for making requests to the tap_aggregator server
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

/// run_server function initializes and starts a JSON-RPC server that handles incoming requests.
pub async fn run_server<
    CA: CollateralAdapter + Send + 'static,
    RCA: ReceiptChecksAdapter + Send + 'static,
    RSA: ReceiptStorageAdapter + Send + 'static,
    RAVSA: RAVStorageAdapter + Send + 'static,
>(
    port: u16, // Port on which the server will listen
    collateral_adapter: CA, // CollateralAdapter instance
    receipt_checks_adapter: RCA, // ReceiptChecksAdapter instance
    receipt_storage_adapter: RSA, // ReceiptStorageAdapter instance
    rav_storage_adapter: RAVSA, // RAVStorageAdapter instance
    initial_checks: Vec<ReceiptCheck>, // Vector of initial checks to be performed on each request
    required_checks: Vec<ReceiptCheck>, // Vector of required checks to be performed on each request
    threshold: u64, // The count at which a RAV request will be triggered
    aggregate_server_address: String, // Address of the aggregator server
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

/// get_current_timestamp_u64_ns function returns current system time since UNIX_EPOCH as a 64-bit unsigned integer.
fn get_current_timestamp_u64_ns() -> Result<u64, Error> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| Error::InvalidSystemTime {
            source_error_message: err.to_string(),
        })?
        .as_nanos() as u64)
}
