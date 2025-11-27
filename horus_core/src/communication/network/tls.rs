//! TLS certificate management and utilities for HORUS
//!
//! Provides self-signed certificate generation, certificate loading,
//! and TLS configuration for secure network communication.

#[cfg(feature = "tls")]
use crate::error::{HorusError, HorusResult};
#[cfg(feature = "tls")]
use std::path::Path;
#[cfg(feature = "tls")]
use std::sync::Arc;

#[cfg(feature = "tls")]
use rcgen::{CertificateParams, DistinguishedName};
#[cfg(feature = "tls")]
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
#[cfg(feature = "tls")]
use rustls::ServerConfig;
#[cfg(feature = "tls")]
use tokio_rustls::TlsAcceptor;

// Re-export TlsStream for use by other modules
#[cfg(feature = "tls")]
pub use tokio_rustls::server::TlsStream;

#[cfg(feature = "tls")]
/// TLS certificate configuration
#[derive(Debug, Clone)]
pub struct TlsCertConfig {
    /// Path to certificate PEM file
    pub cert_path: Option<String>,
    /// Path to private key PEM file
    pub key_path: Option<String>,
    /// Auto-generate self-signed certificate if no paths provided
    pub auto_generate: bool,
    /// Organization name for generated certificates
    pub organization: String,
    /// Common name for generated certificates
    pub common_name: String,
}

#[cfg(feature = "tls")]
impl Default for TlsCertConfig {
    fn default() -> Self {
        Self {
            cert_path: None,
            key_path: None,
            auto_generate: true,
            organization: "HORUS Robotics".to_string(),
            common_name: "horus-router".to_string(),
        }
    }
}

#[cfg(feature = "tls")]
impl TlsCertConfig {
    /// Create a new TLS certificate configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set certificate and key file paths
    pub fn with_files(mut self, cert_path: impl Into<String>, key_path: impl Into<String>) -> Self {
        self.cert_path = Some(cert_path.into());
        self.key_path = Some(key_path.into());
        self.auto_generate = false;
        self
    }

    /// Enable auto-generation of self-signed certificates
    pub fn with_auto_generate(mut self) -> Self {
        self.auto_generate = true;
        self
    }

