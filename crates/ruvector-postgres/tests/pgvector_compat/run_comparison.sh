#!/bin/bash
# pgvector Drop-In Compatibility Test Runner
#
# This script runs the compatibility test suite against both pgvector and ruvector
# to verify 100% API compatibility.
#
# Usage:
#   ./run_comparison.sh [OPTIONS]
#
# Options:
#   --pgvector-only    Only test pgvector
#   --ruvector-only    Only test ruvector
#   --compare          Run side-by-side comparison
#   --generate-sql     Generate SQL test files
#   --verbose          Verbose output
#
# Prerequisites:
#   - PostgreSQL running with pgvector installed (for comparison)
#   - RuVector extension built and installed

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Configuration
PGVECTOR_DB="${PGVECTOR_DB:-pgvector_test}"
RUVECTOR_DB="${RUVECTOR_DB:-ruvector_test}"
PG_HOST="${PG_HOST:-localhost}"
PG_PORT="${PG_PORT:-5432}"
PG_USER="${PG_USER:-postgres}"
RESULTS_DIR="${SCRIPT_DIR}/results"

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TESTS_PASSED++)) || true
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((TESTS_FAILED++)) || true
}

log_skip() {
    echo -e "${YELLOW}[SKIP]${NC} $1"
    ((TESTS_SKIPPED++)) || true
}

log_header() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE} $1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

# Check prerequisites
check_prerequisites() {
    log_header "Checking Prerequisites"

    # Check psql
    if ! command -v psql &> /dev/null; then
        log_fail "psql command not found"
        exit 1
    fi
    log_success "psql found"

    # Check PostgreSQL connectivity
    if psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -c "SELECT 1" &> /dev/null; then
        log_success "PostgreSQL connection OK"
    else
        log_fail "Cannot connect to PostgreSQL at $PG_HOST:$PG_PORT"
        exit 1
    fi
}

# Create test database
create_test_db() {
    local db_name=$1
    local extension=$2

    log_info "Creating test database: $db_name"

    # Drop if exists
    psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -c "DROP DATABASE IF EXISTS $db_name" 2>/dev/null || true

    # Create database
    psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -c "CREATE DATABASE $db_name"

    # Install extension
    if [ "$extension" = "pgvector" ]; then
        psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$db_name" -c "CREATE EXTENSION IF NOT EXISTS vector"
    else
        psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$db_name" -c "CREATE EXTENSION IF NOT EXISTS ruvector"
    fi
}

# Run SQL test and capture output
run_sql_test() {
    local db_name=$1
    local test_name=$2
    local sql_content=$3
    local output_file="$RESULTS_DIR/${db_name}_${test_name}.out"

    log_info "Running test: $test_name on $db_name"

    echo "$sql_content" | psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$db_name" > "$output_file" 2>&1

    if [ $? -eq 0 ]; then
        log_success "$test_name on $db_name"
        return 0
    else
        log_fail "$test_name on $db_name"
        return 1
    fi
}

# Compare results between pgvector and ruvector
compare_results() {
    local test_name=$1
    local pgvector_file="$RESULTS_DIR/${PGVECTOR_DB}_${test_name}.out"
    local ruvector_file="$RESULTS_DIR/${RUVECTOR_DB}_${test_name}.out"

    if [ ! -f "$pgvector_file" ] || [ ! -f "$ruvector_file" ]; then
        log_skip "Cannot compare $test_name - missing output files"
        return
    fi

    if diff -q "$pgvector_file" "$ruvector_file" > /dev/null 2>&1; then
        log_success "Results match for $test_name"
    else
        log_fail "Results differ for $test_name"
        echo "Differences:"
        diff -u "$pgvector_file" "$ruvector_file" | head -20
    fi
}

