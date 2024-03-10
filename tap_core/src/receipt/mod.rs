// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod checks;
mod error;
mod receipt;
mod received_receipt;

pub use error::ReceiptError;
pub use receipt::Receipt;
pub use received_receipt::{
    AwaitingReserve, CategorizedReceiptsWithState, Checking, Failed, ReceiptState, ReceiptWithId,
    ReceiptWithState, ReceivedReceipt, Reserved, ResultReceipt,
};

use crate::signed_message::EIP712SignedMessage;

pub type SignedReceipt = EIP712SignedMessage<Receipt>;
pub type ReceiptResult<T> = Result<T, ReceiptError>;
