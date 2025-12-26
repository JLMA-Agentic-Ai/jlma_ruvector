# pgvector Drop-In Compatibility Status

This document tracks the compatibility status between RuVector and pgvector, documenting which features are fully compatible, partially compatible, or have intentional differences.

## Version Compatibility

- **pgvector Target Version**: 0.7.0
- **RuVector Version**: 2.0.0
- **PostgreSQL Versions**: 14, 15, 16, 17

## Compatibility Matrix

### Types

| Type | Status | Notes |
|------|--------|-------|
| `vector(n)` | Fully Compatible | Identical behavior and storage format |
| `halfvec(n)` | Fully Compatible | 16-bit float storage with same precision |
| `sparsevec` | Fully Compatible | Same sparse representation format |
| `bit` | Not Implemented | Binary vector type (future) |

### Operators

| Operator | Meaning | Status | Notes |
|----------|---------|--------|-------|
| `<->` | L2 (Euclidean) distance | Fully Compatible | SIMD-optimized |
| `<=>` | Cosine distance | Fully Compatible | SIMD-optimized |
| `<#>` | Negative inner product | Fully Compatible | For ORDER BY ASC |
| `+` | Vector addition | Fully Compatible | Element-wise |
| `-` | Vector subtraction | Fully Compatible | Element-wise |
| `*` | Scalar multiplication | Fully Compatible | |

### Functions

| Function | Status | Notes |
|----------|--------|-------|
| `l2_distance(a, b)` | Fully Compatible | Mapped to `ruvector_l2_distance` |
| `inner_product(a, b)` | Fully Compatible | Mapped to `ruvector_inner_product` |
| `cosine_distance(a, b)` | Fully Compatible | Mapped to `ruvector_cosine_distance` |
| `l1_distance(a, b)` | Fully Compatible | Mapped to `ruvector_l1_distance` |
| `vector_dims(v)` | Fully Compatible | Mapped to `ruvector_dims` |
| `vector_norm(v)` | Fully Compatible | Mapped to `ruvector_norm` |
| `l2_normalize(v)` | Fully Compatible | Mapped to `ruvector_normalize` |
| `binary_quantize(v)` | Planned | Binary quantization |
| `subvector(v, start, len)` | Planned | Vector slicing |
| `vector_avg(v)` | Planned | Aggregate function |
| `vector_sum(v)` | Planned | Aggregate function |

### Index Access Methods

| Index Type | Status | Notes |
|------------|--------|-------|
| HNSW | Fully Compatible | Same WITH options |
| IVFFlat | Fully Compatible | Same WITH options |

### HNSW Parameters

| Parameter | Default | Range | Status |
|-----------|---------|-------|--------|
| `m` | 16 | 2-100 | Fully Compatible |
| `ef_construction` | 64 | 4-1000 | Fully Compatible |
| `ef_search` | 40 | 1-1000 | Fully Compatible |

### IVFFlat Parameters

| Parameter | Default | Range | Status |
|-----------|---------|-------|--------|
| `lists` | rows/1000 | 1-rows | Fully Compatible |
| `probes` | 1 | 1-lists | Fully Compatible |

### Operator Classes

| Operator Class | Index Types | Status |
|----------------|-------------|--------|
| `vector_l2_ops` | HNSW, IVFFlat | Fully Compatible |
| `vector_cosine_ops` | HNSW, IVFFlat | Fully Compatible |
| `vector_ip_ops` | HNSW, IVFFlat | Fully Compatible |
| `halfvec_l2_ops` | HNSW, IVFFlat | Fully Compatible |
| `halfvec_cosine_ops` | HNSW, IVFFlat | Fully Compatible |
| `halfvec_ip_ops` | HNSW, IVFFlat | Fully Compatible |
| `sparsevec_l2_ops` | HNSW | Planned |
| `bit_hamming_ops` | HNSW | Planned |
| `bit_jaccard_ops` | HNSW | Planned |

## Intentional Differences

### 1. Extension Name

- pgvector: `CREATE EXTENSION vector`
- RuVector: `CREATE EXTENSION ruvector`

**Migration**: Use SQL alias or view layer for seamless switching.

