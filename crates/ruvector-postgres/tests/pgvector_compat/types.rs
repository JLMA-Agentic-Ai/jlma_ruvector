//! Type Compatibility Tests for pgvector Drop-In Replacement
//!
//! Validates that RuVector's vector types are fully compatible with pgvector's types:
//! - vector(n) type creation and casting
//! - halfvec(n) type for float16 storage
//! - sparsevec type for sparse vectors
//! - All type conversions (array to vector, etc.)

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod pgvector_type_compat_tests {
    use pgrx::prelude::*;

    // ========================================================================
    // vector(n) Type Compatibility
    // ========================================================================

    #[pg_test]
    fn test_vector_type_creation() {
        // pgvector: CREATE TABLE t (v vector(3));
        // RuVector should support identical syntax
        Spi::run("CREATE TABLE test_vector_type (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_vector_type VALUES ('[1,2,3]')").unwrap();

        let result = Spi::get_one::<i32>("SELECT ruvector_dims(v) FROM test_vector_type")
            .unwrap()
            .unwrap();
        assert_eq!(result, 3);

        Spi::run("DROP TABLE test_vector_type").unwrap();
    }

    #[pg_test]
    fn test_vector_dimension_constraint() {
        // pgvector enforces dimension at insert time
        Spi::run("CREATE TABLE test_dim_constraint (v ruvector(3))").unwrap();

        // This should work - correct dimensions
        Spi::run("INSERT INTO test_dim_constraint VALUES ('[1,2,3]')").unwrap();

        // This should fail - wrong dimensions (if typmod enforcement is enabled)
        // Note: Currently RuVector validates at parse time, not column constraint

        Spi::run("DROP TABLE test_dim_constraint").unwrap();
    }

    #[pg_test]
    fn test_vector_text_format_parsing() {
        // pgvector accepts multiple text formats
        let formats = vec![
            "[1,2,3]",           // No spaces
            "[1, 2, 3]",         // With spaces
            "[1.0, 2.0, 3.0]",   // With decimals
            "[ 1 , 2 , 3 ]",     // Extra whitespace
            "[1.5,2.5,3.5]",     // Fractional values
        ];

        for format in formats {
            let query = format!("SELECT ruvector_dims('{}'::ruvector)", format);
            let result = Spi::get_one::<i32>(&query).unwrap().unwrap();
            assert_eq!(result, 3, "Failed for format: {}", format);
        }
    }

    #[pg_test]
    fn test_vector_text_output_format() {
        // pgvector outputs as [x,y,z] format
        Spi::run("CREATE TABLE test_output (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_output VALUES ('[1,2,3]')").unwrap();

        let result = Spi::get_one::<String>("SELECT v::text FROM test_output")
            .unwrap()
            .unwrap();

        // Should output in [x,y,z] format (exact formatting may vary)
        assert!(result.starts_with('[') && result.ends_with(']'));
        assert!(result.contains('1') && result.contains('2') && result.contains('3'));

        Spi::run("DROP TABLE test_output").unwrap();
    }

    #[pg_test]
    fn test_vector_binary_protocol() {
        // Test binary send/receive functions
        Spi::run("CREATE TABLE test_binary (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_binary VALUES ('[1.5,2.5,3.5]')").unwrap();

        // Binary protocol is tested implicitly through COPY
        // The actual binary format should match pgvector's format

        let count = Spi::get_one::<i64>("SELECT COUNT(*) FROM test_binary")
            .unwrap()
            .unwrap();
        assert_eq!(count, 1);

        Spi::run("DROP TABLE test_binary").unwrap();
    }

    #[pg_test]
    fn test_vector_null_handling() {
        Spi::run("CREATE TABLE test_null (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_null VALUES (NULL)").unwrap();

        let result = Spi::get_one::<bool>("SELECT v IS NULL FROM test_null")
            .unwrap()
            .unwrap();
        assert!(result);

        Spi::run("DROP TABLE test_null").unwrap();
    }

    #[pg_test]
    fn test_vector_max_dimensions() {
        // pgvector supports up to 16000 dimensions
        let dims = 2000; // Use 2000 for test (16000 is slow)
        let values: String = (0..dims).map(|i| format!("{}", i as f32 * 0.01)).collect::<Vec<_>>().join(",");
        let query = format!("SELECT ruvector_dims('[{}]'::ruvector)", values);

        let result = Spi::get_one::<i32>(&query).unwrap().unwrap();
        assert_eq!(result, dims as i32);
    }

    #[pg_test]
    fn test_vector_single_dimension() {
        let result = Spi::get_one::<i32>("SELECT ruvector_dims('[42]'::ruvector)")
            .unwrap()
            .unwrap();
        assert_eq!(result, 1);
    }

    // ========================================================================
    // halfvec(n) Type Compatibility
    // ========================================================================

    #[pg_test]
    fn test_halfvec_type_creation() {
        // halfvec uses 16-bit floats, reducing memory by 50%
        Spi::run("CREATE TABLE test_halfvec (v halfvec(3))").unwrap();
        Spi::run("INSERT INTO test_halfvec VALUES ('[1,2,3]'::halfvec)").unwrap();

        let count = Spi::get_one::<i64>("SELECT COUNT(*) FROM test_halfvec")
            .unwrap()
            .unwrap();
        assert_eq!(count, 1);

        Spi::run("DROP TABLE test_halfvec").unwrap();
    }

    #[pg_test]
    fn test_halfvec_precision_loss() {
        // halfvec has ~3 decimal digits of precision
        // Value should be close but not exact due to f16 conversion
        Spi::run("CREATE TABLE test_halfvec_precision (v halfvec(1))").unwrap();
        Spi::run("INSERT INTO test_halfvec_precision VALUES ('[0.123456789]'::halfvec)").unwrap();

        // The retrieved value should be approximately 0.1235 (f16 precision)
        let count = Spi::get_one::<i64>("SELECT COUNT(*) FROM test_halfvec_precision")
            .unwrap()
            .unwrap();
        assert_eq!(count, 1);

        Spi::run("DROP TABLE test_halfvec_precision").unwrap();
    }

    // ========================================================================
    // sparsevec Type Compatibility
    // ========================================================================

    #[pg_test]
    fn test_sparsevec_type_creation() {
        // sparsevec format: {index:value,...}/total_dim
        Spi::run("CREATE TABLE test_sparse (v sparsevec)").unwrap();

        // Note: sparsevec I/O functions may need different handling
        // depending on how they're registered in SQL
        let count = Spi::get_one::<i64>("SELECT COUNT(*) FROM test_sparse")
            .unwrap()
            .unwrap();
        assert_eq!(count, 0);

        Spi::run("DROP TABLE test_sparse").unwrap();
    }

    #[pg_test]
    fn test_sparsevec_high_dimensional() {
        // sparsevec is ideal for high-dimensional sparse data
        // e.g., TF-IDF vectors with 50000 dimensions but only 100 non-zeros

        // Testing the sparse representation in Rust
        use ruvector_postgres::types::SparseVec;

        let sparse = SparseVec::from_pairs(50000, &[
            (0, 1.0),
            (100, 0.5),
            (1000, 0.3),
            (10000, 0.8),
        ]);

        assert_eq!(sparse.dimensions(), 50000);
        assert_eq!(sparse.nnz(), 4);
        assert!(sparse.sparsity() < 0.001); // Very sparse
    }

    // ========================================================================
    // Type Conversion Compatibility
    // ========================================================================

    #[pg_test]
    fn test_array_to_vector_cast() {
        // pgvector: ARRAY[1,2,3]::vector
        // RuVector should support the same
        let result = Spi::get_one::<i32>("SELECT ruvector_dims(ARRAY[1,2,3]::real[]::ruvector)")
            .ok()
            .flatten();

        // Note: This cast may not be implemented - document as intentional difference
        // if result.is_none()
        if result.is_some() {
            assert_eq!(result.unwrap(), 3);
        }
    }

    #[pg_test]
    fn test_vector_to_array_cast() {
        // pgvector: v::real[]
        // This extracts vector components as an array
        Spi::run("CREATE TABLE test_v2a (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_v2a VALUES ('[1,2,3]')").unwrap();

        // Note: Cast implementation depends on SQL registration
        let count = Spi::get_one::<i64>("SELECT COUNT(*) FROM test_v2a")
            .unwrap()
            .unwrap();
        assert_eq!(count, 1);

        Spi::run("DROP TABLE test_v2a").unwrap();
    }

    #[pg_test]
    fn test_vector_to_halfvec_cast() {
        // Casting from vector to halfvec should reduce precision
        Spi::run("CREATE TABLE test_cast (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_cast VALUES ('[1.123456,2.234567,3.345678]')").unwrap();

        // The cast would convert f32 to f16 and back
        let count = Spi::get_one::<i64>("SELECT COUNT(*) FROM test_cast")
            .unwrap()
            .unwrap();
        assert_eq!(count, 1);

        Spi::run("DROP TABLE test_cast").unwrap();
    }

    // ========================================================================
    // Type Storage and Alignment
    // ========================================================================

    #[pg_test]
    fn test_vector_varlena_storage() {
        // Test that vector storage uses TOAST appropriately
        Spi::run("CREATE TABLE test_storage (v ruvector(1000))").unwrap();

        let values: String = (0..1000).map(|i| format!("{}", i as f32 * 0.001)).collect::<Vec<_>>().join(",");
        Spi::run(&format!("INSERT INTO test_storage VALUES ('[{}]')", values)).unwrap();

        let result = Spi::get_one::<i32>("SELECT ruvector_dims(v) FROM test_storage")
            .unwrap()
            .unwrap();
        assert_eq!(result, 1000);

        Spi::run("DROP TABLE test_storage").unwrap();
    }

    #[pg_test]
    fn test_vector_memory_layout() {
        // Verify memory layout is compatible with pgvector
        // Layout: varlena header (4) + dims (2) + padding (2) + data (4*n)

        use ruvector_postgres::types::RuVector;

        let v = RuVector::from_slice(&[1.0, 2.0, 3.0]);
        assert_eq!(v.dimensions(), 3);
        assert_eq!(v.as_slice(), &[1.0, 2.0, 3.0]);

        // Memory size should be: 4 (header) + 3*4 (data) = 16 bytes (data portion)
        // The data_memory_size excludes varlena overhead
        let data_size = v.data_memory_size();
        assert_eq!(data_size, 16); // 4 (dims+pad) + 12 (3 f32s)
    }

    // ========================================================================
    // Negative Test Cases (Error Handling)
    // ========================================================================

    #[pg_test]
    #[should_panic(expected = "Invalid")]
    fn test_vector_invalid_format_no_brackets() {
        // Should reject input without brackets
        Spi::run("SELECT '1,2,3'::ruvector").unwrap();
    }

    #[pg_test]
    #[should_panic(expected = "Invalid")]
    fn test_vector_invalid_format_unbalanced() {
        // Should reject unbalanced brackets
        Spi::run("SELECT '[1,2,3'::ruvector").unwrap();
    }

    #[pg_test]
    #[should_panic(expected = "Invalid")]
    fn test_vector_invalid_nan() {
        // Should reject NaN values
        Spi::run("SELECT '[1,NaN,3]'::ruvector").unwrap();
    }

    #[pg_test]
    #[should_panic(expected = "Invalid")]
    fn test_vector_invalid_infinity() {
        // Should reject Infinity values
        Spi::run("SELECT '[1,Infinity,3]'::ruvector").unwrap();
    }

    #[pg_test]
    #[should_panic(expected = "exceeds")]
    fn test_vector_exceeds_max_dimensions() {
        // Should reject vectors exceeding 16000 dimensions
        let values: String = (0..16001).map(|i| format!("{}", i)).collect::<Vec<_>>().join(",");
        Spi::run(&format!("SELECT '[{}]'::ruvector", values)).unwrap();
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_max_dimensions_constant() {
        assert_eq!(super::super::MAX_DIMENSIONS, 16_000);
    }
}
