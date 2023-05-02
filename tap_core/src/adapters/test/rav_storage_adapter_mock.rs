// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::adapters::rav_storage_adapter::RAVStorageAdapter;
use crate::eip_712_signed_message::EIP712SignedMessage;
use crate::receipt_aggregate_voucher::ReceiptAggregateVoucher;

#[derive(Default)]
pub struct RAVStorageAdapterMock {
    rav_storage: HashMap<u64, EIP712SignedMessage<ReceiptAggregateVoucher>>,
    unique_id: u64,
}

impl RAVStorageAdapterMock {
    pub fn new() -> Self {
        RAVStorageAdapterMock {
            rav_storage: <HashMap<u64, EIP712SignedMessage<ReceiptAggregateVoucher>>>::new(),
            unique_id: 0u64,
        }
    }
}

use thiserror::Error;
#[derive(Debug, Error)]
pub enum AdpaterErrorMock {
    #[error("something went wrong: {Error}")]
    AdapterError { Error: String },
}

impl RAVStorageAdapter for RAVStorageAdapterMock {
    type AdapterError = AdpaterErrorMock;

    fn store_rav(
        &mut self,
        rav: EIP712SignedMessage<ReceiptAggregateVoucher>,
    ) -> Result<u64, Self::AdapterError> {
        let id = self.unique_id;
        self.rav_storage.insert(id, rav);
        self.unique_id += 1;
        Ok(id)
    }
    fn retrieve_rav_by_id(
        &self,
        rav_id: u64,
    ) -> Result<EIP712SignedMessage<ReceiptAggregateVoucher>, Self::AdapterError> {
        self.rav_storage
            .get(&rav_id)
            .cloned()
            .ok_or(AdpaterErrorMock::AdapterError {
                Error: "No RAV found with ID".to_owned(),
            })
    }
    fn remove_rav_by_id(&mut self, rav_id: u64) -> Result<(), Self::AdapterError> {
        self.rav_storage
            .remove(&rav_id)
            .map(|_| ())
            .ok_or(AdpaterErrorMock::AdapterError {
                Error: "No RAV found with ID".to_owned(),
            })
    }
}
