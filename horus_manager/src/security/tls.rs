//! TLS certificate management for HORUS dashboard
//!
//! Provides certificate generation using mkcert (trusted) or self-signed fallback.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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

    /// Check if mkcert is installed
    pub fn is_mkcert_installed() -> bool {
        Command::new("mkcert")
            .arg("-version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Check if mkcert CA is installed (user has run mkcert -install)
    pub fn is_mkcert_ca_installed() -> bool {
        // mkcert -CAROOT returns the CA directory if installed
        Command::new("mkcert")
            .arg("-CAROOT")
            .output()
            .map(|output| output.status.success() && !output.stdout.is_empty())
            .unwrap_or(false)
    }

    /// Get local network IP address for certificate
    fn get_local_ip() -> Option<String> {
        use std::net::UdpSocket;

        // Connect to a public DNS server to determine local IP
        // This doesn't actually send any data
        UdpSocket::bind("0.0.0.0:0")
            .ok()?
            .connect("8.8.8.8:80")
            .ok()?;

        let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.connect("8.8.8.8:80").ok()?;
        socket.local_addr().ok().map(|addr| addr.ip().to_string())
    }

    /// Generate trusted certificate using mkcert
    pub fn generate_mkcert_cert(&self) -> Result<()> {
        if !Self::is_mkcert_installed() {
            anyhow::bail!("mkcert is not installed");
        }

        if !Self::is_mkcert_ca_installed() {
            anyhow::bail!("mkcert CA is not installed. Run 'mkcert -install' first.");
        }

        println!("Generating trusted TLS certificate using mkcert...");

        // Build list of hostnames/IPs to include
        let mut hosts = vec![
            "localhost".to_string(),
            "127.0.0.1".to_string(),
            "::1".to_string(),
        ];

        // Add local network IP if available
        if let Some(local_ip) = Self::get_local_ip() {
            if local_ip != "127.0.0.1" {
                println!("   Including local network IP: {}", local_ip);
                hosts.push(local_ip);
            }
        }

        // Generate certificate
        let output = Command::new("mkcert")
            .arg("-cert-file")
            .arg(&self.cert_path)
            .arg("-key-file")
            .arg(&self.key_path)
            .args(&hosts)
            .output()
            .context("Failed to run mkcert")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to generate certificate with mkcert: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        println!("Trusted certificate generated successfully!");
        println!("   Cert: {}", self.cert_path.display());
        println!("   Key:  {}", self.key_path.display());
        println!("   Hostnames: {}", hosts.join(", "));
        println!();
        println!("No browser warnings - certificate is trusted by your system!");

        Ok(())
    }

    /// Generate self-signed certificate
    pub fn generate_self_signed_cert(&self, hostname: &str) -> Result<()> {
        println!("Generating self-signed TLS certificate...");
        println!("   Hostname: {}", hostname);

        // Use openssl to generate self-signed certificate
        let output = Command::new("openssl")
            .args([
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

    /// Generate certificate if it doesn't exist (tries mkcert first, falls back to self-signed)
    pub fn ensure_certificates(&self, hostname: &str) -> Result<()> {
        if !self.certificates_exist() {
            // Try mkcert first
            if Self::is_mkcert_installed() && Self::is_mkcert_ca_installed() {
                if let Err(e) = self.generate_mkcert_cert() {
                    eprintln!("WARNING: mkcert failed: {}", e);
                    eprintln!("   Falling back to self-signed certificate...\n");
                    self.generate_self_signed_cert(hostname)?;
                }
            } else {
                // Fall back to self-signed
                self.generate_self_signed_cert(hostname)?;
                println!();
                print_mkcert_instructions();
            }
        } else {
            println!(" Using existing TLS certificates");
            println!("   Cert: {}", self.cert_path.display());
            println!("   Key:  {}", self.key_path.display());
        }
        Ok(())
    }

    /// Setup certificates with mkcert (interactive)
    pub fn setup_trusted_certificates(&self) -> Result<()> {
        println!("Setting up trusted TLS certificates for HORUS dashboard\n");

        // Check if mkcert is installed
        if !Self::is_mkcert_installed() {
            println!("WARNING: mkcert is not installed");
            print_mkcert_installation();
            anyhow::bail!("Please install mkcert and run 'horus init --setup-certs' again");
        }

        println!("mkcert is installed");

        // Check if CA is installed
        if !Self::is_mkcert_ca_installed() {
            println!("WARNING: mkcert CA is not installed");
            println!("\nInstalling mkcert CA (will ask for sudo password)...");
            println!("   This is a one-time setup per device\n");

            let status = Command::new("mkcert")
                .arg("-install")
                .status()
                .context("Failed to run 'mkcert -install'")?;

            if !status.success() {
                anyhow::bail!("Failed to install mkcert CA");
            }

            println!("\nmkcert CA installed successfully!");
        } else {
            println!("mkcert CA is already installed");
        }

        // Remove existing certificates if any
        if self.certificates_exist() {
            println!("\nRemoving existing certificates...");
            let _ = fs::remove_file(&self.cert_path);
            let _ = fs::remove_file(&self.key_path);
        }

        // Generate new certificates
        println!();
        self.generate_mkcert_cert()?;

        println!("\nCertificate setup complete!");
        println!("   Your HORUS dashboard will now be trusted by all browsers");
        println!("   on this device with no security warnings!\n");

        Ok(())
    }

    /// Validate certificate files are readable
    pub fn validate(&self) -> Result<()> {
        fs::read(&self.cert_path).context("Cannot read certificate file")?;
        fs::read(&self.key_path).context("Cannot read key file")?;
        Ok(())
    }
}

/// Print instructions for installing mkcert
pub fn print_mkcert_installation() {
    println!("\n[INFO] Install mkcert for trusted certificates:\n");

    #[cfg(target_os = "linux")]
    {
        println!("   Option 1 - Using package manager:");
        println!("   $ sudo apt install mkcert   # Ubuntu/Debian");
        println!("   $ sudo dnf install mkcert   # Fedora");
        println!("   $ sudo pacman -S mkcert     # Arch Linux");
        println!();
        println!("   Option 2 - Download binary:");
        println!("   $ wget https://github.com/FiloSottile/mkcert/releases/download/v1.4.4/mkcert-v1.4.4-linux-amd64");
        println!("   $ chmod +x mkcert-v1.4.4-linux-amd64");
        println!("   $ sudo mv mkcert-v1.4.4-linux-amd64 /usr/local/bin/mkcert");
    }

    #[cfg(target_os = "macos")]
    {
        println!("   Using Homebrew:");
        println!("   $ brew install mkcert");
        println!("   $ brew install nss  # For Firefox");
    }

    #[cfg(target_os = "windows")]
    {
        println!("   Using Chocolatey:");
        println!("   > choco install mkcert");
        println!();
        println!("   Or using Scoop:");
        println!("   > scoop bucket add extras");
        println!("   > scoop install mkcert");
    }

    println!();
    println!("   After installation, run:");
    println!("   $ horus init --setup-certs");
    println!();
}

/// Print instructions for setting up mkcert
pub fn print_mkcert_instructions() {
    println!("WARNING: Using self-signed certificate - browsers will show security warnings");
    println!();
    println!("For trusted certificates (no browser warnings), install mkcert:");
    println!("   1. Install mkcert:");

    #[cfg(target_os = "linux")]
    println!("      $ sudo apt install mkcert");

    #[cfg(target_os = "macos")]
    println!("      $ brew install mkcert");

    #[cfg(target_os = "windows")]
    println!("      > choco install mkcert");

    println!("   2. Setup trusted certificates:");
    println!("      $ horus init --setup-certs");
    println!();
    println!("   This is a one-time setup that works for all HORUS projects!");
    println!();
}

/// Instructions for using Let's Encrypt in production
pub fn print_letsencrypt_instructions() {
    println!("\n[INFO] Production TLS with Let's Encrypt:");
    println!("   1. Install certbot: sudo apt install certbot");
    println!("   2. Get certificate: sudo certbot certonly --standalone -d your-domain.com");
    println!("   3. Certificates will be in /etc/letsencrypt/live/your-domain.com/");
    println!("   4. Use --cert and --key flags to specify certificate paths");
    println!("   5. Set up auto-renewal: sudo certbot renew --dry-run\n");
}