    /// Set organization name for generated certificates
    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.organization = org.into();
        self
    }

    /// Set common name for generated certificates
    pub fn with_common_name(mut self, cn: impl Into<String>) -> Self {
        self.common_name = cn.into();
        self
    }

    /// Load or generate TLS certificate and private key
    pub fn load_or_generate(
        &self,
    ) -> HorusResult<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        // If paths are provided, load from files
        if let (Some(cert_path), Some(key_path)) = (&self.cert_path, &self.key_path) {
            return self.load_from_files(cert_path, key_path);
        }

        // Auto-generate if enabled
        if self.auto_generate {
            return self.generate_self_signed();
        }

        Err(HorusError::config(
            "TLS enabled but no certificate paths provided and auto-generate is disabled",
        ))
    }

    /// Load certificate and key from PEM files
    fn load_from_files(
        &self,
        cert_path: &str,
        key_path: &str,
    ) -> HorusResult<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        // Load certificate
        let cert_file = std::fs::File::open(cert_path)
            .map_err(|e| HorusError::config(format!("Failed to open certificate file: {}", e)))?;
        let mut cert_reader = std::io::BufReader::new(cert_file);
        let certs = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| HorusError::config(format!("Failed to parse certificate: {}", e)))?;

        if certs.is_empty() {
            return Err(HorusError::config(
                "No certificates found in certificate file",
            ));
        }

        // Load private key
        let key_file = std::fs::File::open(key_path)
            .map_err(|e| HorusError::config(format!("Failed to open private key file: {}", e)))?;
        let mut key_reader = std::io::BufReader::new(key_file);
        let key = rustls_pemfile::private_key(&mut key_reader)
            .map_err(|e| HorusError::config(format!("Failed to parse private key: {}", e)))?
            .ok_or_else(|| HorusError::config("No private key found in key file"))?;

        Ok((certs, key))
    }

    /// Generate a self-signed certificate
    ///
    /// # Security Warning
    ///
    /// Self-signed certificates provide encryption but NOT authentication.
    /// They are suitable for:
    /// - Development and testing
    /// - Private/isolated networks
    /// - Lab environments
    ///
    /// They are NOT suitable for:
    /// - Production systems on untrusted networks
    /// - Internet-facing services
    /// - Scenarios requiring server identity verification
    ///
    /// For production use, obtain certificates from a trusted CA and use
    /// `with_files()` to load them.
    fn generate_self_signed(
        &self,
    ) -> HorusResult<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        use rcgen::Ia5String;

        // Create certificate parameters
        let mut params = CertificateParams::default();

        // Set distinguished name
        let mut dn = DistinguishedName::new();
        dn.push(rcgen::DnType::OrganizationName, &self.organization);
        dn.push(rcgen::DnType::CommonName, &self.common_name);
        params.distinguished_name = dn;

        // Add localhost and common IPs as subject alternative names
        let localhost_ia5 = Ia5String::try_from("localhost".to_string())
            .map_err(|e| HorusError::config(format!("Failed to create IA5String: {:?}", e)))?;
        params.subject_alt_names = vec![
            rcgen::SanType::DnsName(localhost_ia5),
            rcgen::SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
            rcgen::SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::new(
                0, 0, 0, 0, 0, 0, 0, 1,
            ))),
        ];

        // Generate key pair first
        let key_pair = rcgen::KeyPair::generate()
            .map_err(|e| HorusError::config(format!("Failed to generate key pair: {}", e)))?;

        // Generate certificate using self_signed
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| HorusError::config(format!("Failed to generate certificate: {}", e)))?;

        // Get PEM-encoded certificate
        let cert_pem = cert.pem();

        // Get private key PEM
        let key_pem = key_pair.serialize_pem();

        // Parse into rustls types
        let cert_der = rustls_pemfile::certs(&mut cert_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                HorusError::config(format!("Failed to parse generated certificate: {}", e))
            })?;

        let key_der = rustls_pemfile::private_key(&mut key_pem.as_bytes())
            .map_err(|e| {
                HorusError::config(format!("Failed to parse generated private key: {}", e))
            })?
            .ok_or_else(|| HorusError::config("Failed to extract private key"))?;

        log::warn!(
            "Generated self-signed TLS certificate for {}",
            self.common_name
        );
        log::warn!("Self-signed certificates provide encryption but NOT authentication");
        log::warn!(
            "Suitable for dev/testing/private networks only - use CA-signed certs for production"
        );

        Ok((cert_der, key_der))
    }

    /// Create a TLS acceptor for server use
    pub fn create_acceptor(&self) -> HorusResult<TlsAcceptor> {
        let (certs, key) = self.load_or_generate()?;

        // Create server config
        let mut config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| HorusError::config(format!("Failed to create TLS config: {}", e)))?;

        // Enable ALPN for HTTP/2 and HTTP/1.1 (useful for future extensions)
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Ok(TlsAcceptor::from(Arc::new(config)))
    }
}

#[cfg(feature = "tls")]
/// Save certificate and key to PEM files
pub fn save_cert_to_files<P: AsRef<Path>>(
    cert_path: P,
    key_path: P,
    cert_pem: &str,
    key_pem: &str,
) -> HorusResult<()> {
    std::fs::write(&cert_path, cert_pem)
        .map_err(|e| HorusError::config(format!("Failed to write certificate file: {}", e)))?;

    std::fs::write(&key_path, key_pem)
        .map_err(|e| HorusError::config(format!("Failed to write key file: {}", e)))?;

    log::info!("Saved TLS certificate and key to files");
    Ok(())
}

#[cfg(test)]
#[cfg(feature = "tls")]
mod tests {
    use super::*;

    #[test]
    fn test_generate_self_signed() {
        let config = TlsCertConfig::default();
        let result = config.generate_self_signed();
        assert!(result.is_ok());

        let (certs, _key) = result.unwrap();
        assert!(!certs.is_empty());
    }

    #[test]
    fn test_create_acceptor() {
        let config = TlsCertConfig::default();
        let result = config.create_acceptor();
        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_config() {
        let config = TlsCertConfig::new()
            .with_organization("Test Org")
            .with_common_name("test-server");

        assert_eq!(config.organization, "Test Org");
        assert_eq!(config.common_name, "test-server");
    }
}
