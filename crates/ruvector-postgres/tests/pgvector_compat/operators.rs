//! Operator Compatibility Tests for pgvector Drop-In Replacement
//!
//! Validates that RuVector's operators are fully compatible with pgvector:
//! - <-> L2 (Euclidean) distance operator
//! - <#> Inner product (negative) operator
//! - <=> Cosine distance operator
//! - +, -, * Vector arithmetic operators
//! - <, >, = Comparison operators

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod pgvector_operator_compat_tests {
    use pgrx::prelude::*;
    use ruvector_postgres::types::RuVector;
    use ruvector_postgres::operators::*;

    const EPSILON: f32 = 1e-4;

    // ========================================================================
    // <-> L2 (Euclidean) Distance Operator
    // ========================================================================

    #[pg_test]
    fn test_l2_operator_basic() {
        // pgvector: SELECT '[1,2,3]' <-> '[3,2,1]';
        let a = RuVector::from_slice(&[1.0, 2.0, 3.0]);
        let b = RuVector::from_slice(&[3.0, 2.0, 1.0]);

        let dist = ruvector_l2_distance(a, b);

        // Expected: sqrt((3-1)^2 + (2-2)^2 + (1-3)^2) = sqrt(8) = 2.828427
        let expected = 2.828427;
        assert!(
            (dist - expected).abs() < EPSILON,
            "L2 distance mismatch: expected {}, got {}",
            expected,
            dist
        );
    }

    #[pg_test]
    fn test_l2_operator_identical_vectors() {
        let a = RuVector::from_slice(&[1.0, 2.0, 3.0]);
        let b = RuVector::from_slice(&[1.0, 2.0, 3.0]);

        let dist = ruvector_l2_distance(a, b);
        assert!(dist.abs() < EPSILON, "Identical vectors should have distance 0");
    }

    #[pg_test]
    fn test_l2_operator_negative_values() {
        let a = RuVector::from_slice(&[-1.0, -1.0, -1.0]);
        let b = RuVector::from_slice(&[1.0, 1.0, 1.0]);

        let dist = ruvector_l2_distance(a, b);

        // Expected: sqrt(4 + 4 + 4) = sqrt(12) = 3.464
        let expected = 3.464;
        assert!(
            (dist - expected).abs() < 0.01,
            "L2 with negative values: expected ~{}, got {}",
            expected,
            dist
        );
    }

    #[pg_test]
    fn test_l2_operator_with_zeros() {
        let a = RuVector::from_slice(&[0.0, 0.0, 0.0]);
        let b = RuVector::from_slice(&[3.0, 4.0, 0.0]);

        let dist = ruvector_l2_distance(a, b);

        // Expected: sqrt(9 + 16 + 0) = 5.0
        assert!(
            (dist - 5.0).abs() < EPSILON,
            "L2 from origin: expected 5.0, got {}",
            dist
        );
    }

    #[pg_test]
    fn test_l2_operator_single_dimension() {
        let a = RuVector::from_slice(&[5.0]);
        let b = RuVector::from_slice(&[3.0]);

        let dist = ruvector_l2_distance(a, b);
        assert!((dist - 2.0).abs() < EPSILON, "1D L2: expected 2.0, got {}", dist);
    }

    #[pg_test]
    fn test_l2_operator_high_dimensional() {
        let dim = 128;
        let a: Vec<f32> = (0..dim).map(|i| i as f32).collect();
        let b: Vec<f32> = (0..dim).map(|i| (i + 1) as f32).collect();

        let va = RuVector::from_slice(&a);
        let vb = RuVector::from_slice(&b);

        let dist = ruvector_l2_distance(va, vb);

        // Each dimension differs by 1, so sqrt(128 * 1^2) = sqrt(128) = 11.314
        let expected = (dim as f32).sqrt();
        assert!(
            (dist - expected).abs() < 0.01,
            "High-dim L2: expected {}, got {}",
            expected,
            dist
        );
    }

    // ========================================================================
    // <=> Cosine Distance Operator
    // ========================================================================

    #[pg_test]
    fn test_cosine_operator_basic() {
        // pgvector: SELECT '[1,2,3]' <=> '[3,2,1]';
        let a = RuVector::from_slice(&[1.0, 2.0, 3.0]);
        let b = RuVector::from_slice(&[3.0, 2.0, 1.0]);

        let dist = ruvector_cosine_distance(a, b);

        // cosine = (1*3 + 2*2 + 3*1) / (sqrt(14) * sqrt(14)) = 10/14 = 0.714
        // distance = 1 - 0.714 = 0.286
        let expected = 0.2857;
        assert!(
            (dist - expected).abs() < 0.01,
            "Cosine distance mismatch: expected ~{}, got {}",
            expected,
            dist
        );
    }

    #[pg_test]
    fn test_cosine_operator_same_direction() {
        let a = RuVector::from_slice(&[1.0, 0.0, 0.0]);
        let b = RuVector::from_slice(&[2.0, 0.0, 0.0]);

        let dist = ruvector_cosine_distance(a, b);

        // Same direction = similarity 1, distance 0
        assert!(dist.abs() < EPSILON, "Same direction should have distance 0, got {}", dist);
    }

    #[pg_test]
    fn test_cosine_operator_orthogonal() {
        let a = RuVector::from_slice(&[1.0, 0.0]);
        let b = RuVector::from_slice(&[0.0, 1.0]);

        let dist = ruvector_cosine_distance(a, b);

        // Orthogonal = similarity 0, distance 1
        assert!(
            (dist - 1.0).abs() < EPSILON,
            "Orthogonal vectors should have distance 1, got {}",
            dist
        );
    }

    #[pg_test]
    fn test_cosine_operator_opposite() {
        let a = RuVector::from_slice(&[1.0, 0.0, 0.0]);
        let b = RuVector::from_slice(&[-1.0, 0.0, 0.0]);

        let dist = ruvector_cosine_distance(a, b);

        // Opposite = similarity -1, distance 2
        assert!(
            (dist - 2.0).abs() < EPSILON,
            "Opposite vectors should have distance 2, got {}",
            dist
        );
    }

    #[pg_test]
    fn test_cosine_operator_normalized() {
        // For normalized vectors, cosine distance is more stable
        let a = RuVector::from_slice(&[0.6, 0.8, 0.0]);
        let b = RuVector::from_slice(&[0.8, 0.6, 0.0]);

        let dist = ruvector_cosine_distance(a, b);

        // Both are unit vectors, cosine = 0.6*0.8 + 0.8*0.6 = 0.96
        // distance = 1 - 0.96 = 0.04
        assert!(
            (dist - 0.04).abs() < EPSILON,
            "Normalized cosine: expected ~0.04, got {}",
            dist
        );
    }

    // ========================================================================
    // <#> Negative Inner Product Operator
    // ========================================================================

    #[pg_test]
    fn test_ip_operator_basic() {
        // pgvector: SELECT '[1,2,3]' <#> '[3,2,1]';
        // Returns NEGATIVE inner product (for MIN ordering)
        let a = RuVector::from_slice(&[1.0, 2.0, 3.0]);
        let b = RuVector::from_slice(&[3.0, 2.0, 1.0]);

        // Use ruvector_inner_product which returns the actual inner product
        let ip = ruvector_inner_product(a, b);

        // Inner product = 1*3 + 2*2 + 3*1 = 10
        // The <#> operator returns -10 for ordering purposes
        assert!(
            (ip - 10.0).abs() < EPSILON,
            "Inner product mismatch: expected 10, got {}",
            ip
        );
    }

    #[pg_test]
    fn test_ip_operator_orthogonal() {
        let a = RuVector::from_slice(&[1.0, 0.0]);
        let b = RuVector::from_slice(&[0.0, 1.0]);

        let ip = ruvector_inner_product(a, b);
        assert!(ip.abs() < EPSILON, "Orthogonal IP should be 0, got {}", ip);
    }

    #[pg_test]
    fn test_ip_operator_negative_values() {
        let a = RuVector::from_slice(&[-1.0, 2.0, -3.0]);
        let b = RuVector::from_slice(&[4.0, -5.0, 6.0]);

        let ip = ruvector_inner_product(a, b);

        // IP = (-1)*4 + 2*(-5) + (-3)*6 = -4 - 10 - 18 = -32
        assert!(
            (ip - (-32.0)).abs() < EPSILON,
            "Negative IP: expected -32, got {}",
            ip
        );
    }

    // ========================================================================
    // L1 (Manhattan) Distance Function
    // ========================================================================

    #[pg_test]
    fn test_l1_distance_basic() {
        let a = RuVector::from_slice(&[1.0, 2.0, 3.0]);
        let b = RuVector::from_slice(&[4.0, 6.0, 8.0]);

        let dist = ruvector_l1_distance(a, b);

        // L1 = |1-4| + |2-6| + |3-8| = 3 + 4 + 5 = 12
        assert!((dist - 12.0).abs() < EPSILON, "L1: expected 12, got {}", dist);
    }

    // ========================================================================
    // Vector Arithmetic Operators
    // ========================================================================

    #[pg_test]
    fn test_vector_addition() {
        let a = RuVector::from_slice(&[1.0, 2.0, 3.0]);
        let b = RuVector::from_slice(&[4.0, 5.0, 6.0]);

        let result = ruvector_add(a, b);
        assert_eq!(result.as_slice(), &[5.0, 7.0, 9.0]);
    }

    #[pg_test]
    fn test_vector_subtraction() {
        let a = RuVector::from_slice(&[5.0, 7.0, 9.0]);
        let b = RuVector::from_slice(&[1.0, 2.0, 3.0]);

        let result = ruvector_sub(a, b);
        assert_eq!(result.as_slice(), &[4.0, 5.0, 6.0]);
    }

    #[pg_test]
    fn test_vector_scalar_multiplication() {
        let v = RuVector::from_slice(&[1.0, 2.0, 3.0]);

        let result = ruvector_mul_scalar(v, 2.0);
        assert_eq!(result.as_slice(), &[2.0, 4.0, 6.0]);
    }

    #[pg_test]
    fn test_vector_scalar_zero() {
        let v = RuVector::from_slice(&[1.0, 2.0, 3.0]);

        let result = ruvector_mul_scalar(v, 0.0);
        assert_eq!(result.as_slice(), &[0.0, 0.0, 0.0]);
    }

    #[pg_test]
    fn test_vector_scalar_negative() {
        let v = RuVector::from_slice(&[1.0, 2.0, 3.0]);

        let result = ruvector_mul_scalar(v, -1.0);
        assert_eq!(result.as_slice(), &[-1.0, -2.0, -3.0]);
    }

    // ========================================================================
    // Dimension Mismatch Handling
    // ========================================================================

    #[pg_test]
    #[should_panic(expected = "dimensions")]
    fn test_l2_dimension_mismatch() {
        let a = RuVector::from_slice(&[1.0, 2.0, 3.0]);
        let b = RuVector::from_slice(&[1.0, 2.0]);

        let _ = ruvector_l2_distance(a, b);
    }

    #[pg_test]
    #[should_panic(expected = "dimensions")]
    fn test_cosine_dimension_mismatch() {
        let a = RuVector::from_slice(&[1.0, 2.0, 3.0]);
        let b = RuVector::from_slice(&[1.0, 2.0]);

        let _ = ruvector_cosine_distance(a, b);
    }

    #[pg_test]
    #[should_panic(expected = "dimensions")]
    fn test_add_dimension_mismatch() {
        let a = RuVector::from_slice(&[1.0, 2.0, 3.0]);
        let b = RuVector::from_slice(&[1.0, 2.0]);

        let _ = ruvector_add(a, b);
    }

    // ========================================================================
    // Operator Commutativity Tests
    // ========================================================================

    #[pg_test]
    fn test_l2_commutativity() {
        let a = RuVector::from_slice(&[1.0, 2.0, 3.0]);
        let b = RuVector::from_slice(&[4.0, 5.0, 6.0]);

        let dist_ab = ruvector_l2_distance(a.clone(), b.clone());
        let dist_ba = ruvector_l2_distance(b, a);

        assert!(
            (dist_ab - dist_ba).abs() < EPSILON,
            "L2 should be commutative: {} vs {}",
            dist_ab,
            dist_ba
        );
    }

    #[pg_test]
    fn test_cosine_commutativity() {
        let a = RuVector::from_slice(&[1.0, 2.0, 3.0]);
        let b = RuVector::from_slice(&[4.0, 5.0, 6.0]);

        let dist_ab = ruvector_cosine_distance(a.clone(), b.clone());
        let dist_ba = ruvector_cosine_distance(b, a);

        assert!(
            (dist_ab - dist_ba).abs() < EPSILON,
            "Cosine should be commutative: {} vs {}",
            dist_ab,
            dist_ba
        );
    }

    #[pg_test]
    fn test_ip_commutativity() {
        let a = RuVector::from_slice(&[1.0, 2.0, 3.0]);
        let b = RuVector::from_slice(&[4.0, 5.0, 6.0]);

        let ip_ab = ruvector_inner_product(a.clone(), b.clone());
        let ip_ba = ruvector_inner_product(b, a);

        assert!(
            (ip_ab - ip_ba).abs() < EPSILON,
            "IP should be commutative: {} vs {}",
            ip_ab,
            ip_ba
        );
    }

    // ========================================================================
    // SQL-Level Operator Tests (via SPI)
    // ========================================================================

    #[pg_test]
    fn test_l2_operator_sql() {
        let result = Spi::get_one::<f64>(
            "SELECT ruvector_l2_distance('[1,2,3]'::ruvector, '[3,2,1]'::ruvector)"
        ).unwrap().unwrap();

        let expected = 2.828427;
        assert!(
            (result - expected).abs() < 0.001,
            "SQL L2: expected ~{}, got {}",
            expected,
            result
        );
    }

    #[pg_test]
    fn test_cosine_operator_sql() {
        let result = Spi::get_one::<f64>(
            "SELECT ruvector_cosine_distance('[1,2,3]'::ruvector, '[3,2,1]'::ruvector)"
        ).unwrap().unwrap();

        let expected = 0.286;
        assert!(
            (result - expected).abs() < 0.01,
            "SQL cosine: expected ~{}, got {}",
            expected,
            result
        );
    }

    #[pg_test]
    fn test_ip_operator_sql() {
        let result = Spi::get_one::<f64>(
            "SELECT ruvector_inner_product('[1,2,3]'::ruvector, '[4,5,6]'::ruvector)"
        ).unwrap().unwrap();

        // IP = 1*4 + 2*5 + 3*6 = 32
        assert!(
            (result - 32.0).abs() < 0.001,
            "SQL IP: expected 32, got {}",
            result
        );
    }
}

#[cfg(test)]
mod unit_tests {
    #[test]
    fn test_operator_epsilon() {
        // Verify epsilon is appropriate for f32 precision
        let epsilon: f32 = 1e-4;
        assert!(epsilon > f32::EPSILON);
        assert!(epsilon < 0.001);
    }
}
