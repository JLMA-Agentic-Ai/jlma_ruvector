//! Function Compatibility Tests for pgvector Drop-In Replacement
//!
//! Validates that RuVector's SQL functions match pgvector's:
//! - l2_distance(a, b)
//! - inner_product(a, b)
//! - cosine_distance(a, b)
//! - l1_distance(a, b)
//! - vector_dims(v)
//! - vector_norm(v)

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod pgvector_function_compat_tests {
    use pgrx::prelude::*;
    use ruvector_postgres::types::RuVector;

    const EPSILON: f64 = 1e-4;

    // ========================================================================
    // l2_distance(a, b) Function
    // ========================================================================

    #[pg_test]
    fn test_l2_distance_function() {
        // pgvector: SELECT l2_distance('[1,2,3]'::vector, '[3,2,1]'::vector);
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_l2_distance('[1,2,3]'::ruvector, '[3,2,1]'::ruvector)"
        ).unwrap().unwrap();

        let expected = 2.828427f32;
        assert!(
            (result - expected).abs() < 0.001,
            "l2_distance: expected {}, got {}",
            expected,
            result
        );
    }

    #[pg_test]
    fn test_l2_distance_zero() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_l2_distance('[1,2,3]'::ruvector, '[1,2,3]'::ruvector)"
        ).unwrap().unwrap();

        assert!(result.abs() < 0.0001, "Same vectors should have distance 0");
    }

    #[pg_test]
    fn test_l2_distance_high_dimensional() {
        // Test with 128-dimensional vectors
        let v1: String = (0..128).map(|i| format!("{}", i as f32 * 0.1)).collect::<Vec<_>>().join(",");
        let v2: String = (0..128).map(|i| format!("{}", (i + 1) as f32 * 0.1)).collect::<Vec<_>>().join(",");

        let query = format!(
            "SELECT ruvector_l2_distance('[{}]'::ruvector, '[{}]'::ruvector)",
            v1, v2
        );

        let result = Spi::get_one::<f32>(&query).unwrap().unwrap();
        assert!(result > 0.0 && result.is_finite(), "Should compute valid distance");
    }

    // ========================================================================
    // inner_product(a, b) Function
    // ========================================================================

    #[pg_test]
    fn test_inner_product_function() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_inner_product('[1,2,3]'::ruvector, '[4,5,6]'::ruvector)"
        ).unwrap().unwrap();

        // IP = 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        assert!(
            (result - 32.0).abs() < 0.001,
            "inner_product: expected 32, got {}",
            result
        );
    }

    #[pg_test]
    fn test_inner_product_orthogonal() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_inner_product('[1,0]'::ruvector, '[0,1]'::ruvector)"
        ).unwrap().unwrap();

        assert!(result.abs() < 0.0001, "Orthogonal vectors should have IP 0");
    }

    #[pg_test]
    fn test_inner_product_unit_vectors() {
        // Unit vectors: IP = cosine of angle
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_inner_product('[0.6,0.8]'::ruvector, '[0.8,0.6]'::ruvector)"
        ).unwrap().unwrap();

        // IP = 0.6*0.8 + 0.8*0.6 = 0.96
        assert!(
            (result - 0.96).abs() < 0.001,
            "Unit vector IP: expected 0.96, got {}",
            result
        );
    }

    // ========================================================================
    // cosine_distance(a, b) Function
    // ========================================================================

    #[pg_test]
    fn test_cosine_distance_function() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_cosine_distance('[1,2,3]'::ruvector, '[3,2,1]'::ruvector)"
        ).unwrap().unwrap();

        // cosine = 10/14 = 0.714, distance = 1 - 0.714 = 0.286
        let expected = 0.2857f32;
        assert!(
            (result - expected).abs() < 0.01,
            "cosine_distance: expected ~{}, got {}",
            expected,
            result
        );
    }

    #[pg_test]
    fn test_cosine_distance_identical() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_cosine_distance('[1,2,3]'::ruvector, '[1,2,3]'::ruvector)"
        ).unwrap().unwrap();

        assert!(result.abs() < 0.0001, "Identical vectors should have cosine distance 0");
    }

    #[pg_test]
    fn test_cosine_distance_opposite() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_cosine_distance('[1,0,0]'::ruvector, '[-1,0,0]'::ruvector)"
        ).unwrap().unwrap();

        // Opposite directions: distance = 2
        assert!(
            (result - 2.0).abs() < 0.001,
            "Opposite vectors should have cosine distance 2, got {}",
            result
        );
    }

    // ========================================================================
    // l1_distance(a, b) Function (Manhattan Distance)
    // ========================================================================

    #[pg_test]
    fn test_l1_distance_function() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_l1_distance('[1,2,3]'::ruvector, '[4,6,8]'::ruvector)"
        ).unwrap().unwrap();

        // L1 = |1-4| + |2-6| + |3-8| = 3 + 4 + 5 = 12
        assert!(
            (result - 12.0).abs() < 0.001,
            "l1_distance: expected 12, got {}",
            result
        );
    }

    #[pg_test]
    fn test_l1_distance_negative() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_l1_distance('[-1,-2,-3]'::ruvector, '[1,2,3]'::ruvector)"
        ).unwrap().unwrap();

        // L1 = 2 + 4 + 6 = 12
        assert!(
            (result - 12.0).abs() < 0.001,
            "l1_distance with negatives: expected 12, got {}",
            result
        );
    }

    // ========================================================================
    // vector_dims(v) Function
    // ========================================================================

    #[pg_test]
    fn test_vector_dims_function() {
        let result = Spi::get_one::<i32>(
            "SELECT ruvector_dims('[1,2,3]'::ruvector)"
        ).unwrap().unwrap();

        assert_eq!(result, 3, "vector_dims: expected 3, got {}", result);
    }

    #[pg_test]
    fn test_vector_dims_single() {
        let result = Spi::get_one::<i32>(
            "SELECT ruvector_dims('[42]'::ruvector)"
        ).unwrap().unwrap();

        assert_eq!(result, 1);
    }

    #[pg_test]
    fn test_vector_dims_high() {
        let values: String = (0..1000).map(|i| format!("{}", i)).collect::<Vec<_>>().join(",");
        let query = format!("SELECT ruvector_dims('[{}]'::ruvector)", values);

        let result = Spi::get_one::<i32>(&query).unwrap().unwrap();
        assert_eq!(result, 1000);
    }

    // ========================================================================
    // vector_norm(v) Function
    // ========================================================================

    #[pg_test]
    fn test_vector_norm_function() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_norm('[3,4]'::ruvector)"
        ).unwrap().unwrap();

        // ||(3,4)|| = sqrt(9 + 16) = 5
        assert!(
            (result - 5.0).abs() < 0.001,
            "vector_norm: expected 5, got {}",
            result
        );
    }

    #[pg_test]
    fn test_vector_norm_unit() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_norm('[0.6,0.8]'::ruvector)"
        ).unwrap().unwrap();

        // ||(0.6,0.8)|| = 1 (unit vector)
        assert!(
            (result - 1.0).abs() < 0.001,
            "Unit vector norm: expected 1, got {}",
            result
        );
    }

    #[pg_test]
    fn test_vector_norm_zero() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_norm('[0,0,0]'::ruvector)"
        ).unwrap().unwrap();

        assert!(result.abs() < 0.0001, "Zero vector norm: expected 0, got {}", result);
    }

    // ========================================================================
    // vector_normalize(v) Function
    // ========================================================================

    #[pg_test]
    fn test_vector_normalize_function() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_norm(ruvector_normalize('[3,4]'::ruvector))"
        ).unwrap().unwrap();

        // Normalized vector should have norm 1
        assert!(
            (result - 1.0).abs() < 0.001,
            "Normalized vector norm: expected 1, got {}",
            result
        );
    }

    #[pg_test]
    fn test_vector_normalize_preserves_direction() {
        // After normalization, angle should be preserved
        let query = r#"
            SELECT ruvector_cosine_distance(
                ruvector_normalize('[3,4]'::ruvector),
                '[3,4]'::ruvector
            )
        "#;

        let result = Spi::get_one::<f32>(query).unwrap().unwrap();

        // Same direction = distance 0
        assert!(result.abs() < 0.001, "Normalization should preserve direction");
    }

    // ========================================================================
    // Vector Arithmetic Functions
    // ========================================================================

    #[pg_test]
    fn test_vector_add_function() {
        let result = Spi::get_one::<String>(
            "SELECT ruvector_add('[1,2,3]'::ruvector, '[4,5,6]'::ruvector)::text"
        ).unwrap().unwrap();

        // Should contain 5, 7, 9
        assert!(result.contains('5') && result.contains('7') && result.contains('9'));
    }

    #[pg_test]
    fn test_vector_sub_function() {
        let result = Spi::get_one::<String>(
            "SELECT ruvector_sub('[5,7,9]'::ruvector, '[1,2,3]'::ruvector)::text"
        ).unwrap().unwrap();

        // Should contain 4, 5, 6
        assert!(result.contains('4') && result.contains('5') && result.contains('6'));
    }

    #[pg_test]
    fn test_vector_mul_scalar_function() {
        let result = Spi::get_one::<String>(
            "SELECT ruvector_mul_scalar('[1,2,3]'::ruvector, 2)::text"
        ).unwrap().unwrap();

        // Should contain 2, 4, 6
        assert!(result.contains('2') && result.contains('4') && result.contains('6'));
    }

    // ========================================================================
    // Function Composition Tests
    // ========================================================================

    #[pg_test]
    fn test_function_composition() {
        // Test: distance between normalized vectors
        let query = r#"
            SELECT ruvector_l2_distance(
                ruvector_normalize('[3,4]'::ruvector),
                ruvector_normalize('[4,3]'::ruvector)
            )
        "#;

        let result = Spi::get_one::<f32>(query).unwrap().unwrap();
        assert!(result > 0.0 && result < 2.0, "Valid distance between unit vectors");
    }

    #[pg_test]
    fn test_aggregation_with_distance() {
        Spi::run("CREATE TABLE test_agg (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_agg VALUES ('[1,0,0]'), ('[0,1,0]'), ('[0,0,1]')").unwrap();

        let query = r#"
            SELECT AVG(ruvector_l2_distance(v, '[0,0,0]'::ruvector))
            FROM test_agg
        "#;

        let result = Spi::get_one::<f64>(query).unwrap().unwrap();

        // All vectors are unit vectors at distance 1 from origin
        assert!(
            (result - 1.0).abs() < 0.001,
            "Average distance: expected 1, got {}",
            result
        );

        Spi::run("DROP TABLE test_agg").unwrap();
    }

    // ========================================================================
    // Precision and Numerical Stability Tests
    // ========================================================================

    #[pg_test]
    fn test_precision_small_values() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_l2_distance('[0.00001,0.00002]'::ruvector, '[0.00003,0.00004]'::ruvector)"
        ).unwrap().unwrap();

        // Should compute correctly even for small values
        assert!(result > 0.0 && result.is_finite());
    }

    #[pg_test]
    fn test_precision_large_values() {
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_l2_distance('[10000,20000]'::ruvector, '[10001,20001]'::ruvector)"
        ).unwrap().unwrap();

        // sqrt(1 + 1) = sqrt(2) = 1.414
        assert!(
            (result - 1.414).abs() < 0.01,
            "Large value precision: expected ~1.414, got {}",
            result
        );
    }

    #[pg_test]
    fn test_cosine_nearly_identical() {
        // Test numerical stability with nearly identical vectors
        let result = Spi::get_one::<f32>(
            "SELECT ruvector_cosine_distance('[1,2,3]'::ruvector, '[1.000001,2.000001,3.000001]'::ruvector)"
        ).unwrap().unwrap();

        assert!(result >= 0.0 && result < 0.001, "Nearly identical should have near-zero distance");
    }

    // ========================================================================
    // Known pgvector Results (Regression)
    // ========================================================================

    #[pg_test]
    fn test_known_result_from_pgvector_docs() {
        // From pgvector documentation examples
        let l2 = Spi::get_one::<f32>(
            "SELECT ruvector_l2_distance('[1,2,3]'::ruvector, '[4,5,6]'::ruvector)"
        ).unwrap().unwrap();

        // sqrt((4-1)^2 + (5-2)^2 + (6-3)^2) = sqrt(9+9+9) = sqrt(27) = 5.196
        assert!((l2 - 5.196).abs() < 0.01, "pgvector example L2: expected ~5.196, got {}", l2);
    }

    #[pg_test]
    fn test_triangle_inequality() {
        // L2 distance should satisfy triangle inequality: d(a,c) <= d(a,b) + d(b,c)
        let ab = Spi::get_one::<f64>(
            "SELECT ruvector_l2_distance('[0,0]'::ruvector, '[1,0]'::ruvector)"
        ).unwrap().unwrap();

        let bc = Spi::get_one::<f64>(
            "SELECT ruvector_l2_distance('[1,0]'::ruvector, '[1,1]'::ruvector)"
        ).unwrap().unwrap();

        let ac = Spi::get_one::<f64>(
            "SELECT ruvector_l2_distance('[0,0]'::ruvector, '[1,1]'::ruvector)"
        ).unwrap().unwrap();

        assert!(ac <= ab + bc + EPSILON, "Triangle inequality violated");
    }
}
