//! Edge Case and Error Handling Tests for pgvector Drop-In Replacement
//!
//! Validates that RuVector handles edge cases correctly and matches pgvector's behavior:
//! - Empty vectors
//! - Zero vectors
//! - Very small/large values
//! - Numerical precision limits
//! - Error conditions
//! - Boundary values

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod pgvector_edge_case_tests {
    use pgrx::prelude::*;
    use ruvector_postgres::types::RuVector;

    const EPSILON: f32 = 1e-5;

    // ========================================================================
    // Zero Vector Edge Cases
    // ========================================================================

    #[pg_test]
    fn test_zero_vector_creation() {
        Spi::run("CREATE TABLE test_zero (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_zero VALUES ('[0,0,0]')").unwrap();

        let norm = Spi::get_one::<f32>(
            "SELECT ruvector_norm(v) FROM test_zero"
        ).unwrap().unwrap();

        assert!(norm.abs() < EPSILON, "Zero vector norm should be 0");

        Spi::run("DROP TABLE test_zero").unwrap();
    }

    #[pg_test]
    fn test_zero_vector_l2_distance() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_l2_distance('[0,0,0]'::ruvector, '[3,4,0]'::ruvector)"
        ).unwrap().unwrap();

        // L2 from origin to [3,4,0] = 5
        assert!((result - 5.0).abs() < EPSILON);
    }

    #[pg_test]
    fn test_zero_vector_cosine_distance() {
        // Cosine with zero vector is undefined - check how it's handled
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_cosine_distance('[0,0,0]'::ruvector, '[1,1,1]'::ruvector)"
        ).unwrap().unwrap();

        // Should return 1.0 (maximum distance) for zero vectors
        assert!((result - 1.0).abs() < 0.01 || result == 1.0,
            "Zero vector cosine should be 1.0 (undefined), got {}", result);
    }

    #[pg_test]
    fn test_zero_vector_normalization() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_norm(ruvector_normalize('[0,0,0]'::ruvector))"
        ).unwrap().unwrap();

        // Normalizing zero vector should return zero vector (norm = 0)
        assert!(result.abs() < 0.01 || result == 0.0);
    }

    // ========================================================================
    // Very Small Values
    // ========================================================================

    #[pg_test]
    fn test_very_small_values() {
        let small = 1e-30f32;
        let v1 = format!("[{},{},{}]", small, small, small);
        let v2 = format!("[{},{},{}]", small * 2.0, small * 2.0, small * 2.0);

        let query = format!(
            "SELECT ruvector_l2_distance('{}'::ruvector, '{}'::ruvector)",
            v1, v2
        );

        let result = Spi::get_one::<f32>(&query).unwrap().unwrap();
        assert!(result.is_finite(), "Should handle very small values");
    }

    #[pg_test]
    fn test_denormalized_floats() {
        // Test with denormalized (subnormal) floats
        let denorm = f32::MIN_POSITIVE / 2.0;
        let v = format!("[{},0,0]", denorm);

        let query = format!("SELECT ruvector_dims('{}'::ruvector)", v);
        let result = Spi::get_one::<i32>(&query).unwrap().unwrap();
        assert_eq!(result, 3);
    }

    // ========================================================================
    // Very Large Values
    // ========================================================================

    #[pg_test]
    fn test_very_large_values() {
        let large = 1e30f32;
        let v1 = format!("[{},{},{}]", large, large, large);
        let v2 = format!("[{},{},{}]", large, large, large);

        let query = format!(
            "SELECT ruvector_l2_distance('{}'::ruvector, '{}'::ruvector)",
            v1, v2
        );

        let result = Spi::get_one::<f32>(&query).unwrap().unwrap();
        assert!(result.abs() < EPSILON, "Identical large vectors should have distance 0");
    }

    #[pg_test]
    fn test_max_float_values() {
        // Near f32::MAX values
        let max_safe = 1e38f32;
        let v = format!("[{},0,0]", max_safe);

        let query = format!("SELECT ruvector_norm('{}'::ruvector)", v);
        let result = Spi::get_one::<f32>(&query).unwrap().unwrap();

        assert!(result.is_finite(), "Should handle near-max float values");
    }

    // ========================================================================
    // Negative Values
    // ========================================================================

    #[pg_test]
    fn test_negative_values() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_l2_distance('[-1,-2,-3]'::ruvector, '[1,2,3]'::ruvector)"
        ).unwrap().unwrap();

        // sqrt((2)^2 + (4)^2 + (6)^2) = sqrt(4+16+36) = sqrt(56) = 7.48
        assert!((result - 7.48).abs() < 0.1);
    }

    #[pg_test]
    fn test_mixed_sign_values() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_inner_product('[-1,2,-3]'::ruvector, '[4,-5,6]'::ruvector)"
        ).unwrap().unwrap();

        // IP = -1*4 + 2*-5 + -3*6 = -4 - 10 - 18 = -32
        assert!((result - (-32.0)).abs() < EPSILON);
    }

    // ========================================================================
    // Single Dimension Edge Cases
    // ========================================================================

    #[pg_test]
    fn test_single_dimension_l2() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_l2_distance('[10]'::ruvector, '[7]'::ruvector)"
        ).unwrap().unwrap();

        assert!((result - 3.0).abs() < EPSILON);
    }

    #[pg_test]
    fn test_single_dimension_cosine() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_cosine_distance('[1]'::ruvector, '[-1]'::ruvector)"
        ).unwrap().unwrap();

        // Opposite directions = distance 2
        assert!((result - 2.0).abs() < EPSILON);
    }

    // ========================================================================
    // High Dimension Edge Cases
    // ========================================================================

    #[pg_test]
    fn test_high_dimension_stability() {
        // High dimensional vectors can suffer from numerical instability
        let dim = 1000;
        let values1: String = (0..dim).map(|i| format!("{}", (i as f32) * 0.001)).collect::<Vec<_>>().join(",");
        let values2: String = (0..dim).map(|i| format!("{}", (i as f32 + 0.5) * 0.001)).collect::<Vec<_>>().join(",");

        let query = format!(
            "SELECT ruvector_cosine_distance('[{}]'::ruvector, '[{}]'::ruvector)",
            values1, values2
        );

        let result = Spi::get_one::<f32>(&query).unwrap().unwrap();
        assert!(result >= 0.0 && result <= 2.0, "Cosine distance should be in [0,2]");
    }

    #[pg_test]
    fn test_max_dimensions() {
        // Test with maximum supported dimensions (2000 for test speed)
        let dim = 2000;
        let values: String = (0..dim).map(|i| format!("{}", i as f32 * 0.001)).collect::<Vec<_>>().join(",");

        let query = format!("SELECT ruvector_dims('[{}]'::ruvector)", values);
        let result = Spi::get_one::<i32>(&query).unwrap().unwrap();

        assert_eq!(result, dim as i32);
    }

    // ========================================================================
    // Numerical Precision Edge Cases
    // ========================================================================

    #[pg_test]
    fn test_catastrophic_cancellation() {
        // Test case that could trigger catastrophic cancellation
        let a = "[1000000.0,1000000.0,1000000.0]";
        let b = "[1000000.1,1000000.1,1000000.1]";

        let query = format!(
            "SELECT ruvector_l2_distance('{}'::ruvector, '{}'::ruvector)", a, b
        );

        let result = Spi::get_one::<f32>(&query).unwrap().unwrap();

        // sqrt(0.1^2 + 0.1^2 + 0.1^2) = sqrt(0.03) = 0.173
        assert!((result - 0.173).abs() < 0.01, "Should handle near values correctly: {}", result);
    }

    #[pg_test]
    fn test_nearly_identical_vectors() {
        let a = "[1.0000001,2.0000001,3.0000001]";
        let b = "[1.0000002,2.0000002,3.0000002]";

        let query = format!(
            "SELECT ruvector_l2_distance('{}'::ruvector, '{}'::ruvector)", a, b
        );

        let result = Spi::get_one::<f32>(&query).unwrap().unwrap();
        assert!(result >= 0.0 && result < 1e-5, "Nearly identical vectors should have tiny distance");
    }

    // ========================================================================
    // Sparse Vector Edge Cases
    // ========================================================================

    #[pg_test]
    fn test_sparse_all_zeros() {
        use ruvector_postgres::types::SparseVec;

        let sparse = SparseVec::zeros(1000);
        assert_eq!(sparse.dimensions(), 1000);
        assert_eq!(sparse.nnz(), 0);
        assert_eq!(sparse.norm(), 0.0);
    }

    #[pg_test]
    fn test_sparse_single_nonzero() {
        use ruvector_postgres::types::SparseVec;

        let sparse = SparseVec::from_pairs(1000, &[(500, 3.0)]);
        assert_eq!(sparse.dimensions(), 1000);
        assert_eq!(sparse.nnz(), 1);
        assert!((sparse.norm() - 3.0).abs() < EPSILON);
    }

    #[pg_test]
    fn test_sparse_dot_product() {
        use ruvector_postgres::types::SparseVec;

        let a = SparseVec::from_pairs(100, &[(0, 1.0), (50, 2.0), (99, 3.0)]);
        let b = SparseVec::from_pairs(100, &[(0, 4.0), (50, 5.0), (99, 6.0)]);

        let dot = a.dot(&b);
        // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        assert!((dot - 32.0).abs() < EPSILON);
    }

    // ========================================================================
    // Error Condition Tests
    // ========================================================================

    #[pg_test]
    #[should_panic(expected = "dimension")]
    fn test_dimension_mismatch_error() {
        Spi::run("SELECT ruvector_l2_distance('[1,2,3]'::ruvector, '[1,2]'::ruvector)").unwrap();
    }

    #[pg_test]
    #[should_panic]
    fn test_invalid_bracket_format() {
        Spi::run("SELECT '1,2,3'::ruvector").unwrap();
    }

    #[pg_test]
    #[should_panic]
    fn test_nan_value_rejected() {
        Spi::run("SELECT '[1,NaN,3]'::ruvector").unwrap();
    }

    #[pg_test]
    #[should_panic]
    fn test_infinity_rejected() {
        Spi::run("SELECT '[1,Inf,3]'::ruvector").unwrap();
    }

    #[pg_test]
    #[should_panic]
    fn test_negative_infinity_rejected() {
        Spi::run("SELECT '[1,-Infinity,3]'::ruvector").unwrap();
    }

    // ========================================================================
    // Boundary Value Tests
    // ========================================================================

    #[pg_test]
    fn test_boundary_dimension_1() {
        let v = RuVector::from_slice(&[42.0]);
        assert_eq!(v.dimensions(), 1);
    }

    #[pg_test]
    fn test_boundary_dimension_16000() {
        // Maximum dimensions test
        let data: Vec<f32> = (0..16000).map(|i| i as f32 * 0.001).collect();
        let v = RuVector::from_slice(&data);
        assert_eq!(v.dimensions(), 16000);
    }

    #[pg_test]
    fn test_boundary_value_zero() {
        let v = RuVector::from_slice(&[0.0, 0.0, 0.0]);
        assert_eq!(v.norm(), 0.0);
    }

    #[pg_test]
    fn test_boundary_value_negative_zero() {
        let v = RuVector::from_slice(&[-0.0, -0.0, -0.0]);
        assert_eq!(v.norm(), 0.0);
    }

    // ========================================================================
    // Memory and Performance Edge Cases
    // ========================================================================

    #[pg_test]
    fn test_repeated_operations() {
        // Test memory stability under repeated operations
        Spi::run("CREATE TABLE test_repeat (v ruvector(3))").unwrap();

        for i in 0..100 {
            Spi::run(&format!("INSERT INTO test_repeat VALUES ('[{},{},{}]')", i, i, i)).unwrap();
        }

        for i in 0..10 {
            let query = format!(
                "SELECT SUM(ruvector_l2_distance(v, '[{},{},{}]'::ruvector)) FROM test_repeat",
                i * 10, i * 10, i * 10
            );
            let result = Spi::get_one::<f64>(&query).unwrap().unwrap();
            assert!(result.is_finite());
        }

        Spi::run("DROP TABLE test_repeat").unwrap();
    }

    #[pg_test]
    fn test_bulk_insert() {
        Spi::run("CREATE TABLE test_bulk (v ruvector(64))").unwrap();

        // Bulk insert 1000 vectors
        for i in 0..1000 {
            let values: String = (0..64)
                .map(|j| format!("{}", ((i * 64 + j) % 1000) as f32 * 0.001))
                .collect::<Vec<_>>()
                .join(",");
            Spi::run(&format!("INSERT INTO test_bulk VALUES ('[{}]')", values)).unwrap();
        }

        let count = Spi::get_one::<i64>("SELECT COUNT(*) FROM test_bulk").unwrap().unwrap();
        assert_eq!(count, 1000);

        Spi::run("DROP TABLE test_bulk").unwrap();
    }

    // ========================================================================
    // Special Float Value Tests
    // ========================================================================

    #[pg_test]
    fn test_positive_negative_zero_equivalence() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_l2_distance('[0.0,0.0]'::ruvector, '[-0.0,-0.0]'::ruvector)"
        ).unwrap().unwrap();

        assert!(result == 0.0, "Positive and negative zero should be equivalent");
    }

    #[pg_test]
    fn test_subnormal_handling() {
        // Subnormal (denormalized) floats
        let tiny = format!("{}", f32::MIN_POSITIVE / 2.0);
        let v = format!("[{},0,0]", tiny);

        let query = format!("SELECT ruvector_norm('{}'::ruvector)", v);
        let result = Spi::get_one::<f32>(&query).unwrap().unwrap();

        assert!(result.is_finite() && result >= 0.0);
    }

    // ========================================================================
    // Unicode and Special Character Tests (Text Format)
    // ========================================================================

    #[pg_test]
    fn test_whitespace_handling() {
        // Extra whitespace should be handled
        let vectors = vec![
            "[1,2,3]",
            "[ 1, 2, 3 ]",
            "[  1  ,  2  ,  3  ]",
            "[1, 2, 3]",
        ];

        for v in vectors {
            let query = format!("SELECT ruvector_dims('{}'::ruvector)", v);
            let result = Spi::get_one::<i32>(&query).unwrap().unwrap();
            assert_eq!(result, 3, "Failed for: {}", v);
        }
    }

    #[pg_test]
    fn test_scientific_notation() {
        // Scientific notation should work
        let result = Spi::get_one::<i32>(
            "SELECT ruvector_dims('[1e-5,2.5e3,3E+2]'::ruvector)"
        ).unwrap().unwrap();
        assert_eq!(result, 3);
    }
}
