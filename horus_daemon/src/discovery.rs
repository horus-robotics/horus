use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

const SERVICE_TYPE: &str = "_horus._tcp.local.";
const SERVICE_PORT: u16 = 8080;

pub struct DiscoveryService {
    daemon: Arc<ServiceDaemon>,
}

impl DiscoveryService {
    pub fn new() -> anyhow::Result<Self> {
        let daemon = ServiceDaemon::new()?;
        Ok(Self {
            daemon: Arc::new(daemon),
        })
    }

    /// Start broadcasting this robot's presence via mDNS
    pub fn start_broadcasting(&self, hostname: String) -> anyhow::Result<()> {
        // Get local IP addresses
        let local_ips = self.get_local_ips();

        if local_ips.is_empty() {
            tracing::warn!("No local IP addresses found for mDNS broadcasting");
            return Ok(());
        }

        let service_hostname = format!("{}.local.", hostname);

        // Create service info
        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &hostname,
            &service_hostname,
            &local_ips[0],
            SERVICE_PORT,
            None,
        )?;

        // Register the service
        self.daemon.register(service_info)?;

        tracing::info!("🔍 Broadcasting HORUS robot '{}' via mDNS on {}", hostname, local_ips[0]);

        Ok(())
    }

    fn get_local_ips(&self) -> Vec<IpAddr> {
        use std::net::{IpAddr, Ipv4Addr};

        // Try to get local IP addresses
        let mut ips = Vec::new();

        // Get network interfaces
        if let Ok(ifaces) = nix::ifaddrs::getifaddrs() {
            for iface in ifaces {
                if let Some(addr) = iface.address {
                    if let Some(sockaddr) = addr.as_sockaddr_in() {
                        let ip = sockaddr.ip();
                        // Skip localhost
                        if ip != 0x7f000001 {
                            ips.push(IpAddr::V4(Ipv4Addr::from(ip.to_be())));
                        }
                    }
                }
            }
        }

        // Fallback: try to get default interface IP
        if ips.is_empty() {
            if let Ok(hostname) = hostname::get() {
                if let Ok(hostname_str) = hostname.into_string() {
                    if let Ok(addrs) = std::net::ToSocketAddrs::to_socket_addrs(&(hostname_str.as_str(), 0)) {
                        for addr in addrs {
                            ips.push(addr.ip());
                        }
                    }
                }
            }
        }

        ips
    }

    /// Stop broadcasting (called on shutdown)
    pub fn stop(&self) {
        self.daemon.shutdown().ok();
    }
}
