# Changelog

## [0.2.0](https://github.com/aasseman/timeline-aggregation-protocol/compare/tap_core-v0.1.0...tap_core-v0.2.0) (2023-06-07)


### ⚠ BREAKING CHANGES

* dummy

### Features

* dummy ([e950c9f](https://github.com/aasseman/timeline-aggregation-protocol/commit/e950c9fe37fdc1d5b3f58d7b5ca759f51f884388))

## 0.1.0 (2023-06-07)


### ⚠ BREAKING CHANGES

* **receipts:** Updates the receipt checks adapter api
* **signed-message:** Updates signed message API
* **allocation-adapter:** removes allocation adapter
* **adapters:** existing adapter trait definitions are updated

### Features

* **adapter-mocks:** alters adapters to use references (arc) to allow sharing common resources ([03e7668](https://github.com/aasseman/timeline-aggregation-protocol/commit/03e7668e3c59d27e6cfc869a3a35ad1434d18d6d))
* **adapters:** split adapters into storage and check adapters ([39c1c82](https://github.com/aasseman/timeline-aggregation-protocol/commit/39c1c827447aceb938ad03643bb7bf08ff330cae))
* **core:** add `verify` to EIP712 signed msg ([a7e3e7d](https://github.com/aasseman/timeline-aggregation-protocol/commit/a7e3e7d18044dbe6937cf725376167171fb177b1))
* **receipt-storage:** Adds functionality to update receipt in storage by id ([eb4f8ba](https://github.com/aasseman/timeline-aggregation-protocol/commit/eb4f8bae233406b6c5d25def4de1d628d7860b1e))
* **receipts:** Updates checking mechanisms and adds auditor to complete checks ([8d0088d](https://github.com/aasseman/timeline-aggregation-protocol/commit/8d0088d6fbc83416737cf33c3e305412741c8ec8))
* **signed-message:** Updates library to use ether-rs for wallet, address, key, signature, and verification ([7f1cb85](https://github.com/aasseman/timeline-aggregation-protocol/commit/7f1cb8586e7577221008588cacd5cc6ad47d1c83))
* **tap-manager:** Adds a tap manager for handling receipt and rav validation and storage ([3786042](https://github.com/aasseman/timeline-aggregation-protocol/commit/378604263a91d3abd9fdad1dd978f1ba715f7aca))


### Bug Fixes

* **allocation-adapter:** remove obsolete trait ([957f3f9](https://github.com/aasseman/timeline-aggregation-protocol/commit/957f3f99efa8b5ebe7f61024f96905b0448c4bed))
* **receipt-errors:** updates receipt errors to work with adapter trait api ([fc121bf](https://github.com/aasseman/timeline-aggregation-protocol/commit/fc121bf21bee2b8bdf0d1db026e97afb2844d75b))
* **receipts:** adds receipt ID to check unique to allow checking after receipt is in storage ([5072fb9](https://github.com/aasseman/timeline-aggregation-protocol/commit/5072fb9ba614d58bcc712778deb451eaefbc993f))
