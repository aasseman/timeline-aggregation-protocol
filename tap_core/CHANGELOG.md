# Changelog

## 0.1.0 (2023-06-28)


### ⚠ BREAKING CHANGES

* **tap-manager:** RAV Request definition is updated to include optional previous RAV
* **receipts:** Updates the receipt checks adapter api
* **signed-message:** Updates signed message API
* **allocation-adapter:** removes allocation adapter
* **adapters:** existing adapter trait definitions are updated

### Features

* **adapter-mocks:** alters adapters to use references (arc) to allow sharing common resources ([03e7668](https://github.com/aasseman/timeline-aggregation-protocol/commit/03e7668e3c59d27e6cfc869a3a35ad1434d18d6d))
* **adapters:** adds functionality to retrieve and delete receipts within a range ([4143ac6](https://github.com/aasseman/timeline-aggregation-protocol/commit/4143ac6293751a0e837709bba43d4fc600911bcc))
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
* **tap-manager:** adds an error when receipt timestamp window is inverted ([e21d7d9](https://github.com/aasseman/timeline-aggregation-protocol/commit/e21d7d941b8b298bdda5dc8143b3163f65ca1e85))
* **tap-manager:** receipts being used after being added to RAV ([efd88a2](https://github.com/aasseman/timeline-aggregation-protocol/commit/efd88a214a3737b7bb201cabaf4037284ec5d547))
* verification benchmarks ([#114](https://github.com/aasseman/timeline-aggregation-protocol/issues/114)) ([96cdf24](https://github.com/aasseman/timeline-aggregation-protocol/commit/96cdf24db98a715cec654ff77de1837ba36f81a4))
