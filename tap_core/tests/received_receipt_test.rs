// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, RwLock},
};

use alloy_primitives::Address;
use alloy_sol_types::Eip712Domain;
use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
use rstest::*;
use tap_core::{
    manager::context::memory::{
        checks::get_full_list_of_checks, EscrowStorage, InMemoryContext, QueryAppraisals,
    },
    receipt::{
        checks::{ReceiptCheck, TimestampCheck},
        Receipt, ReceiptWithState,
    },
    signed_message::EIP712SignedMessage,
    tap_eip712_domain,
};

#[fixture]
fn keys() -> (LocalWallet, Address) {
    let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();
    // Alloy library does not have feature parity with ethers library (yet) This workaround is needed to get the address
    // to convert to an alloy Address. This will not be needed when the alloy library has wallet support.
    let address: [u8; 20] = wallet.address().into();

    (wallet, address.into())
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
fn sender_ids() -> Vec<Address> {
    vec![
        Address::from_str("0xfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfb").unwrap(),
        Address::from_str("0xfafafafafafafafafafafafafafafafafafafafa").unwrap(),
        Address::from_str("0xadadadadadadadadadadadadadadadadadadadad").unwrap(),
        keys().1,
    ]
}

#[fixture]
fn domain_separator() -> Eip712Domain {
    tap_eip712_domain(1, Address::from([0x11u8; 20]))
}

struct ContextFixture {
    context: InMemoryContext,
    escrow_storage: EscrowStorage,
    query_appraisals: QueryAppraisals,
    checks: Vec<ReceiptCheck>,
}

#[fixture]
fn context(
    domain_separator: Eip712Domain,
    allocation_ids: Vec<Address>,
    sender_ids: Vec<Address>,
    keys: (LocalWallet, Address),
) -> ContextFixture {
    let escrow_storage = Arc::new(RwLock::new(HashMap::new()));
    let rav_storage = Arc::new(RwLock::new(None));
    let receipt_storage = Arc::new(RwLock::new(HashMap::new()));
    let query_appraisals = Arc::new(RwLock::new(HashMap::new()));

    let timestamp_check = Arc::new(TimestampCheck::new(0));
    let context = InMemoryContext::new(
        rav_storage,
        receipt_storage.clone(),
        escrow_storage.clone(),
        timestamp_check.clone(),
    )
    .with_sender_address(keys.1);
    let mut checks = get_full_list_of_checks(
        domain_separator,
        sender_ids.iter().cloned().collect(),
        Arc::new(RwLock::new(allocation_ids.iter().cloned().collect())),
        query_appraisals.clone(),
    );
    checks.push(timestamp_check);

    ContextFixture {
        context,
        escrow_storage,
        query_appraisals,
        checks,
    }
}

#[rstest]
#[tokio::test]
async fn partial_then_full_check_valid_receipt(
    keys: (LocalWallet, Address),
    domain_separator: Eip712Domain,
    allocation_ids: Vec<Address>,
    context: ContextFixture,
) {
    let ContextFixture {
        checks,
        escrow_storage,
        query_appraisals,
        ..
    } = context;

    let query_value = 20u128;
    let signed_receipt = EIP712SignedMessage::new(
        &domain_separator,
        Receipt::new(allocation_ids[0], query_value).unwrap(),
        &keys.0,
    )
    .unwrap();

    let query_id = signed_receipt.unique_hash();

    // add escrow for sender
    escrow_storage
        .write()
        .unwrap()
        .insert(keys.1, query_value + 500);
    // appraise query
    query_appraisals
        .write()
        .unwrap()
        .insert(query_id, query_value);

    let mut received_receipt = ReceiptWithState::new(signed_receipt);

    let result = received_receipt.perform_checks(&checks).await;
    assert!(result.is_ok());
}

#[rstest]
#[tokio::test]
async fn partial_then_finalize_valid_receipt(
    keys: (LocalWallet, Address),
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    context: ContextFixture,
) {
    let ContextFixture {
        checks,
        context,
        escrow_storage,
        query_appraisals,
        ..
    } = context;

    let query_value = 20u128;
    let signed_receipt = EIP712SignedMessage::new(
        &domain_separator,
        Receipt::new(allocation_ids[0], query_value).unwrap(),
        &keys.0,
    )
    .unwrap();
    let query_id = signed_receipt.unique_hash();

    // add escrow for sender
    escrow_storage
        .write()
        .unwrap()
        .insert(keys.1, query_value + 500);
    // appraise query
    query_appraisals
        .write()
        .unwrap()
        .insert(query_id, query_value);

    let received_receipt = ReceiptWithState::new(signed_receipt);

    let awaiting_escrow_receipt = received_receipt.finalize_receipt_checks(&checks).await;
    assert!(awaiting_escrow_receipt.is_ok());

    let awaiting_escrow_receipt = awaiting_escrow_receipt.unwrap();
    let receipt = awaiting_escrow_receipt
        .check_and_reserve_escrow(&context, &domain_separator)
        .await;
    assert!(receipt.is_ok());
}

#[rstest]
#[tokio::test]
async fn standard_lifetime_valid_receipt(
    keys: (LocalWallet, Address),
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    context: ContextFixture,
) {
    let ContextFixture {
        checks,
        context,
        escrow_storage,
        query_appraisals,
        ..
    } = context;

    let query_value = 20u128;
    let signed_receipt = EIP712SignedMessage::new(
        &domain_separator,
        Receipt::new(allocation_ids[0], query_value).unwrap(),
        &keys.0,
    )
    .unwrap();

    let query_id = signed_receipt.unique_hash();

    // add escrow for sender
    escrow_storage
        .write()
        .unwrap()
        .insert(keys.1, query_value + 500);
    // appraise query
    query_appraisals
        .write()
        .unwrap()
        .insert(query_id, query_value);

    let received_receipt = ReceiptWithState::new(signed_receipt);

    let awaiting_escrow_receipt = received_receipt.finalize_receipt_checks(&checks).await;
    assert!(awaiting_escrow_receipt.is_ok());

    let awaiting_escrow_receipt = awaiting_escrow_receipt.unwrap();
    let receipt = awaiting_escrow_receipt
        .check_and_reserve_escrow(&context, &domain_separator)
        .await;
    assert!(receipt.is_ok());
}
