// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    rav::{ReceiptAggregateVoucher, SignedRAV},
    receipt::{Failed, ReceiptWithState, SignedReceipt},
};

#[derive(Debug)]
pub struct RAVRequest {
    pub valid_receipts: Vec<SignedReceipt>,
    pub previous_rav: Option<SignedRAV>,
    pub invalid_receipts: Vec<ReceiptWithState<Failed>>,
    pub expected_rav: ReceiptAggregateVoucher,
}
