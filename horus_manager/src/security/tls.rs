//! TLS certificate management for HORUS dashboard
//!
//! Provides self-signed certificate generation and TLS configuration.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// TLS configuration
pub struct TlsConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

impl TlsConfig {
    /// Create TLS configuration with paths to certificate and key
    pub fn new(cert_path: PathBuf, key_path: PathBuf) -> Self {
        Self {
            cert_path,
            key_path,
        }
    }

    /// Get default TLS certificate paths in ~/.horus/certs/
    pub fn default_paths() -> Result<Self> {
        let cert_dir = dirs::home_dir()
            .context("Cannot find home directory")?
            .join(".horus")
            .join("certs");

        fs::create_dir_all(&cert_dir)?;

        Ok(Self {
            cert_path: cert_dir.join("cert.pem"),
            key_path: cert_dir.join("key.pem"),
        })
    }

    /// Check if certificates exist
    pub fn certificates_exist(&self) -> bool {
        self.cert_path.exists() && self.key_path.exists()
    }

    /// Generate self-signed certificate
    pub fn generate_self_signed_cert(&self, hostname: &str) -> Result<()> {
        use std::process::Command;

        println!("🔐 Generating self-signed TLS certificate...");
        println!("   Hostname: {}", hostname);

        // Use openssl to generate self-signed certificate
        let output = Command::new("openssl")
            .args(&[
                "req",
                "-x509",
                "-newkey",
                "rsa:4096",
                "-keyout",
                self.key_path.to_str().unwrap(),
                "-out",
                self.cert_path.to_str().unwrap(),
                "-days",
                "365",
                "-nodes",
                "-subj",
                &format!("/CN={}/O=HORUS/C=US", hostname),
            ])
            .output()
            .context("Failed to run openssl. Please install openssl.")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to generate certificate: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        println!(" Certificate generated successfully");
        println!("   Cert: {}", self.cert_path.display());
        println!("   Key:  {}", self.key_path.display());
        println!("     This is a self-signed certificate. Your browser will show a warning.");
        println!("     For production, use certificates from a trusted CA (Let's Encrypt).");

        Ok(())
    }

    /// Generate certificate if it doesn't exist
    pub fn ensure_certificates(&self, hostname: &str) -> Result<()> {
        if !self.certificates_exist() {
            self.generate_self_signed_cert(hostname)?;
        } else {
            println!(" Using existing TLS certificates");
            println!("   Cert: {}", self.cert_path.display());
            println!("   Key:  {}", self.key_path.display());
        }
        Ok(())
    }

    /// Validate certificate files are readable
    pub fn validate(&self) -> Result<()> {
        fs::read(&self.cert_path).context("Cannot read certificate file")?;
        fs::read(&self.key_path).context("Cannot read key file")?;
        Ok(())
    }
}

/// Instructions for using Let's Encrypt in production
pub fn print_letsencrypt_instructions() {
    println!("\n📖 Production TLS with Let's Encrypt:");
    println!("   1. Install certbot: sudo apt install certbot");
    println!("   2. Get certificate: sudo certbot certonly --standalone -d your-domain.com");
    println!("   3. Certificates will be in /etc/letsencrypt/live/your-domain.com/");
    println!("   4. Use --cert and --key flags to specify certificate paths");
    println!("   5. Set up auto-renewal: sudo certbot renew --dry-run\n");
}
