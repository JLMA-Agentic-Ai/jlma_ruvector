//! Side-by-Side Comparison Tests for pgvector vs RuVector
//!
//! This module provides utilities for comparing RuVector results against pgvector
//! to validate 100% API compatibility. These tests can be run against both extensions
//! to ensure identical behavior.

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod comparison_tests {
    use pgrx::prelude::*;

    /// Test case structure for comparison
    pub struct ComparisonTestCase {
        pub name: &'static str,
        pub query: &'static str,
        pub expected_type: &'static str,
        pub tolerance: f64,
    }

    // ========================================================================
    // Distance Calculation Comparison Tests
    // ========================================================================

    /// Reference test cases from pgvector documentation
    const L2_DISTANCE_TESTS: &[(&str, &str, f64)] = &[
        ("[1,2,3]", "[3,2,1]", 2.828427),       // sqrt(8)
        ("[0,0,0]", "[3,4,0]", 5.0),            // 3-4-5 triangle
        ("[1,1,1]", "[2,2,2]", 1.732050808),    // sqrt(3)
        ("[-1,-1,-1]", "[1,1,1]", 3.464101615), // sqrt(12)
        ("[1,0]", "[0,1]", 1.414213562),        // sqrt(2)
    ];

    const COSINE_DISTANCE_TESTS: &[(&str, &str, f64)] = &[
        ("[1,2,3]", "[3,2,1]", 0.285714),       // 1 - 10/14
        ("[1,0,0]", "[1,0,0]", 0.0),            // same direction
        ("[1,0,0]", "[0,1,0]", 1.0),            // orthogonal
        ("[1,0,0]", "[-1,0,0]", 2.0),           // opposite
        ("[0.6,0.8]", "[0.8,0.6]", 0.04),       // unit vectors
    ];

    const INNER_PRODUCT_TESTS: &[(&str, &str, f64)] = &[
        ("[1,2,3]", "[4,5,6]", 32.0),           // 4+10+18
        ("[1,0]", "[0,1]", 0.0),                // orthogonal
        ("[1,1,1]", "[1,1,1]", 3.0),            // self
        ("[-1,2,-3]", "[4,-5,6]", -32.0),       // negative
    ];

    #[pg_test]
    fn test_l2_distance_comparison() {
        for (v1, v2, expected) in L2_DISTANCE_TESTS {
            let query = format!(
                "SELECT ruvector_l2_distance('{}'::ruvector, '{}'::ruvector)",
                v1, v2
            );

            let result = Spi::get_one::<f32>(&query).unwrap().unwrap();
            let diff = (result as f64 - expected).abs();

            assert!(
                diff < 0.001,
                "L2 distance test failed for {} <-> {}: expected {}, got {} (diff: {})",
                v1, v2, expected, result, diff
            );
        }
    }

    #[pg_test]
    fn test_cosine_distance_comparison() {
        for (v1, v2, expected) in COSINE_DISTANCE_TESTS {
            let query = format!(
                "SELECT ruvector_cosine_distance('{}'::ruvector, '{}'::ruvector)",
                v1, v2
            );

            let result = Spi::get_one::<f32>(&query).unwrap().unwrap();
            let diff = (result as f64 - expected).abs();

            assert!(
                diff < 0.01,
                "Cosine distance test failed for {} <=> {}: expected {}, got {} (diff: {})",
                v1, v2, expected, result, diff
            );
        }
    }

    #[pg_test]
    fn test_inner_product_comparison() {
        for (v1, v2, expected) in INNER_PRODUCT_TESTS {
            let query = format!(
                "SELECT ruvector_inner_product('{}'::ruvector, '{}'::ruvector)",
                v1, v2
            );

            let result = Spi::get_one::<f32>(&query).unwrap().unwrap();
            let diff = (result as f64 - expected).abs();

            assert!(
                diff < 0.001,
                "Inner product test failed for {} <#> {}: expected {}, got {} (diff: {})",
                v1, v2, expected, result, diff
            );
        }
    }

    // ========================================================================
    // Utility Function Comparison Tests
    // ========================================================================

    const DIMS_TESTS: &[(&str, i32)] = &[
        ("[1]", 1),
        ("[1,2,3]", 3),
        ("[1,2,3,4,5]", 5),
        ("[1,2,3,4,5,6,7,8,9,10]", 10),
    ];

    const NORM_TESTS: &[(&str, f64)] = &[
        ("[3,4]", 5.0),              // 3-4-5
        ("[0,0,0]", 0.0),            // zero
        ("[1,0,0]", 1.0),            // unit
        ("[0.6,0.8]", 1.0),          // unit
        ("[1,1,1,1]", 2.0),          // sqrt(4)
    ];

    #[pg_test]
    fn test_dims_comparison() {
        for (v, expected) in DIMS_TESTS {
            let query = format!("SELECT ruvector_dims('{}'::ruvector)", v);

            let result = Spi::get_one::<i32>(&query).unwrap().unwrap();

            assert_eq!(
                result, *expected,
                "Dims test failed for {}: expected {}, got {}",
                v, expected, result
            );
        }
    }

    #[pg_test]
    fn test_norm_comparison() {
        for (v, expected) in NORM_TESTS {
            let query = format!("SELECT ruvector_norm('{}'::ruvector)", v);

            let result = Spi::get_one::<f32>(&query).unwrap().unwrap();
            let diff = (result as f64 - expected).abs();

            assert!(
                diff < 0.001,
                "Norm test failed for {}: expected {}, got {} (diff: {})",
                v, expected, result, diff
            );
        }
    }

    // ========================================================================
    // Query Result Ordering Comparison Tests
    // ========================================================================

    #[pg_test]
    fn test_knn_ordering_matches_pgvector() {
        Spi::run("CREATE TABLE knn_test (id int, v ruvector(3))").unwrap();
        Spi::run("INSERT INTO knn_test VALUES
            (1, '[0,0,0]'),
            (2, '[1,0,0]'),
            (3, '[0,1,0]'),
            (4, '[1,1,0]'),
            (5, '[2,2,0]')
        ").unwrap();

        // Query for nearest neighbors to [0.9, 0.9, 0]
        let query = r#"
            SELECT id
            FROM knn_test
            ORDER BY ruvector_l2_distance(v, '[0.9,0.9,0]'::ruvector)
            LIMIT 3
        "#;

        let ids: Vec<i32> = Spi::connect(|client| {
            let mut results = Vec::new();
            let tup_table = client.select(query, None, None)?;
            for row in tup_table {
                if let Some(id) = row.get_by_name::<i32, _>("id")? {
                    results.push(id);
                }
            }
            Ok::<_, spi::Error>(results)
        }).unwrap();

        // Expected order: [1,1,0] (id=4) closest, then [1,0,0] or [0,1,0]
        assert_eq!(ids[0], 4, "First result should be id=4 (nearest to [0.9,0.9,0])");

        Spi::run("DROP TABLE knn_test").unwrap();
    }

    // ========================================================================
    // Aggregate Function Comparison Tests
    // ========================================================================

    #[pg_test]
    fn test_aggregate_avg_distance() {
        Spi::run("CREATE TABLE agg_test (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO agg_test VALUES
            ('[1,0,0]'),
            ('[0,1,0]'),
            ('[0,0,1]'),
            ('[1,1,1]')
        ").unwrap();

        // Average distance from origin
        let query = "SELECT AVG(ruvector_l2_distance(v, '[0,0,0]'::ruvector)) FROM agg_test";
        let result = Spi::get_one::<f64>(query).unwrap().unwrap();

        // Expected: (1 + 1 + 1 + sqrt(3)) / 4 = (3 + 1.732) / 4 = 1.183
        let expected = (3.0 + 3.0_f64.sqrt()) / 4.0;
        assert!(
            (result - expected).abs() < 0.01,
            "AVG distance: expected {}, got {}",
            expected, result
        );

        Spi::run("DROP TABLE agg_test").unwrap();
    }

    // ========================================================================
    // Cross-Type Compatibility Tests
    // ========================================================================

    #[pg_test]
    fn test_vector_text_roundtrip() {
        Spi::run("CREATE TABLE roundtrip (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO roundtrip VALUES ('[1.5,2.5,3.5]')").unwrap();

        // Read back and verify
        let text = Spi::get_one::<String>("SELECT v::text FROM roundtrip")
            .unwrap()
            .unwrap();

        // Parse values from text format
        let trimmed = text.trim_start_matches('[').trim_end_matches(']');
        let values: Vec<f32> = trimmed
            .split(',')
            .map(|s| s.trim().parse::<f32>().unwrap())
            .collect();

        assert!((values[0] - 1.5).abs() < 0.01);
        assert!((values[1] - 2.5).abs() < 0.01);
        assert!((values[2] - 3.5).abs() < 0.01);

        Spi::run("DROP TABLE roundtrip").unwrap();
    }

    // ========================================================================
    // Precision Comparison Tests
    // ========================================================================

    #[pg_test]
    fn test_precision_matches_pgvector() {
        // pgvector uses f32 internally, so precision should match
        let test_values = vec![
            0.123456789,
            0.987654321,
            0.000001,
            999999.999,
        ];

        for val in test_values {
            let v = format!("[{},0,0]", val);
            let query = format!("SELECT ruvector_norm('{}'::ruvector)", v);

            let result = Spi::get_one::<f32>(&query).unwrap().unwrap();

            // Norm of [x,0,0] = |x|
            let expected = (val as f32).abs();
            let diff = (result - expected).abs();

            // Allow for f32 precision (about 7 decimal digits)
            assert!(
                diff < expected * 1e-6 + 1e-7,
                "Precision mismatch for {}: expected {}, got {} (diff: {})",
                val, expected, result, diff
            );
        }
    }

    // ========================================================================
    // SIMD Consistency Tests
    // ========================================================================

    #[pg_test]
    fn test_simd_matches_scalar() {
        // Test various dimension sizes to catch SIMD edge cases
        let dim_sizes = vec![1, 3, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256];

        for dim in dim_sizes {
            let v1: String = (0..dim).map(|i| format!("{}", i as f32 * 0.1)).collect::<Vec<_>>().join(",");
            let v2: String = (0..dim).map(|i| format!("{}", (i + 1) as f32 * 0.1)).collect::<Vec<_>>().join(",");

            let query = format!(
                "SELECT ruvector_l2_distance('[{}]'::ruvector, '[{}]'::ruvector)",
                v1, v2
            );

            let result = Spi::get_one::<f32>(&query).unwrap().unwrap();

            assert!(
                result.is_finite() && result > 0.0,
                "SIMD consistency failed for dim {}: got {}",
                dim, result
            );

            // Verify against expected (each component differs by 0.1)
            // Distance = sqrt(dim * 0.1^2) = sqrt(dim) * 0.1
            let expected = (dim as f32).sqrt() * 0.1;
            let diff = (result - expected).abs();

            assert!(
                diff < 0.01,
                "SIMD result mismatch for dim {}: expected {}, got {} (diff: {})",
                dim, expected, result, diff
            );
        }
    }

    // ========================================================================
    // Bulk Operation Comparison Tests
    // ========================================================================

    #[pg_test]
    fn test_bulk_distance_calculation() {
        Spi::run("CREATE TABLE bulk_test (id serial, v ruvector(3))").unwrap();

        // Insert 100 vectors
        for i in 0..100 {
            Spi::run(&format!("INSERT INTO bulk_test (v) VALUES ('[{},{},{}]')", i, i, i)).unwrap();
        }

        // Calculate all distances from [50,50,50]
        let query = r#"
            SELECT
                SUM(ruvector_l2_distance(v, '[50,50,50]'::ruvector)) as total_dist,
                AVG(ruvector_l2_distance(v, '[50,50,50]'::ruvector)) as avg_dist,
                MIN(ruvector_l2_distance(v, '[50,50,50]'::ruvector)) as min_dist,
                MAX(ruvector_l2_distance(v, '[50,50,50]'::ruvector)) as max_dist
            FROM bulk_test
        "#;

        Spi::connect(|client| {
            let tup_table = client.select(query, None, None)?;
            for row in tup_table {
                let min_dist = row.get_by_name::<f64, _>("min_dist")?.unwrap();
                let max_dist = row.get_by_name::<f64, _>("max_dist")?.unwrap();

                // Min should be 0 (for [50,50,50])
                assert!(min_dist < 0.001, "Min distance should be ~0, got {}", min_dist);

                // Max should be from [0,0,0] or [99,99,99]
                // Distance to [0,0,0]: sqrt(50^2 + 50^2 + 50^2) = sqrt(7500) = 86.6
                // Distance to [99,99,99]: sqrt(49^2 * 3) = sqrt(7203) = 84.9
                assert!(max_dist > 80.0 && max_dist < 90.0, "Max distance should be ~86.6, got {}", max_dist);
            }
            Ok::<_, spi::Error>(())
        }).unwrap();

        Spi::run("DROP TABLE bulk_test").unwrap();
    }
}

// ============================================================================
// SQL Test File Generator
// ============================================================================

/// Generate SQL test files that can be run against both pgvector and ruvector
#[cfg(test)]
mod sql_test_generator {
    /// Generate types.sql content
    pub fn generate_types_sql() -> String {
        r#"-- pgvector Drop-In Compatibility Test: Types
-- Run against both pgvector and ruvector, compare results

-- Test 1: vector(n) type creation
CREATE TABLE test_vector_type (
    id serial,
    v vector(3)
);

-- Test 2: Insert and retrieve
INSERT INTO test_vector_type (v) VALUES
    ('[1,2,3]'),
    ('[4,5,6]'),
    ('[1.5,2.5,3.5]');

-- Test 3: Text format output
SELECT id, v::text FROM test_vector_type ORDER BY id;

-- Test 4: Dimension check
SELECT id, vector_dims(v) FROM test_vector_type ORDER BY id;

-- Cleanup
DROP TABLE test_vector_type;
"#.to_string()
    }

    /// Generate operators.sql content
    pub fn generate_operators_sql() -> String {
        r#"-- pgvector Drop-In Compatibility Test: Operators
-- Run against both pgvector and ruvector, compare results

-- Test 1: L2 distance operator <->
SELECT '[1,2,3]'::vector <-> '[3,2,1]'::vector AS l2_distance;
-- Expected: 2.828427

-- Test 2: Cosine distance operator <=>
SELECT '[1,2,3]'::vector <=> '[3,2,1]'::vector AS cosine_distance;
-- Expected: 0.285714

-- Test 3: Inner product operator <#>
SELECT '[1,2,3]'::vector <#> '[4,5,6]'::vector AS neg_inner_product;
-- Expected: -32

-- Test 4: Vector addition
SELECT '[1,2,3]'::vector + '[4,5,6]'::vector AS sum;

-- Test 5: Vector subtraction
SELECT '[5,7,9]'::vector - '[1,2,3]'::vector AS diff;

-- Test 6: Scalar multiplication
SELECT '[1,2,3]'::vector * 2 AS scaled;
"#.to_string()
    }

    /// Generate functions.sql content
    pub fn generate_functions_sql() -> String {
        r#"-- pgvector Drop-In Compatibility Test: Functions
-- Run against both pgvector and ruvector, compare results

-- Test 1: l2_distance function
SELECT l2_distance('[1,2,3]'::vector, '[4,5,6]'::vector);
-- Expected: 5.196152

-- Test 2: inner_product function
SELECT inner_product('[1,2,3]'::vector, '[4,5,6]'::vector);
-- Expected: 32

-- Test 3: cosine_distance function
SELECT cosine_distance('[1,2,3]'::vector, '[3,2,1]'::vector);
-- Expected: 0.285714

-- Test 4: l1_distance function
SELECT l1_distance('[1,2,3]'::vector, '[4,6,8]'::vector);
-- Expected: 12

-- Test 5: vector_dims function
SELECT vector_dims('[1,2,3,4,5]'::vector);
-- Expected: 5

-- Test 6: vector_norm function
SELECT vector_norm('[3,4]'::vector);
-- Expected: 5.0
"#.to_string()
    }

    /// Generate indexes.sql content
    pub fn generate_indexes_sql() -> String {
        r#"-- pgvector Drop-In Compatibility Test: Indexes
-- Run against both pgvector and ruvector, compare results

-- Setup test table
CREATE TABLE test_index (
    id serial PRIMARY KEY,
    embedding vector(3)
);

-- Insert test data
INSERT INTO test_index (embedding) VALUES
    ('[1,0,0]'),
    ('[0,1,0]'),
    ('[0,0,1]'),
    ('[1,1,0]'),
    ('[1,0,1]'),
    ('[0,1,1]'),
    ('[1,1,1]');

-- Test 1: HNSW index creation (L2)
CREATE INDEX idx_hnsw_l2 ON test_index
USING hnsw (embedding vector_l2_ops)
WITH (m = 16, ef_construction = 64);

-- Test 2: Query with HNSW index
SET hnsw.ef_search = 40;
SELECT id, embedding <-> '[0.9,0.9,0]' AS distance
FROM test_index
ORDER BY embedding <-> '[0.9,0.9,0]'
LIMIT 3;

-- Test 3: Drop and recreate with cosine
DROP INDEX idx_hnsw_l2;
CREATE INDEX idx_hnsw_cosine ON test_index
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

-- Test 4: Query with cosine index
SELECT id, embedding <=> '[0.9,0.9,0]' AS distance
FROM test_index
ORDER BY embedding <=> '[0.9,0.9,0]'
LIMIT 3;

-- Cleanup
DROP TABLE test_index;
"#.to_string()
    }

    /// Generate queries.sql content
    pub fn generate_queries_sql() -> String {
        r#"-- pgvector Drop-In Compatibility Test: Query Patterns
-- Run against both pgvector and ruvector, compare results

-- Setup
CREATE TABLE items (
    id serial PRIMARY KEY,
    category text,
    embedding vector(3)
);

INSERT INTO items (category, embedding) VALUES
    ('A', '[1,0,0]'),
    ('A', '[1.1,0,0]'),
    ('B', '[0,1,0]'),
    ('B', '[0,1.1,0]'),
    ('C', '[0,0,1]');

-- Test 1: Basic KNN
SELECT id, embedding <-> '[1,0,0]' AS distance
FROM items
ORDER BY embedding <-> '[1,0,0]'
LIMIT 3;

-- Test 2: KNN with filter
SELECT id, embedding <-> '[0.5,0.5,0]' AS distance
FROM items
WHERE category = 'A'
ORDER BY embedding <-> '[0.5,0.5,0]'
LIMIT 2;

-- Test 3: Aggregate with distance
SELECT category,
       MIN(embedding <-> '[0.5,0.5,0.5]') AS min_dist,
       AVG(embedding <-> '[0.5,0.5,0.5]') AS avg_dist
FROM items
GROUP BY category
ORDER BY min_dist;

-- Test 4: CTE with KNN
WITH nearest AS (
    SELECT id, category, embedding <-> '[0,0,0]' AS dist
    FROM items
    ORDER BY dist
    LIMIT 3
)
SELECT * FROM nearest;

-- Test 5: Distance threshold
SELECT id, category
FROM items
WHERE embedding <-> '[0,0,0]' < 1.5;

-- Cleanup
DROP TABLE items;
"#.to_string()
    }
}
