# @ruvector/rvf

Unified TypeScript SDK for the RuVector Format (RVF) cognitive container. A single `.rvf` file stores vectors, carries models, boots services, and proves everything.

## Install

```bash
npm install @ruvector/rvf
```

## Usage

```typescript
import { RvfDatabase } from '@ruvector/rvf';

// Create a vector store
const db = RvfDatabase.create('vectors.rvf', { dimension: 384 });

// Insert vectors
db.ingestBatch(new Float32Array(384), [1]);

// Query nearest neighbors
const results = db.query(new Float32Array(384), 10);

// Lineage & inspection
console.log(db.fileId());       // unique file UUID
console.log(db.dimension());    // 384
console.log(db.segments());     // [{ type, id, size }]

db.close();
```

## What is RVF?

RVF (RuVector Format) is a universal binary substrate that merges database, model, graph engine, kernel, and attestation into a single deployable file.

| Capability | Segment |
|------------|---------|
| Vector storage | VEC_SEG + INDEX_SEG |
| LoRA adapters | OVERLAY_SEG |
| Graph state | GRAPH_SEG |
| Self-boot Linux | KERNEL_SEG |
| eBPF acceleration | EBPF_SEG |
| Browser queries | WASM_SEG |
| Witness chains | WITNESS_SEG + CRYPTO_SEG |
| COW branching | COW_MAP + MEMBERSHIP |

## Packages

| Package | Description |
|---------|-------------|
| `@ruvector/rvf` | Unified SDK (this package) |
| `@ruvector/rvf-node` | Native N-API bindings |
| `@ruvector/rvf-wasm` | WASM build for browsers |
| `@ruvector/rvf-mcp-server` | MCP server for AI agents |

## License

MIT
