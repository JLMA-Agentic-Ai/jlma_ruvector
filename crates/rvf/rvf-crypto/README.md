# rvf-crypto

Cryptographic primitives for RuVector Format -- SHA-3 hashing and Ed25519 signing.

## Overview

`rvf-crypto` provides the cryptographic building blocks used by RVF for data integrity and authentication:

- **SHA-3 hashing** -- content-addressable segment identifiers
- **Ed25519 signing** -- segment-level digital signatures for provenance
- **Key management** -- keypair generation and verification utilities

## Usage

```toml
[dependencies]
rvf-crypto = "0.1"
```

## Features

- `std` (default) -- enable `std` support

## Lineage Witness Functions

`rvf-crypto` provides the cryptographic functions for DNA-style lineage provenance chains.

### `lineage_record_to_bytes()` / `lineage_record_from_bytes()`

Serialize and deserialize a `LineageRecord` to/from a fixed 128-byte array. The codec preserves all fields including the 47-byte description buffer:

```rust
use rvf_crypto::lineage::{lineage_record_to_bytes, lineage_record_from_bytes};
use rvf_types::{LineageRecord, DerivationType};

let record = LineageRecord::new(
    [1u8; 16], [2u8; 16], [3u8; 32],
    DerivationType::Filter, 5, 1_700_000_000_000_000_000,
    "filtered by category",
);
let bytes = lineage_record_to_bytes(&record);
let decoded = lineage_record_from_bytes(&bytes).unwrap();
assert_eq!(decoded.description_str(), "filtered by category");
```

### `lineage_witness_entry()`

Creates a `WitnessEntry` for a derivation event. The `action_hash` is the SHAKE-256-256 digest of the serialized 128-byte record. Uses witness type `WITNESS_DERIVATION` (`0x09`):

```rust
use rvf_crypto::lineage::lineage_witness_entry;

let entry = lineage_witness_entry(&record, [0u8; 32]);
assert_eq!(entry.witness_type, 0x09);
```

### `compute_manifest_hash()`

Computes SHAKE-256-256 over a 4096-byte manifest for use as `parent_hash` in `FileIdentity`:

```rust
use rvf_crypto::lineage::compute_manifest_hash;

let manifest = [0u8; 4096];
let hash = compute_manifest_hash(&manifest);
```

### `verify_lineage_chain()`

Validates a lineage chain from root to leaf. For each child entry it checks:

1. `parent_id` matches the parent's `file_id`
2. `parent_hash` matches the parent's manifest hash
3. `lineage_depth` increments by exactly 1

Returns `Err(LineageBroken)` or `Err(ParentHashMismatch)` on failure:

```rust
use rvf_crypto::lineage::verify_lineage_chain;
use rvf_types::FileIdentity;

let root = FileIdentity::new_root([1u8; 16]);
let root_hash = [0xAAu8; 32];
let child = FileIdentity {
    file_id: [2u8; 16],
    parent_id: [1u8; 16],
    parent_hash: root_hash,
    lineage_depth: 1,
};
verify_lineage_chain(&[(root, root_hash), (child, [0xBBu8; 32])]).unwrap();
```

## License

MIT OR Apache-2.0
