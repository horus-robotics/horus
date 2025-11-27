//! QUIC transport for reliable, low-latency network communication
//!
//! QUIC provides:
//! - 0-RTT connection resumption (no handshake latency for repeat connections)
//! - No head-of-line blocking (streams are independent)
//! - Built-in encryption (TLS 1.3)
//! - Connection migration (handles IP changes)
//! - Better congestion control
//!
//! This is ideal for WAN communication or when reliability is required.
//!
//! Requires the `quic` feature to be enabled.

#[cfg(feature = "quic")]
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, TransportConfig};

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "quic")]
use tokio::sync::RwLock;

/// QUIC transport configuration
#[derive(Debug, Clone)]
pub struct QuicConfig {
    /// Maximum idle timeout before connection is closed
    pub max_idle_timeout: Duration,
    /// Keep-alive interval
    pub keep_alive_interval: Duration,
    /// Maximum concurrent bidirectional streams
    pub max_concurrent_bidi_streams: u32,
    /// Maximum concurrent unidirectional streams
    pub max_concurrent_uni_streams: u32,
    /// Enable 0-RTT (faster but less secure for first bytes)
    pub enable_0rtt: bool,
    /// Initial RTT estimate (for congestion control)
    pub initial_rtt: Duration,
    /// Maximum UDP payload size
    pub max_udp_payload_size: u16,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            max_idle_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(5),
            max_concurrent_bidi_streams: 100,
            max_concurrent_uni_streams: 100,
            enable_0rtt: true,
            initial_rtt: Duration::from_millis(10),
            max_udp_payload_size: 1472, // Standard MTU - headers
        }
    }
}

impl QuicConfig {
    /// Low latency configuration
    pub fn low_latency() -> Self {
        Self {
            max_idle_timeout: Duration::from_secs(60),
            keep_alive_interval: Duration::from_secs(2),
            max_concurrent_bidi_streams: 256,
            max_concurrent_uni_streams: 256,
            enable_0rtt: true,
            initial_rtt: Duration::from_millis(5),
            max_udp_payload_size: 1472,
        }
    }

    /// High throughput configuration
    pub fn high_throughput() -> Self {
        Self {
            max_idle_timeout: Duration::from_secs(120),
            keep_alive_interval: Duration::from_secs(10),
            max_concurrent_bidi_streams: 1000,
            max_concurrent_uni_streams: 1000,
            enable_0rtt: true,
            initial_rtt: Duration::from_millis(20),
            max_udp_payload_size: 65527, // Jumbo frames
        }
    }
}

/// Statistics for QUIC transport
#[derive(Debug, Default)]
pub struct QuicStats {
    pub connections_established: AtomicU64,
    pub connections_closed: AtomicU64,
    pub streams_opened: AtomicU64,
    pub streams_closed: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub messages_sent: AtomicU64,
    pub messages_received: AtomicU64,
    pub zero_rtt_accepted: AtomicU64,
    pub zero_rtt_rejected: AtomicU64,
}

/// QUIC transport backend
#[cfg(feature = "quic")]
pub struct QuicTransport {
    /// Local endpoint
    endpoint: Endpoint,
    /// Cached connections by remote address
    connections: Arc<RwLock<HashMap<SocketAddr, Connection>>>,
    /// Configuration (kept for future use)
    #[allow(dead_code)]
    config: QuicConfig,
    /// Statistics
    stats: Arc<QuicStats>,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Server name for TLS
    server_name: String,
}

#[cfg(feature = "quic")]
impl QuicTransport {
    /// Create a new QUIC client (outbound connections only)
    pub async fn new_client(bind_addr: SocketAddr, config: QuicConfig) -> io::Result<Self> {
        let client_config = Self::create_client_config(&config)?;

        let mut endpoint =
            Endpoint::client(bind_addr).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        endpoint.set_default_client_config(client_config);

        Ok(Self {
            endpoint,
            connections: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(QuicStats::default()),
            running: Arc::new(AtomicBool::new(true)),
            server_name: "horus".to_string(),
        })
    }

