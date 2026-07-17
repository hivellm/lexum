//! Load testing binary for Lexum

use anyhow::Result;
use clap::Parser;
use lexum_core::IndexManager;
use lexum_server::load_test::{
    LoadTestConfig, LoadTestRunner, print_detailed_results, print_results,
};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "lexum-load-test")]
#[command(about = "Load testing tool for Lexum search engine")]
struct Args {
    /// Number of concurrent clients
    #[arg(short, long, default_value = "10")]
    clients: usize,

    /// Number of requests per client
    #[arg(short, long, default_value = "100")]
    requests: usize,

    /// Delay between requests (milliseconds)
    #[arg(long, default_value = "100")]
    delay: u64,

    /// Test duration (seconds)
    #[arg(short, long, default_value = "60")]
    duration: u64,

    /// Index name for testing
    #[arg(short, long, default_value = "load_test_index")]
    index: String,

    /// Run full test suite
    #[arg(long)]
    suite: bool,

    /// Data directory
    #[arg(long, default_value = "./data")]
    data_dir: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Starting Lexum Load Test");
    println!("========================");

    // Create index manager
    let index_manager = Arc::new(IndexManager::new(&args.data_dir));
    let runner = LoadTestRunner::new(index_manager);

    if args.suite {
        // Run full test suite
        println!("Running full test suite...");
        let results = runner.run_test_suite().await?;

        // Print summary
        let summary_results: Vec<_> = results.iter().map(|(_, result)| result.clone()).collect();
        print_results(&summary_results);

        // Print detailed results
        for (name, result) in &results {
            print_detailed_results(name, result);
        }
    } else {
        // Run single test
        let config = LoadTestConfig {
            concurrent_clients: args.clients,
            requests_per_client: args.requests,
            request_delay_ms: args.delay,
            test_duration_secs: args.duration,
            index_name: args.index,
        };

        println!("Configuration:");
        println!("  Concurrent Clients: {}", config.concurrent_clients);
        println!("  Requests per Client: {}", config.requests_per_client);
        println!("  Request Delay: {} ms", config.request_delay_ms);
        println!("  Test Duration: {} seconds", config.test_duration_secs);
        println!("  Index Name: {}", config.index_name);
        println!();

        let result = runner.run_test(config).await?;
        print_detailed_results("Load Test", &result);
    }

    println!("\nLoad test completed successfully!");
    Ok(())
}
