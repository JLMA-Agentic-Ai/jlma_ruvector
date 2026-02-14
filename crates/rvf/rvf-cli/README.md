# rvf-cli

Unified command-line interface for RuVector Format operations.

## Install

```bash
cargo install --path crates/rvf/rvf-cli
```

Or build from source:

```bash
cargo build -p rvf-cli --release
```

The binary is named `rvf`.

## Commands

### create

Create a new empty RVF store.

```bash
rvf create store.rvf --dimension 128 --metric cosine --profile 0
rvf create store.rvf -d 128 -m l2 --json
```

### ingest

Import vectors from JSON, CSV, or TSV files.

```bash
# JSON format: [{"id": 0, "vector": [1.0, 0.0, ...]}, ...]
rvf ingest store.rvf --input data.json --format json

# CSV format: each row is a vector (auto-assigned IDs)
rvf ingest store.rvf --input data.csv --format csv

# TSV
rvf ingest store.rvf --input data.tsv --format tsv --json
```

### query

Search for k nearest neighbors.

```bash
rvf query store.rvf --vector "1.0,0.0,0.0,0.0" --k 10
rvf query store.rvf -v "0.5,0.5,0.0,0.0" -k 5 --json
```

### delete

Delete vectors by ID.

```bash
rvf delete store.rvf --ids 1,2,3
rvf delete store.rvf --ids 42 --json
```

### status

Show store status.

```bash
rvf status store.rvf
rvf status store.rvf --json
```

### inspect

Inspect store segments and lineage.

```bash
rvf inspect store.rvf
rvf inspect store.rvf --json
```

### compact

Reclaim dead space from deleted vectors.

```bash
rvf compact store.rvf
rvf compact store.rvf --json
```

### derive

Create a derived child store from a parent.

```bash
rvf derive parent.rvf child.rvf --derivation-type filter
rvf derive parent.rvf child.rvf --derivation-type snapshot --json
```

### serve

Start an HTTP server (requires `serve` feature).

```bash
cargo build -p rvf-cli --features serve
rvf serve store.rvf --port 8080
```

## JSON Output

All commands support `--json` for machine-readable output suitable for piping to `jq` or other tools.

## License

MIT OR Apache-2.0