    /// Create a new QUIC server (accepts inbound connections)
    pub async fn new_server(
        bind_addr: SocketAddr,
        cert_chain: Vec<rustls::pki_types::CertificateDer<'static>>,
        private_key: rustls::pki_types::PrivateKeyDer<'static>,
        config: QuicConfig,
    ) -> io::Result<Self> {
        let server_config = Self::create_server_config(cert_chain, private_key, &config)?;

        let endpoint = Endpoint::server(server_config, bind_addr)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(Self {
            endpoint,
            connections: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(QuicStats::default()),
            running: Arc::new(AtomicBool::new(true)),
            server_name: "horus".to_string(),
        })
    }

    /// Create client TLS configuration
    fn create_client_config(config: &QuicConfig) -> io::Result<ClientConfig> {
        // Create a client config that accepts any certificate (for development)
        // In production, you'd want proper certificate validation
        let crypto = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();

        let mut transport = TransportConfig::default();
        transport.max_idle_timeout(Some(config.max_idle_timeout.try_into().unwrap_or_default()));
        transport.keep_alive_interval(Some(config.keep_alive_interval));
        transport.initial_rtt(config.initial_rtt);

        let mut client_config = ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(crypto)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
        ));
        client_config.transport_config(Arc::new(transport));

        Ok(client_config)
    }

    /// Create server TLS configuration
    fn create_server_config(
        cert_chain: Vec<rustls::pki_types::CertificateDer<'static>>,
        private_key: rustls::pki_types::PrivateKeyDer<'static>,
        config: &QuicConfig,
    ) -> io::Result<ServerConfig> {
        let mut transport = TransportConfig::default();
        transport.max_idle_timeout(Some(config.max_idle_timeout.try_into().unwrap_or_default()));
        transport.keep_alive_interval(Some(config.keep_alive_interval));
        transport.initial_rtt(config.initial_rtt);

        let mut server_config = ServerConfig::with_single_cert(cert_chain, private_key)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        server_config.transport_config(Arc::new(transport));

        Ok(server_config)
    }

    /// Get or create a connection to a remote address
    pub async fn get_connection(&self, addr: SocketAddr) -> io::Result<Connection> {
        // Check cache first
        {
            let conns = self.connections.read().await;
            if let Some(conn) = conns.get(&addr) {
                if conn.close_reason().is_none() {
                    return Ok(conn.clone());
                }
            }
        }

        // Create new connection
        let connecting = self
            .endpoint
            .connect(addr, &self.server_name)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let connection = connecting
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e))?;

        self.stats
            .connections_established
            .fetch_add(1, Ordering::Relaxed);

        // Cache the connection
        {
            let mut conns = self.connections.write().await;
            conns.insert(addr, connection.clone());
        }

        Ok(connection)
    }

    /// Send data to a remote address (unidirectional stream)
    pub async fn send(&self, addr: SocketAddr, data: &[u8]) -> io::Result<()> {
        let conn = self.get_connection(addr).await?;

        let mut stream = conn
            .open_uni()
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.stats.streams_opened.fetch_add(1, Ordering::Relaxed);

        // Write length prefix + data
        let len = (data.len() as u32).to_le_bytes();
        stream
            .write_all(&len)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        stream
            .write_all(data)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        stream
            .finish()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.stats
            .bytes_sent
            .fetch_add(data.len() as u64 + 4, Ordering::Relaxed);
        self.stats.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.stats.streams_closed.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Send data and wait for response (bidirectional stream)
    pub async fn send_recv(&self, addr: SocketAddr, data: &[u8]) -> io::Result<Vec<u8>> {
        let conn = self.get_connection(addr).await?;

        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.stats.streams_opened.fetch_add(1, Ordering::Relaxed);

        // Send request
        let len = (data.len() as u32).to_le_bytes();
        send.write_all(&len)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        send.write_all(data)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        send.finish()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.stats
            .bytes_sent
            .fetch_add(data.len() as u64 + 4, Ordering::Relaxed);
        self.stats.messages_sent.fetch_add(1, Ordering::Relaxed);

        // Receive response
        let mut len_buf = [0u8; 4];
        recv.read_exact(&mut len_buf)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let response_len = u32::from_le_bytes(len_buf) as usize;
        let mut response = vec![0u8; response_len];
        recv.read_exact(&mut response)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.stats
            .bytes_received
            .fetch_add(response_len as u64 + 4, Ordering::Relaxed);
        self.stats.messages_received.fetch_add(1, Ordering::Relaxed);
        self.stats.streams_closed.fetch_add(1, Ordering::Relaxed);

        Ok(response)
    }

    /// Accept incoming connections (for server)
    pub async fn accept(&self) -> io::Result<(Connection, SocketAddr)> {
        let incoming =
            self.endpoint.accept().await.ok_or_else(|| {
                io::Error::new(io::ErrorKind::ConnectionAborted, "Endpoint closed")
            })?;

        let conn = incoming
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let addr = conn.remote_address();

        self.stats
            .connections_established
            .fetch_add(1, Ordering::Relaxed);

        // Cache the connection
        {
            let mut conns = self.connections.write().await;
            conns.insert(addr, conn.clone());
        }

        Ok((conn, addr))
    }

    /// Accept incoming unidirectional stream
    pub async fn accept_uni(&self, conn: &Connection) -> io::Result<(Vec<u8>, SocketAddr)> {
        let mut recv = conn
            .accept_uni()
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.stats.streams_opened.fetch_add(1, Ordering::Relaxed);

        // Read length prefix
        let mut len_buf = [0u8; 4];
        recv.read_exact(&mut len_buf)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let data_len = u32::from_le_bytes(len_buf) as usize;
        let mut data = vec![0u8; data_len];
        recv.read_exact(&mut data)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.stats
            .bytes_received
            .fetch_add(data_len as u64 + 4, Ordering::Relaxed);
        self.stats.messages_received.fetch_add(1, Ordering::Relaxed);
        self.stats.streams_closed.fetch_add(1, Ordering::Relaxed);

        Ok((data, conn.remote_address()))
    }

    /// Get statistics
    pub fn stats(&self) -> &QuicStats {
        &self.stats
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Stop the transport
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        self.endpoint.close(0u32.into(), b"shutdown");
    }

    /// Get local address
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.endpoint.local_addr()
    }

    /// Clean up stale connections
    pub async fn cleanup_connections(&self) {
        let mut conns = self.connections.write().await;
        conns.retain(|_, conn| conn.close_reason().is_none());
    }
}

