// Simple wildcard matching implementation
fn matches_wildcard(pattern: &str, name: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let name_chars: Vec<char> = name.chars().collect();
    
    matches_wildcard_recursive(&pattern_chars, &name_chars, 0, 0)
}

fn matches_wildcard_recursive(pattern: &[char], name: &[char], p_idx: usize, n_idx: usize) -> bool {
    // If we've consumed all pattern characters
    if p_idx >= pattern.len() {
        return n_idx >= name.len();
    }
    
    // If we've consumed all name characters but pattern has more
    if n_idx >= name.len() {
        // Only match if remaining pattern is all '*'
        return pattern[p_idx..].iter().all(|&c| c == '*');
    }
    
    match pattern[p_idx] {
        '*' => {
            // Try matching zero characters (skip the *)
            if matches_wildcard_recursive(pattern, name, p_idx + 1, n_idx) {
                return true;
            }
            // Try matching one or more characters
            if matches_wildcard_recursive(pattern, name, p_idx, n_idx + 1) {
                return true;
            }
            false
        }
        '?' => {
            // Match exactly one character - must have a character to match
            if n_idx < name.len() {
                matches_wildcard_recursive(pattern, name, p_idx + 1, n_idx + 1)
            } else {
                false
            }
        }
        _ => {
            // Match exact character
            if pattern[p_idx] == name[n_idx] {
                matches_wildcard_recursive(pattern, name, p_idx + 1, n_idx + 1)
            } else {
                false
            }
        }
    }
}

fn main() {
    println!("Testing wildcard matching:");
    println!("logs-? matches logs-1: {}", matches_wildcard("logs-?", "logs-1"));
    println!("logs-? matches logs-a: {}", matches_wildcard("logs-?", "logs-a"));
    println!("logs-? matches logs-12: {}", matches_wildcard("logs-?", "logs-12"));
    println!("logs-? matches logs: {}", matches_wildcard("logs-?", "logs"));
    
    println!("\nTesting * patterns:");
    println!("logs-* matches logs-1: {}", matches_wildcard("logs-*", "logs-1"));
    println!("logs-*-* matches logs-app-2024: {}", matches_wildcard("logs-*-*", "logs-app-2024"));
}
