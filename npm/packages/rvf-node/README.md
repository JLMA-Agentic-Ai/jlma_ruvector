# @ruvector/rvf-node

Node.js native bindings for the RuVector Format (RVF) cognitive container.

## Install

```bash
npm install @ruvector/rvf-node
```

## Usage

```javascript
const { RvfDatabase } = require('@ruvector/rvf-node');

// Create a vector store
const db = RvfDatabase.create('vectors.rvf', { dimension: 384 });

// Insert vectors
db.ingestBatch(new Float32Array(384), [1]);

// Query nearest neighbors
const results = db.query(new Float32Array(384), 10);
console.log(results); // [{ id, distance }]

// Inspect segments
console.log(db.fileId());       // unique file UUID
console.log(db.dimension());    // 384
console.log(db.segments());     // [{ type, id, size }]

db.close();
```

## Features

- Native performance via N-API bindings to Rust `rvf-runtime`
- Full store lifecycle: create, open, ingest, query, delete, compact
- Lineage tracking with FileIdentity derivation chains
- Kernel/eBPF segment inspection
- Cross-platform: Linux x64/arm64, macOS x64/arm64, Windows x64

## API

| Method | Description |
|--------|-------------|
| `RvfDatabase.create(path, options)` | Create a new RVF store |
| `RvfDatabase.open(path)` | Open an existing store |
| `db.ingestBatch(vectors, ids)` | Insert vectors |
| `db.query(vector, k)` | k-NN similarity search |
| `db.delete(ids)` | Delete vectors by ID |
| `db.compact()` | Reclaim deleted space |
| `db.status()` | Get store stats |
| `db.segments()` | List all segments |
| `db.fileId()` | Get unique file UUID |
| `db.close()` | Close and release lock |

## License

MIT
