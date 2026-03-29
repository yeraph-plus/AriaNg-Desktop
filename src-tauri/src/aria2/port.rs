use std::net::TcpListener;

use crate::constants::PORT_SCAN_RANGE;

/// Check if a specific port is available on 127.0.0.1
pub fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Find an available port starting from the preferred port.
/// Scans upward within PORT_SCAN_RANGE if the preferred port is occupied.
pub fn find_available_port(preferred: u16) -> Result<u16, String> {
    let end = preferred.saturating_add(PORT_SCAN_RANGE);
    for port in preferred..end {
        if is_port_available(port) {
            return Ok(port);
        }
    }
    Err(format!(
        "No available port found in range {}-{}",
        preferred,
        end - 1
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::DEFAULT_RPC_PORT;

    #[test]
    fn test_find_available_port() {
        let port = find_available_port(DEFAULT_RPC_PORT);
        assert!(port.is_ok());
    }

    #[test]
    fn test_is_port_available() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(!is_port_available(port));
        drop(listener);
        assert!(is_port_available(port));
    }
}
