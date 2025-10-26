#!/bin/bash
# Comprehensive benchmark runner for Lexum

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BENCHMARK_DIR="benchmark"
RESULTS_DIR="benchmark_results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
RESULTS_FILE="${RESULTS_DIR}/benchmark_${TIMESTAMP}.json"

# Create results directory
mkdir -p "$RESULTS_DIR"

echo -e "${BLUE}=== LEXUM COMPREHENSIVE BENCHMARK SUITE ===${NC}"
echo "Timestamp: $(date)"
echo "Results will be saved to: $RESULTS_FILE"
echo ""

# Function to run benchmarks
run_benchmarks() {
    local benchmark_name="$1"
    local benchmark_file="$2"
    local description="$3"
    
    echo -e "${YELLOW}Running $description...${NC}"
    
    # Run the benchmark
    cargo bench --bench "$benchmark_name" -- --output-format json > "${RESULTS_DIR}/${benchmark_name}_${TIMESTAMP}.json" 2>&1
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ $description completed successfully${NC}"
    else
        echo -e "${RED}✗ $description failed${NC}"
        return 1
    fi
}

# Function to generate summary report
generate_summary() {
    echo -e "${BLUE}=== GENERATING SUMMARY REPORT ===${NC}"
    
    # Create a summary script
    cat > "${RESULTS_DIR}/generate_summary.py" << 'EOF'
#!/usr/bin/env python3
import json
import os
import glob
from datetime import datetime

def load_benchmark_results(results_dir, timestamp):
    """Load all benchmark results for a given timestamp"""
    results = {}
    
    for file_path in glob.glob(f"{results_dir}/*_{timestamp}.json"):
        benchmark_name = os.path.basename(file_path).replace(f"_{timestamp}.json", "")
        
        try:
            with open(file_path, 'r') as f:
                data = json.load(f)
                results[benchmark_name] = data
        except Exception as e:
            print(f"Error loading {file_path}: {e}")
    
    return results

def generate_summary_report(results, output_file):
    """Generate a human-readable summary report"""
    report = []
    report.append("# Lexum Benchmark Results Summary")
    report.append(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    report.append("")
    
    for benchmark_name, data in results.items():
        report.append(f"## {benchmark_name}")
        report.append("")
        
        if 'benchmarks' in data:
            for benchmark in data['benchmarks']:
                name = benchmark.get('name', 'Unknown')
                mean_time = benchmark.get('mean', {}).get('point_estimate', 0)
                std_dev = benchmark.get('mean', {}).get('standard_error', 0)
                
                report.append(f"### {name}")
                report.append(f"- Mean Time: {mean_time:.2e} seconds")
                report.append(f"- Std Dev: {std_dev:.2e} seconds")
                report.append("")
    
    with open(output_file, 'w') as f:
        f.write('\n'.join(report))
    
    print(f"Summary report generated: {output_file}")

if __name__ == "__main__":
    import sys
    
    if len(sys.argv) != 3:
        print("Usage: python3 generate_summary.py <results_dir> <timestamp>")
        sys.exit(1)
    
    results_dir = sys.argv[1]
    timestamp = sys.argv[2]
    
    results = load_benchmark_results(results_dir, timestamp)
    generate_summary_report(results, f"{results_dir}/summary_{timestamp}.md")
EOF

    chmod +x "${RESULTS_DIR}/generate_summary.py"
    
    # Generate summary
    python3 "${RESULTS_DIR}/generate_summary.py" "$RESULTS_DIR" "$TIMESTAMP"
    
    echo -e "${GREEN}✓ Summary report generated: ${RESULTS_DIR}/summary_${TIMESTAMP}.md${NC}"
}

# Function to run performance tests
run_performance_tests() {
    echo -e "${BLUE}=== RUNNING PERFORMANCE TESTS ===${NC}"
    
    # Test 1: Basic search performance
    run_benchmarks "search_benchmarks" "search_benchmarks.rs" "Basic Search Performance"
    
    # Test 2: Comprehensive performance suite
    run_benchmarks "comprehensive_benchmarks" "comprehensive_benchmarks.rs" "Comprehensive Performance Suite"
    
    # Test 3: Memory usage tests
    echo -e "${YELLOW}Running memory usage tests...${NC}"
    cargo test --package lexum-core --lib performance::tests --release
    
    # Test 4: Load testing
    echo -e "${YELLOW}Running load tests...${NC}"
    cargo test --package lexum-server --test load_test --release
}

# Function to run regression tests
run_regression_tests() {
    echo -e "${BLUE}=== RUNNING REGRESSION TESTS ===${NC}"
    
    # Compare with baseline if it exists
    if [ -f "${RESULTS_DIR}/baseline.json" ]; then
        echo -e "${YELLOW}Comparing with baseline...${NC}"
        # This would implement comparison logic
        echo "Baseline comparison not yet implemented"
    else
        echo -e "${YELLOW}No baseline found, creating new baseline...${NC}"
        # Save current results as baseline
        cp "${RESULTS_FILE}" "${RESULTS_DIR}/baseline.json"
    fi
}

# Function to run stress tests
run_stress_tests() {
    echo -e "${BLUE}=== RUNNING STRESS TESTS ===${NC}"
    
    # Test with high document counts
    echo -e "${YELLOW}Testing with 100,000 documents...${NC}"
    BENCHMARK_DOC_COUNT=100000 cargo bench --bench comprehensive_benchmarks
    
    # Test with concurrent operations
    echo -e "${YELLOW}Testing concurrent operations...${NC}"
    BENCHMARK_CONCURRENCY=16 cargo bench --bench comprehensive_benchmarks
    
    # Test memory limits
    echo -e "${YELLOW}Testing memory limits...${NC}"
    BENCHMARK_MEMORY_LIMIT=1GB cargo bench --bench comprehensive_benchmarks
}

# Function to run specific benchmark categories
run_category() {
    local category="$1"
    
    case "$category" in
        "search")
            run_benchmarks "search_benchmarks" "search_benchmarks.rs" "Search Performance"
            ;;
        "comprehensive")
            run_benchmarks "comprehensive_benchmarks" "comprehensive_benchmarks.rs" "Comprehensive Performance"
            ;;
        "memory")
            echo -e "${YELLOW}Running memory benchmarks...${NC}"
            cargo bench --bench comprehensive_benchmarks -- --bench memory_usage
            ;;
        "concurrent")
            echo -e "${YELLOW}Running concurrency benchmarks...${NC}"
            cargo bench --bench comprehensive_benchmarks -- --bench concurrent_operations
            ;;
        "all")
            run_performance_tests
            run_regression_tests
            run_stress_tests
            ;;
        *)
            echo -e "${RED}Unknown category: $category${NC}"
            echo "Available categories: search, comprehensive, memory, concurrent, all"
            exit 1
            ;;
    esac
}