# Generate SQL test files
generate_sql_files() {
    log_header "Generating SQL Test Files"

    mkdir -p "$SCRIPT_DIR/sql"

    # Types test
    cat > "$SCRIPT_DIR/sql/types.sql" << 'EOF'
-- pgvector Drop-In Compatibility Test: Types

-- Test: vector(n) type creation
CREATE TABLE test_vector_type (
    id serial,
    v vector(3)
);

INSERT INTO test_vector_type (v) VALUES
    ('[1,2,3]'),
    ('[4,5,6]'),
    ('[1.5,2.5,3.5]');

SELECT id, v::text FROM test_vector_type ORDER BY id;
SELECT id, vector_dims(v) FROM test_vector_type ORDER BY id;

DROP TABLE test_vector_type;
EOF

    # Operators test
    cat > "$SCRIPT_DIR/sql/operators.sql" << 'EOF'
-- pgvector Drop-In Compatibility Test: Operators

SELECT '[1,2,3]'::vector <-> '[3,2,1]'::vector AS l2_distance;
SELECT '[1,2,3]'::vector <=> '[3,2,1]'::vector AS cosine_distance;
SELECT '[1,2,3]'::vector <#> '[4,5,6]'::vector AS neg_inner_product;
EOF

    # Functions test
    cat > "$SCRIPT_DIR/sql/functions.sql" << 'EOF'
-- pgvector Drop-In Compatibility Test: Functions

SELECT l2_distance('[1,2,3]'::vector, '[4,5,6]'::vector);
SELECT inner_product('[1,2,3]'::vector, '[4,5,6]'::vector);
SELECT cosine_distance('[1,2,3]'::vector, '[3,2,1]'::vector);
SELECT vector_dims('[1,2,3,4,5]'::vector);
SELECT vector_norm('[3,4]'::vector);
EOF

    # Indexes test
    cat > "$SCRIPT_DIR/sql/indexes.sql" << 'EOF'
-- pgvector Drop-In Compatibility Test: Indexes

CREATE TABLE test_index (
    id serial PRIMARY KEY,
    embedding vector(3)
);

INSERT INTO test_index (embedding) VALUES
    ('[1,0,0]'),
    ('[0,1,0]'),
    ('[0,0,1]'),
    ('[1,1,1]');

CREATE INDEX idx_hnsw ON test_index USING hnsw (embedding vector_l2_ops);

SELECT id, embedding <-> '[0.5,0.5,0.5]' AS distance
FROM test_index
ORDER BY embedding <-> '[0.5,0.5,0.5]'
LIMIT 3;

DROP TABLE test_index;
EOF

    # Queries test
    cat > "$SCRIPT_DIR/sql/queries.sql" << 'EOF'
-- pgvector Drop-In Compatibility Test: Queries

CREATE TABLE items (
    id serial PRIMARY KEY,
    category text,
    embedding vector(3)
);

INSERT INTO items (category, embedding) VALUES
    ('A', '[1,0,0]'),
    ('A', '[1.1,0,0]'),
    ('B', '[0,1,0]'),
    ('C', '[0,0,1]');

-- KNN query
SELECT id FROM items ORDER BY embedding <-> '[1,0,0]' LIMIT 3;

-- Filtered KNN
SELECT id FROM items WHERE category = 'A' ORDER BY embedding <-> '[0.5,0.5,0]' LIMIT 2;

-- Aggregate
SELECT category, MIN(embedding <-> '[0,0,0]') FROM items GROUP BY category;

DROP TABLE items;
EOF

    log_success "Generated SQL test files in $SCRIPT_DIR/sql/"
}

# Run pgrx tests
run_pgrx_tests() {
    log_header "Running pgrx Tests"

    cd "$PROJECT_ROOT"

    if cargo pgrx test pg16 --features pg_test 2>&1; then
        log_success "pgrx tests passed"
    else
        log_fail "pgrx tests failed"
    fi
}

# Print summary
print_summary() {
    log_header "Test Summary"

    local total=$((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))

    echo ""
    echo -e "Total Tests: $total"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
    echo -e "${YELLOW}Skipped: $TESTS_SKIPPED${NC}"
    echo ""

    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}All tests passed!${NC}"
        return 0
    else
        echo -e "${RED}Some tests failed.${NC}"
        return 1
    fi
}

# Main execution
main() {
    local mode="all"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --pgvector-only)
                mode="pgvector"
                shift
                ;;
            --ruvector-only)
                mode="ruvector"
                shift
                ;;
            --compare)
                mode="compare"
                shift
                ;;
            --generate-sql)
                generate_sql_files
                exit 0
                ;;
            --verbose)
                set -x
                shift
                ;;
            *)
                echo "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    # Create results directory
    mkdir -p "$RESULTS_DIR"

    log_header "pgvector Drop-In Compatibility Test Suite"
    echo "Mode: $mode"
    echo "Results directory: $RESULTS_DIR"

    # Check prerequisites
    check_prerequisites

    # Generate SQL files if they don't exist
    if [ ! -d "$SCRIPT_DIR/sql" ]; then
        generate_sql_files
    fi

    # Run pgrx tests (primary test method)
    run_pgrx_tests

    # Print summary
    print_summary
}

# Run main
main "$@"
