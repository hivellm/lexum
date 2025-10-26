use lexum_core::index::template::IndexPattern;

fn main() {
    let pattern = IndexPattern::new("logs-?");
    println!("Pattern: logs-?");
    println!("Matches logs-1: {}", pattern.matches("logs-1"));
    println!("Matches logs-a: {}", pattern.matches("logs-a"));
    println!("Matches logs-12: {}", pattern.matches("logs-12"));
    println!("Matches logs: {}", pattern.matches("logs"));
}