# Function to setup environment
setup_environment() {
    echo -e "${BLUE}=== SETTING UP BENCHMARK ENVIRONMENT ===${NC}"
    
    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ]; then
        echo -e "${RED}Error: Not in project root directory${NC}"
        exit 1
    fi
    
    # Check if benchmarks exist
    if [ ! -d "$BENCHMARK_DIR" ]; then
        echo -e "${RED}Error: Benchmarks directory not found${NC}"
        exit 1
    fi
    
    # Build in release mode for accurate benchmarks
    echo -e "${YELLOW}Building in release mode...${NC}"
    cargo build --release --workspace
    
    # Check if build was successful
    if [ $? -ne 0 ]; then
        echo -e "${RED}Error: Build failed${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ Environment setup complete${NC}"
}

# Function to cleanup
cleanup() {
    echo -e "${BLUE}=== CLEANUP ===${NC}"
    
    # Remove temporary files
    rm -f "${RESULTS_DIR}/generate_summary.py"
    
    # Compress results
    echo -e "${YELLOW}Compressing results...${NC}"
    tar -czf "${RESULTS_DIR}/benchmark_${TIMESTAMP}.tar.gz" -C "$RESULTS_DIR" .
    
    echo -e "${GREEN}✓ Cleanup complete${NC}"
    echo -e "${GREEN}✓ Results compressed: ${RESULTS_DIR}/benchmark_${TIMESTAMP}.tar.gz${NC}"
}

# Main execution
main() {
    local category="${1:-all}"
    
    setup_environment
    run_category "$category"
    generate_summary
    cleanup
    
    echo ""
    echo -e "${GREEN}=== BENCHMARK SUITE COMPLETED ===${NC}"
    echo -e "Results saved to: ${RESULTS_DIR}/"
    echo -e "Summary report: ${RESULTS_DIR}/summary_${TIMESTAMP}.md"
    echo -e "Compressed archive: ${RESULTS_DIR}/benchmark_${TIMESTAMP}.tar.gz"
}

# Run main function with all arguments
main "$@"