//! Query Compatibility Tests for pgvector Drop-In Replacement
//!
//! Validates that RuVector supports the same query patterns as pgvector:
//! - ORDER BY with distance operators
//! - LIMIT with approximate search
//! - WHERE clause filtering
//! - Aggregate functions with vectors
//! - Subqueries and CTEs
//! - Complex query patterns

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod pgvector_query_compat_tests {
    use pgrx::prelude::*;

    // ========================================================================
    // ORDER BY Distance Queries
    // ========================================================================

    #[pg_test]
    fn test_order_by_l2_distance() {
        Spi::run("CREATE TABLE test_order (id serial, v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_order (v) VALUES
            ('[0,0,0]'),
            ('[1,1,1]'),
            ('[2,2,2]'),
            ('[3,3,3]')
        ").unwrap();

        // Order by distance from [1,1,1]
        let query = r#"
            SELECT id, ruvector_l2_distance(v, '[1,1,1]'::ruvector) as dist
            FROM test_order
            ORDER BY dist
        "#;

        let first_id = Spi::get_one::<i32>(
            "SELECT id FROM test_order ORDER BY ruvector_l2_distance(v, '[1,1,1]'::ruvector) LIMIT 1"
        ).unwrap().unwrap();

        assert_eq!(first_id, 2, "Nearest to [1,1,1] should be id=2");

        Spi::run("DROP TABLE test_order").unwrap();
    }

    #[pg_test]
    fn test_order_by_cosine_distance() {
        Spi::run("CREATE TABLE test_cosine_order (id serial, v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_cosine_order (v) VALUES
            ('[1,0,0]'),
            ('[0,1,0]'),
            ('[1,1,0]'),
            ('[-1,0,0]')
        ").unwrap();

        // Order by cosine distance from [1,0,0]
        // Closest should be [1,0,0] (same direction)
        // Furthest should be [-1,0,0] (opposite direction)

        let closest_id = Spi::get_one::<i32>(
            "SELECT id FROM test_cosine_order ORDER BY ruvector_cosine_distance(v, '[1,0,0]'::ruvector) LIMIT 1"
        ).unwrap().unwrap();

        assert_eq!(closest_id, 1, "Same direction should be closest");

        Spi::run("DROP TABLE test_cosine_order").unwrap();
    }

    #[pg_test]
    fn test_order_by_inner_product() {
        Spi::run("CREATE TABLE test_ip_order (id serial, v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_ip_order (v) VALUES
            ('[1,1,1]'),
            ('[2,2,2]'),
            ('[3,3,3]')
        ").unwrap();

        // For MAX inner product, we use negative inner product and ORDER BY ASC
        // or ORDER BY inner_product DESC
        let highest_ip_id = Spi::get_one::<i32>(
            "SELECT id FROM test_ip_order ORDER BY ruvector_inner_product(v, '[1,1,1]'::ruvector) DESC LIMIT 1"
        ).unwrap().unwrap();

        assert_eq!(highest_ip_id, 3, "[3,3,3] has highest IP with [1,1,1]");

        Spi::run("DROP TABLE test_ip_order").unwrap();
    }

    // ========================================================================
    // LIMIT Queries
    // ========================================================================

    #[pg_test]
    fn test_limit_basic() {
        Spi::run("CREATE TABLE test_limit (id serial, v ruvector(3))").unwrap();
        for i in 0..100 {
            Spi::run(&format!("INSERT INTO test_limit (v) VALUES ('[{},{},{}]')", i, i, i)).unwrap();
        }

        let query = r#"
            SELECT id
            FROM test_limit
            ORDER BY ruvector_l2_distance(v, '[50,50,50]'::ruvector)
            LIMIT 10
        "#;

        let count = Spi::connect(|client| {
            let tup_table = client.select(query, None, None)?;
            Ok::<_, spi::Error>(tup_table.len())
        }).unwrap();

        assert_eq!(count, 10, "LIMIT 10 should return exactly 10 rows");

        Spi::run("DROP TABLE test_limit").unwrap();
    }

    #[pg_test]
    fn test_limit_offset() {
        Spi::run("CREATE TABLE test_offset (id serial, v ruvector(3))").unwrap();
        for i in 0..20 {
            Spi::run(&format!("INSERT INTO test_offset (v) VALUES ('[{},{},{}]')", i, i, i)).unwrap();
        }

        let query = r#"
            SELECT id
            FROM test_offset
            ORDER BY ruvector_l2_distance(v, '[10,10,10]'::ruvector)
            LIMIT 5 OFFSET 5
        "#;

        let count = Spi::connect(|client| {
            let tup_table = client.select(query, None, None)?;
            Ok::<_, spi::Error>(tup_table.len())
        }).unwrap();

        assert_eq!(count, 5, "LIMIT 5 OFFSET 5 should return 5 rows");

        Spi::run("DROP TABLE test_offset").unwrap();
    }

    // ========================================================================
    // WHERE Clause Filtering
    // ========================================================================

    #[pg_test]
    fn test_where_with_knn() {
        Spi::run("CREATE TABLE test_where (id serial, category text, v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_where (category, v) VALUES
            ('A', '[1,1,1]'),
            ('B', '[2,2,2]'),
            ('A', '[3,3,3]'),
            ('B', '[4,4,4]'),
            ('A', '[5,5,5]')
        ").unwrap();

        let query = r#"
            SELECT id
            FROM test_where
            WHERE category = 'A'
            ORDER BY ruvector_l2_distance(v, '[3,3,3]'::ruvector)
            LIMIT 2
        "#;

        let first_id = Spi::get_one::<i32>(
            "SELECT id FROM test_where WHERE category = 'A'
             ORDER BY ruvector_l2_distance(v, '[3,3,3]'::ruvector) LIMIT 1"
        ).unwrap().unwrap();

        assert_eq!(first_id, 3, "Nearest A to [3,3,3] should be id=3");

        Spi::run("DROP TABLE test_where").unwrap();
    }

    #[pg_test]
    fn test_where_distance_threshold() {
        Spi::run("CREATE TABLE test_threshold (id serial, v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_threshold (v) VALUES
            ('[0,0,0]'),
            ('[1,0,0]'),
            ('[2,0,0]'),
            ('[10,0,0]')
        ").unwrap();

        // Find vectors within distance 3 of origin
        let query = r#"
            SELECT COUNT(*)
            FROM test_threshold
            WHERE ruvector_l2_distance(v, '[0,0,0]'::ruvector) < 3
        "#;

        let count = Spi::get_one::<i64>(query).unwrap().unwrap();
        assert_eq!(count, 3, "3 vectors should be within distance 3 of origin");

        Spi::run("DROP TABLE test_threshold").unwrap();
    }

    #[pg_test]
    fn test_where_multiple_conditions() {
        Spi::run("CREATE TABLE test_multi (id serial, category text, score float, v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_multi (category, score, v) VALUES
            ('A', 0.9, '[1,1,1]'),
            ('A', 0.5, '[2,2,2]'),
            ('B', 0.9, '[3,3,3]'),
            ('A', 0.9, '[4,4,4]')
        ").unwrap();

        let query = r#"
            SELECT id
            FROM test_multi
            WHERE category = 'A' AND score > 0.8
            ORDER BY ruvector_l2_distance(v, '[2,2,2]'::ruvector)
            LIMIT 2
        "#;

        let count = Spi::connect(|client| {
            let tup_table = client.select(query, None, None)?;
            Ok::<_, spi::Error>(tup_table.len())
        }).unwrap();

        assert_eq!(count, 2);

        Spi::run("DROP TABLE test_multi").unwrap();
    }

    // ========================================================================
    // Aggregate Functions with Vectors
    // ========================================================================

    #[pg_test]
    fn test_avg_distance() {
        Spi::run("CREATE TABLE test_avg (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_avg VALUES ('[1,0,0]'), ('[0,1,0]'), ('[0,0,1]')").unwrap();

        let avg = Spi::get_one::<f64>(
            "SELECT AVG(ruvector_l2_distance(v, '[0,0,0]'::ruvector)) FROM test_avg"
        ).unwrap().unwrap();

        // All unit vectors, distance = 1 from origin
        assert!((avg - 1.0).abs() < 0.001, "Average distance should be 1, got {}", avg);

        Spi::run("DROP TABLE test_avg").unwrap();
    }

    #[pg_test]
    fn test_min_max_distance() {
        Spi::run("CREATE TABLE test_minmax (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_minmax VALUES ('[1,0,0]'), ('[3,0,0]'), ('[5,0,0]')").unwrap();

        let min = Spi::get_one::<f64>(
            "SELECT MIN(ruvector_l2_distance(v, '[0,0,0]'::ruvector)) FROM test_minmax"
        ).unwrap().unwrap();

        let max = Spi::get_one::<f64>(
            "SELECT MAX(ruvector_l2_distance(v, '[0,0,0]'::ruvector)) FROM test_minmax"
        ).unwrap().unwrap();

        assert!((min - 1.0).abs() < 0.001, "Min distance should be 1, got {}", min);
        assert!((max - 5.0).abs() < 0.001, "Max distance should be 5, got {}", max);

        Spi::run("DROP TABLE test_minmax").unwrap();
    }

    #[pg_test]
    fn test_count_within_radius() {
        Spi::run("CREATE TABLE test_count (v ruvector(3))").unwrap();
        for i in 0..100 {
            Spi::run(&format!("INSERT INTO test_count VALUES ('[{},0,0]')", i)).unwrap();
        }

        let count = Spi::get_one::<i64>(
            "SELECT COUNT(*) FROM test_count WHERE ruvector_l2_distance(v, '[50,0,0]'::ruvector) <= 10"
        ).unwrap().unwrap();

        // Vectors from [40,0,0] to [60,0,0] = 21 vectors
        assert_eq!(count, 21, "21 vectors should be within distance 10 of [50,0,0]");

        Spi::run("DROP TABLE test_count").unwrap();
    }

    // ========================================================================
    // Subqueries and CTEs
    // ========================================================================

    #[pg_test]
    fn test_subquery_knn() {
        Spi::run("CREATE TABLE test_sub (id serial, v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_sub (v) VALUES ('[1,1,1]'), ('[2,2,2]'), ('[3,3,3]')").unwrap();

        let query = r#"
            SELECT * FROM (
                SELECT id, ruvector_l2_distance(v, '[2,2,2]'::ruvector) as dist
                FROM test_sub
                ORDER BY dist
                LIMIT 2
            ) AS nearest
            WHERE dist < 5
        "#;

        let count = Spi::connect(|client| {
            let tup_table = client.select(query, None, None)?;
            Ok::<_, spi::Error>(tup_table.len())
        }).unwrap();

        assert!(count >= 1 && count <= 2);

        Spi::run("DROP TABLE test_sub").unwrap();
    }

    #[pg_test]
    fn test_cte_knn() {
        Spi::run("CREATE TABLE test_cte (id serial, v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_cte (v) VALUES ('[1,1,1]'), ('[2,2,2]'), ('[3,3,3]')").unwrap();

        let query = r#"
            WITH nearest AS (
                SELECT id, ruvector_l2_distance(v, '[2,2,2]'::ruvector) as dist
                FROM test_cte
                ORDER BY dist
                LIMIT 2
            )
            SELECT id, dist FROM nearest ORDER BY dist
        "#;

        let count = Spi::connect(|client| {
            let tup_table = client.select(query, None, None)?;
            Ok::<_, spi::Error>(tup_table.len())
        }).unwrap();

        assert_eq!(count, 2);

        Spi::run("DROP TABLE test_cte").unwrap();
    }

    // ========================================================================
    // JOIN Queries
    // ========================================================================

    #[pg_test]
    fn test_join_with_knn() {
        Spi::run("CREATE TABLE items (id serial, name text, v ruvector(3))").unwrap();
        Spi::run("CREATE TABLE queries (id serial, query_name text, q ruvector(3))").unwrap();

        Spi::run("INSERT INTO items (name, v) VALUES
            ('item1', '[1,0,0]'),
            ('item2', '[0,1,0]'),
            ('item3', '[0,0,1]')
        ").unwrap();

        Spi::run("INSERT INTO queries (query_name, q) VALUES
            ('query1', '[0.9,0.1,0]')
        ").unwrap();

        let query = r#"
            SELECT i.name, q.query_name, ruvector_l2_distance(i.v, q.q) as dist
            FROM items i
            CROSS JOIN queries q
            ORDER BY dist
            LIMIT 1
        "#;

        let name = Spi::get_one::<String>(
            "SELECT i.name FROM items i CROSS JOIN queries q
             ORDER BY ruvector_l2_distance(i.v, q.q) LIMIT 1"
        ).unwrap().unwrap();

        assert_eq!(name, "item1", "item1 should be closest to [0.9,0.1,0]");

        Spi::run("DROP TABLE items").unwrap();
        Spi::run("DROP TABLE queries").unwrap();
    }

    // ========================================================================
    // GROUP BY Queries
    // ========================================================================

    #[pg_test]
    fn test_group_by_with_min_distance() {
        Spi::run("CREATE TABLE test_group (category text, v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_group VALUES
            ('A', '[1,0,0]'),
            ('A', '[2,0,0]'),
            ('B', '[10,0,0]'),
            ('B', '[11,0,0]')
        ").unwrap();

        let query = r#"
            SELECT category, MIN(ruvector_l2_distance(v, '[0,0,0]'::ruvector)) as min_dist
            FROM test_group
            GROUP BY category
            ORDER BY min_dist
        "#;

        let count = Spi::connect(|client| {
            let tup_table = client.select(query, None, None)?;
            Ok::<_, spi::Error>(tup_table.len())
        }).unwrap();

        assert_eq!(count, 2, "Should have 2 groups");

        Spi::run("DROP TABLE test_group").unwrap();
    }

    // ========================================================================
    // DISTINCT Queries
    // ========================================================================

    #[pg_test]
    fn test_distinct_on_knn() {
        Spi::run("CREATE TABLE test_distinct (category text, v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_distinct VALUES
            ('A', '[1,0,0]'),
            ('A', '[1.1,0,0]'),
            ('B', '[5,0,0]'),
            ('B', '[5.1,0,0]')
        ").unwrap();

        let query = r#"
            SELECT DISTINCT ON (category) category,
                   ruvector_l2_distance(v, '[0,0,0]'::ruvector) as dist
            FROM test_distinct
            ORDER BY category, dist
        "#;

        let count = Spi::connect(|client| {
            let tup_table = client.select(query, None, None)?;
            Ok::<_, spi::Error>(tup_table.len())
        }).unwrap();

        assert_eq!(count, 2, "Should have 2 distinct categories");

        Spi::run("DROP TABLE test_distinct").unwrap();
    }

    // ========================================================================
    // CASE Expressions
    // ========================================================================

    #[pg_test]
    fn test_case_with_distance() {
        Spi::run("CREATE TABLE test_case (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_case VALUES ('[0,0,0]'), ('[5,0,0]'), ('[15,0,0]')").unwrap();

        let query = r#"
            SELECT
                CASE
                    WHEN ruvector_l2_distance(v, '[0,0,0]'::ruvector) < 3 THEN 'near'
                    WHEN ruvector_l2_distance(v, '[0,0,0]'::ruvector) < 10 THEN 'medium'
                    ELSE 'far'
                END as proximity
            FROM test_case
        "#;

        let count = Spi::connect(|client| {
            let tup_table = client.select(query, None, None)?;
            Ok::<_, spi::Error>(tup_table.len())
        }).unwrap();

        assert_eq!(count, 3);

        Spi::run("DROP TABLE test_case").unwrap();
    }

    // ========================================================================
    // Window Functions
    // ========================================================================

    #[pg_test]
    fn test_window_function_rank() {
        Spi::run("CREATE TABLE test_window (id serial, category text, v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_window (category, v) VALUES
            ('A', '[1,0,0]'),
            ('A', '[2,0,0]'),
            ('B', '[10,0,0]'),
            ('B', '[11,0,0]')
        ").unwrap();

        let query = r#"
            SELECT id, category,
                   RANK() OVER (PARTITION BY category ORDER BY ruvector_l2_distance(v, '[0,0,0]'::ruvector)) as rank
            FROM test_window
        "#;

        let count = Spi::connect(|client| {
            let tup_table = client.select(query, None, None)?;
            Ok::<_, spi::Error>(tup_table.len())
        }).unwrap();

        assert_eq!(count, 4);

        Spi::run("DROP TABLE test_window").unwrap();
    }

    // ========================================================================
    // Prepared Statements
    // ========================================================================

    #[pg_test]
    fn test_prepared_statement() {
        Spi::run("CREATE TABLE test_prepared (v ruvector(3))").unwrap();
        Spi::run("INSERT INTO test_prepared VALUES ('[1,1,1]'), ('[2,2,2]'), ('[3,3,3]')").unwrap();

        // Simulate prepared statement by executing parameterized query multiple times
        for i in 1..=3 {
            let query = format!(
                "SELECT COUNT(*) FROM test_prepared WHERE ruvector_l2_distance(v, '[{},{},{}]'::ruvector) < 5",
                i, i, i
            );
            let count = Spi::get_one::<i64>(&query).unwrap().unwrap();
            assert!(count > 0, "Query {} should return results", i);
        }

        Spi::run("DROP TABLE test_prepared").unwrap();
    }

    // ========================================================================
    // RETURNING Clause
    // ========================================================================

    #[pg_test]
    fn test_insert_returning() {
        Spi::run("CREATE TABLE test_returning (id serial, v ruvector(3))").unwrap();

        let query = "INSERT INTO test_returning (v) VALUES ('[1,2,3]') RETURNING id, v::text";

        let result = Spi::connect(|client| {
            let tup_table = client.select(query, None, None)?;
            Ok::<_, spi::Error>(tup_table.len())
        }).unwrap();

        assert_eq!(result, 1);

        Spi::run("DROP TABLE test_returning").unwrap();
    }
}