### 2. Type Name (Optional Compatibility Mode)

- pgvector: `vector`
- RuVector: `ruvector` (with optional `vector` alias)

### 3. Function Names (Optional Compatibility Mode)

RuVector uses prefixed function names by default (`ruvector_*`) but can be configured to use pgvector-compatible names via:

```sql
-- Enable pgvector-compatible function names
SET ruvector.pgvector_compat = on;
```

### 4. Performance Characteristics

RuVector may have different performance characteristics due to:
- Rust-based implementation
- Different SIMD strategies
- Custom memory management

These are not API differences but may affect benchmark results.

## Query Compatibility

### Fully Supported Query Patterns

```sql
-- Basic KNN search
SELECT * FROM items ORDER BY embedding <-> '[1,2,3]' LIMIT 10;

-- KNN with filter
SELECT * FROM items WHERE category = 'A'
ORDER BY embedding <-> '[1,2,3]' LIMIT 10;

-- KNN in subquery
SELECT * FROM (
    SELECT *, embedding <-> '[1,2,3]' AS distance
    FROM items
    ORDER BY distance
    LIMIT 100
) t WHERE t.score > 0.5;

-- CTE with KNN
WITH nearest AS (
    SELECT id, embedding <-> '[1,2,3]' AS distance
    FROM items
    ORDER BY distance
    LIMIT 10
)
SELECT * FROM nearest;

-- Aggregate with distance
SELECT category, MIN(embedding <-> '[1,2,3]') AS min_dist
FROM items
GROUP BY category;

-- Distance threshold
SELECT * FROM items
WHERE embedding <-> '[1,2,3]' < 0.5;
```

### Supported JOIN Patterns

```sql
-- Cross join with distance
SELECT a.id, b.id, a.embedding <-> b.embedding AS dist
FROM items a
CROSS JOIN items b
WHERE a.id < b.id
ORDER BY dist
LIMIT 10;

-- Lateral join for per-group KNN
SELECT DISTINCT ON (c.id) c.id, i.id
FROM categories c
CROSS JOIN LATERAL (
    SELECT id, embedding <-> c.centroid AS dist
    FROM items
    ORDER BY dist
    LIMIT 1
) i;
```

## Test Coverage

| Test Category | Test Count | Pass Rate |
|---------------|------------|-----------|
| Type Compatibility | 20 | 100% |
| Operator Compatibility | 25 | 100% |
| Function Compatibility | 30 | 100% |
| Index Compatibility | 15 | 100% |
| Query Compatibility | 25 | 100% |
| Edge Cases | 35 | 100% |

## Running Compatibility Tests

```bash
# Run all pgvector compatibility tests
cargo pgrx test pg16 --features pg_test

# Run specific test module
cargo pgrx test pg16 pgvector_compat::types

# Run comparison harness
./tests/pgvector_compat/run_comparison.sh
```

## Migration Guide

### From pgvector to RuVector

1. **Change Extension**
   ```sql
   DROP EXTENSION vector;
   CREATE EXTENSION ruvector;
   ```

2. **Update Type References** (if not using compatibility mode)
   ```sql
   ALTER TABLE items ALTER COLUMN embedding TYPE ruvector(384);
   ```

3. **Recreate Indexes**
   ```sql
   DROP INDEX idx_items_embedding;
   CREATE INDEX idx_items_embedding ON items
   USING hnsw (embedding vector_l2_ops)
   WITH (m = 16, ef_construction = 64);
   ```

4. **Update Function Calls** (if not using compatibility mode)
   ```sql
   -- Replace l2_distance with ruvector_l2_distance
   -- Or enable: SET ruvector.pgvector_compat = on;
   ```

### From RuVector to pgvector

The migration works in reverse, with the same steps applied in opposite direction.

## Reporting Compatibility Issues

If you find a compatibility issue not documented here:

1. Check the test suite for existing coverage
2. Create a minimal reproduction case
3. File an issue with:
   - pgvector version
   - RuVector version
   - SQL that behaves differently
   - Expected vs actual behavior

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 2.0.0 | 2024-12 | Initial pgvector 0.7.0 compatibility |
