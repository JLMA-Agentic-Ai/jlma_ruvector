# RuVector Edge - Distributed AI Swarm Communication

Edge AI swarm communication using `ruv-swarm-transport` with RuVector intelligence synchronization.

## Features

- **🌐 Multi-Transport**: WebSocket, SharedMemory, and WASM support
- **🧠 Distributed Learning**: Sync Q-learning patterns across agents
- **💾 Shared Memory**: Vector memory for collaborative RAG
- **📦 Tensor Compression**: LZ4 + quantization for efficient transfer
- **🔄 Real-time Sync**: Automatic pattern propagation
- **🎯 Agent Roles**: Coordinator, Worker, Scout, Specialist

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    ruv-swarm-transport                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  WebSocket   │  │ SharedMemory │  │    WASM      │      │
│  │  (Remote)    │  │   (Local)    │  │  (Browser)   │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
│         └─────────────────┼─────────────────┘              │
│                           │                                 │
│  ┌────────────────────────┴────────────────────────┐       │
│  │              RuVector Integration                │       │
│  │                                                  │       │
│  │  ┌─────────────┐  ┌─────────────┐  ┌──────────┐ │       │
│  │  │ Intelligence │  │   Vector    │  │  Tensor  │ │       │
│  │  │    Sync      │  │   Memory    │  │ Compress │ │       │
│  │  └─────────────┘  └─────────────┘  └──────────┘ │       │
│  └──────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Installation

```bash
# Add to your Cargo.toml
cargo add ruv-swarm-transport

# Or build this example
cd examples/edge
cargo build --release
```

### Run Demo

```bash
# Run the demo (local swarm simulation)
cargo run --bin edge-demo

# Expected output:
# 🚀 RuVector Edge Swarm Demo
# ✅ Coordinator created: coordinator-001
# ✅ Worker created: worker-001
# ✅ Worker created: worker-002
# ✅ Worker created: worker-003
# 📚 Simulating distributed learning...
```

### Run Coordinator

```bash
# Start a coordinator
cargo run --bin edge-coordinator -- --id coord-001

# With WebSocket transport
cargo run --bin edge-coordinator -- --transport websocket --listen 0.0.0.0:8080
```

### Run Agent

```bash
# Start a worker agent
cargo run --bin edge-agent -- --role worker

# Connect to coordinator
cargo run --bin edge-agent -- --coordinator ws://localhost:8080

# As a scout
cargo run --bin edge-agent -- --role scout --id scout-001
```

## Usage

### Create a Swarm Agent

```rust
use ruvector_edge::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let config = SwarmConfig::default()
        .with_agent_id("my-agent")
        .with_role(AgentRole::Worker)
        .with_transport(Transport::WebSocket);

    let mut agent = SwarmAgent::new(config).await?;

    // Join swarm
    agent.join_swarm("ws://coordinator:8080").await?;

    // Learn from experience
    agent.learn("edit_ts", "typescript-developer", 0.9).await;

    // Get best action
    let actions = vec!["coder".to_string(), "reviewer".to_string()];
    if let Some((action, confidence)) = agent.get_best_action("edit_ts", &actions).await {
        println!("Best action: {} ({:.0}% confidence)", action, confidence * 100.0);
    }

    // Store vector memory
    let embedding = vec![0.1, 0.2, 0.3, 0.4];
    agent.store_memory("API authentication flow", embedding).await?;

    // Search memory
    let query = vec![0.1, 0.2, 0.3, 0.4];
    let results = agent.search_memory(&query, 5).await;

    Ok(())
}
```

### Distributed Learning Sync

```rust
use ruvector_edge::intelligence::IntelligenceSync;

// Create sync manager
let sync = IntelligenceSync::new("agent-001");

// Update patterns locally
sync.update_pattern("edit_rs", "rust-developer", 0.95).await;

// Serialize for network transfer
let data = sync.serialize_state().await?;

// Merge peer state (federated learning)
let merge_result = sync.merge_peer_state("peer-002", &peer_data).await?;
println!("Merged {} patterns from peer", merge_result.merged_patterns);

// Get aggregated stats
let stats = sync.get_swarm_stats().await;
println!("Swarm: {} agents, {} patterns", stats.total_agents, stats.total_patterns);
```

### Tensor Compression

```rust
use ruvector_edge::compression::{TensorCodec, CompressionLevel};

// Create codec with quantization
let codec = TensorCodec::with_level(CompressionLevel::Quantized8);

// Compress tensor (75% size reduction)
let tensor: Vec<f32> = vec![0.1, 0.2, 0.3, /* ... */];
let compressed = codec.compress_tensor(&tensor)?;

// Decompress
let restored = codec.decompress_tensor(&compressed)?;
```

## Transport Options

| Transport | Use Case | Latency | Throughput |
|-----------|----------|---------|------------|
| WebSocket | Remote agents, cloud | Medium | High |
| SharedMemory | Local multi-process | Ultra-low | Very High |
| WASM | Browser-based agents | Low | Medium |

## Compression Levels

| Level | Ratio | Quality | Use Case |
|-------|-------|---------|----------|
| None | 1.0x | Lossless | Debugging |
| Fast | ~2x | Lossless | Default |
| High | ~3x | Lossless | Bandwidth-limited |
| Quantized8 | ~6x | Near-lossless | Pattern sync |
| Quantized4 | ~12x | Lossy | Archive |

## Agent Roles

| Role | Responsibilities |
|------|------------------|
| **Coordinator** | Manages swarm, distributes tasks |
| **Worker** | Executes tasks, learns patterns |
| **Scout** | Explores codebase, gathers context |
| **Specialist** | Domain expert (Rust, ML, etc.) |

## Protocol Messages

```
JOIN      → Agent joining swarm
LEAVE     → Agent leaving gracefully
PING/PONG → Heartbeat
SYNC_PATTERNS → Share learning state
REQUEST_PATTERNS → Request delta from peer
SYNC_MEMORIES → Share vector memories
BROADCAST_TASK → Distribute task to swarm
TASK_RESULT → Return task result
```

## Environment Variables

```bash
RUST_LOG=info          # Logging level
SWARM_COORDINATOR=ws://localhost:8080  # Default coordinator
SWARM_SYNC_INTERVAL=1000  # Sync interval in ms
```

## Integration with RuVector

This example integrates with the main RuVector ecosystem:

- **Learning Engine**: 9 RL algorithms for pattern learning
- **TensorCompress**: Adaptive compression based on access frequency
- **ONNX Embeddings**: Local semantic embeddings (all-MiniLM-L6-v2)
- **GNN/Attention**: Graph neural networks for code understanding

## Performance

| Metric | Value |
|--------|-------|
| Sync latency (SharedMemory) | < 1ms |
| Sync latency (WebSocket) | 5-50ms |
| Pattern merge throughput | 10K/sec |
| Compression ratio | 2-12x |
| Max agents per swarm | 1000+ |

## License

MIT
