# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]

### Changed

- `umbral-pre` dependency bumped to 0.5 (and to match it, MSRV to 1.56, and Rust edition to 2021). The API was updated accordingly (mainly due to the no-clone approach). Note that this changes the ABI as well. ([#4])
- `NodeMetadataPayload.decentralized_identity_evidence` is now a fixed-sized array in the serialized metadata. ([#2])
- `k256` dependency bumped to 0.10, and `umbral-pre` to 0.5. ([#2])
- `NodeMetadataPayload.canonical_address` and the parameter `ursula_address` of `RevocationOrder::new()` are renamed to `staker_address`. ([#2])


### Added

- WASM bindings. ([#1])
- `NodeMetadataPayload::derive_worker_address()` method. ([#2])


[#1]: https://github.com/nucypher/nucypher-core/pull/4
[#2]: https://github.com/nucypher/nucypher-core/pull/4
[#4]: https://github.com/nucypher/nucypher-core/pull/4

## [0.0.1] - 2021-12-25

Initial release.


[Unreleased]: https://github.com/nucypher/nucypher-core/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/nucypher/nucypher-core/releases/tag/v0.0.1