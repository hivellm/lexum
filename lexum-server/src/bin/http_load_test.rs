//! HTTP load testing binary for Lexum REST API

use anyhow::Result;
use clap::{Arg, Command};
use lexum_server::http_load_test::{
    HttpLoadTestConfig, HttpLoadTestRunner, print_detailed_results,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let matches = Command::new("lexum-http-load-test")
        .version(env!("CARGO_PKG_VERSION"))
        .about("HTTP load testing tool for Lexum REST API")
        .arg(
            Arg::new("url")
                .short('u')
                .long("url")
                .value_name("URL")
                .help("Base URL of the Lexum server")
                .default_value("http://127.0.0.1:9200"),
        )
        .arg(
            Arg::new("clients")
                .short('c')
                .long("clients")
                .value_name("NUMBER")
                .help("Number of concurrent clients")
                .default_value("10"),
        )
        .arg(
            Arg::new("requests")
                .short('r')
                .long("requests")
                .value_name("NUMBER")
                .help("Number of requests per client")
                .default_value("100"),
        )
        .arg(
            Arg::new("delay")
                .short('d')
                .long("delay")
                .value_name("MS")
                .help("Delay between requests in milliseconds")
                .default_value("100"),
        )
        .arg(
            Arg::new("duration")
                .long("duration")
                .value_name("SECONDS")
                .help("Test duration in seconds")
                .default_value("60"),
        )
        .arg(
            Arg::new("api-key")
                .long("api-key")
                .value_name("KEY")
                .help("API key for authentication"),
        )
        .arg(
            Arg::new("suite")
                .long("suite")
                .help("Run the full test suite")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let base_url = matches.get_one::<String>("url").unwrap().clone();
    let concurrent_clients = matches
        .get_one::<String>("clients")
        .unwrap()
        .parse::<usize>()?;
    let requests_per_client = matches
        .get_one::<String>("requests")
        .unwrap()
        .parse::<usize>()?;
    let request_delay_ms = matches.get_one::<String>("delay").unwrap().parse::<u64>()?;
    let test_duration_secs = matches
        .get_one::<String>("duration")
        .unwrap()
        .parse::<u64>()?;
    let api_key = matches.get_one::<String>("api-key").cloned();
    let run_suite = matches.get_flag("suite");

    if run_suite {
        println!("Running HTTP load test suite...");
        let results = HttpLoadTestRunner::run_test_suite().await?;

        println!("\n=== HTTP Load Test Suite Results ===");
        for (name, result) in &results {
            print_detailed_results(name, result);
        }

        // Print summary
        println!("\n=== Summary ===");
        for (name, result) in &results {
            println!(
                "{}: {:.2} RPS, {:.2}ms avg, {:.2}% success",
                name,
                result.requests_per_second,
                result.avg_response_time_ms,
                if result.total_requests > 0 {
                    (result.successful_requests as f64 / result.total_requests as f64) * 100.0
                } else {
                    0.0
                }
            );
        }
    } else {
        let config = HttpLoadTestConfig {
            base_url,
            concurrent_clients,
            requests_per_client,
            request_delay_ms,
            test_duration_secs,
            index_name: "http_load_test_index".to_string(),
            api_key,
            test_type: lexum_server::http_load_test::TestType::Load,
            ramp_up_duration_secs: 10,
            ramp_down_duration_secs: 10,
            memory_profiling: false,
            cpu_profiling: false,
        };

        println!("Running single HTTP load test...");
        let result = HttpLoadTestRunner::run_test(config).await?;
        print_detailed_results("Single Test", &result);
    }

    Ok(())
}
