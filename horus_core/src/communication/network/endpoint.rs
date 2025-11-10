use std::net::IpAddr;

/// Endpoint types for HORUS network communication
#[derive(Debug, Clone, PartialEq)]
pub enum Endpoint {
    /// Local shared memory: "topic"
    Local { topic: String },

    /// Localhost (same machine): "topic@localhost"
    Localhost { topic: String, port: Option<u16> },

    /// Direct network connection: "topic@192.168.1.5" or "topic@192.168.1.5:9000"
    Direct { topic: String, host: IpAddr, port: u16 },

    /// Multicast discovery: "topic@*"
    Multicast { topic: String },

    /// Router (central broker): "topic@router" or "topic@router:7777"
    Router { topic: String, host: Option<IpAddr>, port: Option<u16> },
}

/// Default port for HORUS direct connections
pub const DEFAULT_PORT: u16 = 9870;

/// Default multicast address for discovery
pub const MULTICAST_ADDR: &str = "239.255.72.85";
pub const MULTICAST_PORT: u16 = 9871;

/// Parse endpoint string into Endpoint enum
///
/// # Format:
/// - `"topic"` → Local shared memory
/// - `"topic@localhost"` → Localhost (Unix socket or shared memory)
/// - `"topic@192.168.1.5"` → Direct network (default port 9870)
/// - `"topic@192.168.1.5:9000"` → Direct network (custom port)
/// - `"topic@*"` → Multicast discovery
pub fn parse_endpoint(input: &str) -> Result<Endpoint, String> {
    // Split on '@'
    if !input.contains('@') {
        return Ok(Endpoint::Local {
            topic: input.to_string(),
        });
    }

    let parts: Vec<&str> = input.split('@').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid endpoint format '{}': expected 'topic@location'",
            input
        ));
    }

    let topic = parts[0].to_string();
    let location = parts[1];

    if topic.is_empty() {
        return Err("Topic name cannot be empty".to_string());
    }

    // Check for multicast wildcard
    if location == "*" {
        return Ok(Endpoint::Multicast { topic });
    }

    // Check for router
    if location == "router" {
        return Ok(Endpoint::Router {
            topic,
            host: None,  // Use default localhost
            port: None,  // Use default 7777
        });
    }

    // Check for router with port: "router:7777"
    if location.starts_with("router:") {
        let port_str = &location[7..];
        let port = port_str
            .parse::<u16>()
            .map_err(|e| format!("Invalid router port '{}': {}", port_str, e))?;
        return Ok(Endpoint::Router {
            topic,
            host: None,  // Use default localhost
            port: Some(port),
        });
    }

    // Check for localhost
    if location == "localhost" || location == "127.0.0.1" || location == "::1" {
        return Ok(Endpoint::Localhost { topic, port: None });
    }

    // Parse host:port
    // Note: IPv6 addresses can contain ':' so we need special handling
    // IPv6 with port: [2001:db8::1]:9000
    // IPv6 without port: 2001:db8::1
    // IPv4 with port: 192.168.1.5:9000
    // IPv4 without port: 192.168.1.5

    if location.starts_with('[') {
        // IPv6 with brackets (and optional port)
        if let Some(bracket_end) = location.find(']') {
            let ipv6_str = &location[1..bracket_end];
            let host = ipv6_str
                .parse::<IpAddr>()
                .map_err(|e| format!("Invalid IPv6 address '{}': {}", ipv6_str, e))?;

            // Check if there's a port after the bracket
            if location.len() > bracket_end + 1 {
                if location.chars().nth(bracket_end + 1) == Some(':') {
                    let port_str = &location[bracket_end + 2..];
                    let port = port_str
                        .parse::<u16>()
                        .map_err(|e| format!("Invalid port '{}': {}", port_str, e))?;
                    return Ok(Endpoint::Direct { topic, host, port });
                } else {
                    return Err(format!("Invalid format after IPv6 address: '{}'", location));
                }
            }

            return Ok(Endpoint::Direct {
                topic,
                host,
                port: DEFAULT_PORT,
            });
        } else {
            return Err(format!("Missing closing bracket in IPv6 address '{}'", location));
        }
    }

    // Try to parse as IPv6 without brackets (no port)
    if let Ok(host) = location.parse::<IpAddr>() {
        return Ok(Endpoint::Direct {
            topic,
            host,
            port: DEFAULT_PORT,
        });
    }

    // Try IPv4 with port
    if let Some(colon_pos) = location.rfind(':') {
        let host_str = &location[..colon_pos];
        let port_str = &location[colon_pos + 1..];

        if let (Ok(host), Ok(port)) = (host_str.parse::<IpAddr>(), port_str.parse::<u16>()) {
            return Ok(Endpoint::Direct { topic, host, port });
        }
    }

    // Failed to parse
    Err(format!("Invalid IP address or host:port format '{}'", location))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_local() {
        let ep = parse_endpoint("mytopic").unwrap();
        assert_eq!(
            ep,
            Endpoint::Local {
                topic: "mytopic".to_string()
            }
        );
    }

    #[test]
    fn test_parse_local_with_underscores() {
        let ep = parse_endpoint("my_topic_name").unwrap();
        assert_eq!(
            ep,
            Endpoint::Local {
                topic: "my_topic_name".to_string()
            }
        );
    }

    #[test]
    fn test_parse_localhost() {
        let ep = parse_endpoint("mytopic@localhost").unwrap();
        assert_eq!(
            ep,
            Endpoint::Localhost {
                topic: "mytopic".to_string(),
                port: None
            }
        );
    }

    #[test]
    fn test_parse_localhost_ipv4() {
        let ep = parse_endpoint("mytopic@127.0.0.1").unwrap();
        assert_eq!(
            ep,
            Endpoint::Localhost {
                topic: "mytopic".to_string(),
                port: None
            }
        );
    }

    #[test]
    fn test_parse_localhost_ipv6() {
        let ep = parse_endpoint("mytopic@::1").unwrap();
        assert_eq!(
            ep,
            Endpoint::Localhost {
                topic: "mytopic".to_string(),
                port: None
            }
        );
    }

    #[test]
    fn test_parse_direct_default_port() {
        let ep = parse_endpoint("mytopic@192.168.1.5").unwrap();
        match ep {
            Endpoint::Direct { topic, host, port } => {
                assert_eq!(topic, "mytopic");
                assert_eq!(host, "192.168.1.5".parse::<IpAddr>().unwrap());
                assert_eq!(port, DEFAULT_PORT);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_parse_direct_custom_port() {
        let ep = parse_endpoint("mytopic@192.168.1.5:9000").unwrap();
        match ep {
            Endpoint::Direct { topic, host, port } => {
                assert_eq!(topic, "mytopic");
                assert_eq!(host, "192.168.1.5".parse::<IpAddr>().unwrap());
                assert_eq!(port, 9000);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_parse_direct_ipv6() {
        let ep = parse_endpoint("mytopic@2001:db8::1").unwrap();
        match ep {
            Endpoint::Direct { topic, host, port } => {
                assert_eq!(topic, "mytopic");
                assert_eq!(host, "2001:db8::1".parse::<IpAddr>().unwrap());
                assert_eq!(port, DEFAULT_PORT);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_parse_direct_ipv6_with_port() {
        let ep = parse_endpoint("mytopic@[2001:db8::1]:9000").unwrap();
        match ep {
            Endpoint::Direct { topic, host, port } => {
                assert_eq!(topic, "mytopic");
                assert_eq!(host, "2001:db8::1".parse::<IpAddr>().unwrap());
                assert_eq!(port, 9000);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_parse_multicast() {
        let ep = parse_endpoint("mytopic@*").unwrap();
        assert_eq!(
            ep,
            Endpoint::Multicast {
                topic: "mytopic".to_string()
            }
        );
    }

    #[test]
    fn test_parse_error_empty_topic() {
        let result = parse_endpoint("@192.168.1.5");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_parse_error_invalid_ip() {
        let result = parse_endpoint("mytopic@invalid.ip");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_invalid_port() {
        let result = parse_endpoint("mytopic@192.168.1.5:99999");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_multiple_at() {
        let result = parse_endpoint("mytopic@host@other");
        assert!(result.is_err());
    }
}
