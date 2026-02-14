# rvf-node

Node.js N-API bindings for native RuVector Format operations.

## Overview

`rvf-node` exposes the RVF runtime to Node.js via N-API for high-performance vector operations without leaving JavaScript:

- **Async API** -- non-blocking vector store operations
- **Native speed** -- Rust-compiled N-API addon, no serialization overhead
- **Cross-platform** -- builds for Linux, macOS, and Windows
- **Full feature parity** -- lineage, kernel/eBPF embedding, segment inspection

## Build

```bash
cd crates/rvf/rvf-node
npm run build
```

## API

### Store Lifecycle

```typescript
import { RvfDatabase } from '@ruvector/rvf';

// Create
const db = RvfDatabase.create('/tmp/store.rvf', { dimension: 128, metric: 'cosine' });

// Open for read-write
const db = RvfDatabase.open('/tmp/store.rvf');

// Open read-only (no lock)
const db = RvfDatabase.openReadonly('/tmp/store.rvf');

// Close
db.close();
```

### Vector Operations

```typescript
// Ingest vectors
const vectors = new Float32Array([1,0,0,0, 0,1,0,0]);
const result = db.ingestBatch(vectors, [0, 1]);
// { accepted: 2, rejected: 0, epoch: 1 }

// Query
const results = db.query(new Float32Array([1,0,0,0]), 5);
// [{ id: 0, distance: 0.0 }, { id: 1, distance: 1.414 }]

// Query with filter
const results = db.query(new Float32Array([1,0,0,0]), 5, {
  filter: '{"op":"eq","fieldId":0,"valueType":"string","value":"cat_a"}'
});

// Delete by ID
db.delete([0, 1]);

// Delete by filter
db.deleteByFilter('{"op":"gt","fieldId":1,"valueType":"f64","value":"0.5"}');

// Compact
const compaction = db.compact();
// { segmentsCompacted: 2, bytesReclaimed: 4096, epoch: 3 }

// Status
const status = db.status();
// { totalVectors, totalSegments, fileSize, currentEpoch, ... }
```

### Lineage

```typescript
// Get file identity
const fileId = db.fileId();       // "a1b2c3d4..."
const parentId = db.parentId();   // "00000000..." for root
const depth = db.lineageDepth();  // 0 for root

// Derive a child store
const child = db.derive('/tmp/child.rvf', { dimension: 128 });
child.lineageDepth(); // 1
```

### Kernel / eBPF Embedding

```typescript
// Embed a kernel image
const kernelImage = Buffer.from(fs.readFileSync('kernel.bin'));
const segId = db.embedKernel(
  1,              // arch: x86_64
  0,              // kernel_type
  0,              // flags
  kernelImage,    // image bytes
  9000,           // api_port
  'root=/dev/sda' // cmdline (optional)
);

// Extract kernel
const kernel = db.extractKernel();
if (kernel) {
  // kernel.header: Buffer (128-byte KernelHeader)
  // kernel.image: Buffer (kernel image bytes)
}

// Embed an eBPF program
const ebpfCode = Buffer.from(fs.readFileSync('program.o'));
db.embedEbpf(1, 2, 128, ebpfCode);

// Extract eBPF
const ebpf = db.extractEbpf();
if (ebpf) {
  // ebpf.header: Buffer (64-byte EbpfHeader)
  // ebpf.payload: Buffer (bytecode + optional BTF)
}
```

### Segment Inspection

```typescript
// List all segments
const segments = db.segments();
// [{ id: 1, offset: 0, payloadLength: 4096, segType: "vec" }, ...]

// Get dimension
const dim = db.dimension(); // 128
```

## License

MIT OR Apache-2.0