/// Skip server certificate verification (for development only!)
#[cfg(feature = "quic")]
#[derive(Debug)]
struct SkipServerVerification;

#[cfg(feature = "quic")]
impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

/// Generate a self-signed certificate for testing/development
#[cfg(feature = "quic")]
pub fn generate_self_signed_cert() -> io::Result<(
    Vec<rustls::pki_types::CertificateDer<'static>>,
    rustls::pki_types::PrivateKeyDer<'static>,
)> {
    let cert =
        rcgen::generate_simple_self_signed(vec!["horus".to_string(), "localhost".to_string()])
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let cert_der = rustls::pki_types::CertificateDer::from(cert.cert.der().to_vec());
    let key_der = rustls::pki_types::PrivateKeyDer::try_from(cert.key_pair.serialize_der())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok((vec![cert_der], key_der))
}

// Stub implementation when quic feature is not enabled
#[cfg(not(feature = "quic"))]
pub struct QuicTransport;

#[cfg(not(feature = "quic"))]
impl QuicTransport {
    pub async fn new_client(_bind_addr: SocketAddr, _config: QuicConfig) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "QUIC requires the 'quic' feature",
        ))
    }
}

#[cfg(not(feature = "quic"))]
pub fn generate_self_signed_cert() -> io::Result<(Vec<Vec<u8>>, Vec<u8>)> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "QUIC requires the 'quic' feature",
    ))
}

#[cfg(all(test, feature = "quic"))]
mod tests {
    use super::*;

    fn install_crypto_provider() {
        // Install the default crypto provider for rustls
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    #[tokio::test]
    async fn test_config() {
        let config = QuicConfig::default();
        assert_eq!(config.max_idle_timeout, Duration::from_secs(30));

        let ll = QuicConfig::low_latency();
        assert!(ll.keep_alive_interval < config.keep_alive_interval);
    }

    #[tokio::test]
    async fn test_self_signed_cert() {
        install_crypto_provider();
        let result = generate_self_signed_cert();
        assert!(result.is_ok());

        let (certs, _key) = result.unwrap();
        assert_eq!(certs.len(), 1);
    }

    #[tokio::test]
    async fn test_client_creation() {
        install_crypto_provider();
        let addr = "127.0.0.1:0".parse().unwrap();
        let client = QuicTransport::new_client(addr, QuicConfig::default()).await;
        assert!(client.is_ok());

        let client = client.unwrap();
        assert!(client.is_running());
    }
}
