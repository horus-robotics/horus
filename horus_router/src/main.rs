/// HORUS Router Service
///
/// Central message broker for many-to-many pub/sub communication.
/// Clients connect via TCP, subscribe to topics, and publish messages.
/// The router forwards messages to all subscribers of each topic.

use clap::Parser;
use horus_core::communication::network::protocol::{HorusPacket, MessageType};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

#[cfg(feature = "tls")]
use horus_core::communication::network::tls::{TlsCertConfig, TlsStream};

const DEFAULT_PORT: u16 = 7777;
const BUFFER_SIZE: usize = 65536;

#[derive(Parser, Debug)]
#[command(name = "horus_router")]
#[command(about = "HORUS central message router service", long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// Bind address
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Enable TLS (requires 'tls' feature)
    #[cfg(feature = "tls")]
    #[arg(long)]
    tls: bool,

    /// Path to TLS certificate file (PEM format)
    #[cfg(feature = "tls")]
    #[arg(long)]
    tls_cert: Option<String>,

    /// Path to TLS private key file (PEM format)
    #[cfg(feature = "tls")]
    #[arg(long)]
    tls_key: Option<String>,
}

/// A client connection
struct Client {
    addr: SocketAddr,
    tx: mpsc::UnboundedSender<Vec<u8>>,
}

/// Router state managing all subscriptions
struct RouterState {
    /// Map of topic -> list of clients subscribed to that topic
    subscriptions: RwLock<HashMap<String, Vec<Arc<Client>>>>,

    /// Map of client address -> client info (for cleanup on disconnect)
    clients: RwLock<HashMap<SocketAddr, Arc<Client>>>,
}

impl RouterState {
    fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
            clients: RwLock::new(HashMap::new()),
        }
    }

    /// Subscribe a client to a topic
    async fn subscribe(&self, topic: String, client: Arc<Client>) {
        let mut subs = self.subscriptions.write().await;
        subs.entry(topic.clone()).or_insert_with(Vec::new).push(client.clone());
        info!("Client {} subscribed to topic '{}'", client.addr, topic);
    }

    /// Unsubscribe a client from a topic
    async fn unsubscribe(&self, topic: &str, client_addr: SocketAddr) {
        let mut subs = self.subscriptions.write().await;
        if let Some(clients) = subs.get_mut(topic) {
            clients.retain(|c| c.addr != client_addr);
            if clients.is_empty() {
                subs.remove(topic);
            }
            info!("Client {} unsubscribed from topic '{}'", client_addr, topic);
        }
    }

    /// Publish a message to all subscribers of a topic
    async fn publish(&self, topic: &str, packet_data: Vec<u8>) {
        let subs = self.subscriptions.read().await;
        if let Some(clients) = subs.get(topic) {
            let count = clients.len();
            for client in clients {
                if let Err(e) = client.tx.send(packet_data.clone()) {
                    warn!("Failed to send to client {}: {}", client.addr, e);
                }
            }
            info!("Published message on topic '{}' to {} subscribers", topic, count);
        } else {
            warn!("No subscribers for topic '{}'", topic);
        }
    }

    /// Register a new client
    async fn register_client(&self, client: Arc<Client>) {
        let mut clients = self.clients.write().await;
        clients.insert(client.addr, client);
    }

    /// Remove a client and all its subscriptions
    async fn remove_client(&self, addr: SocketAddr) {
        // Remove from clients map
        let mut clients = self.clients.write().await;
        clients.remove(&addr);
        drop(clients);

        // Remove from all subscriptions
        let mut subs = self.subscriptions.write().await;
        for (_topic, clients) in subs.iter_mut() {
            clients.retain(|c| c.addr != addr);
        }
        subs.retain(|_, clients| !clients.is_empty());

        info!("Removed client {}", addr);
    }
}

/// Handle a single client connection (generic over stream type)
async fn handle_client_inner<S>(
    stream: S,
    addr: SocketAddr,
    state: Arc<RouterState>,
)
where
    S: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
{
    info!("New client connected: {}", addr);

    let (mut read_half, mut write_half) = tokio::io::split(stream);

    // Create channel for sending messages to this client
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    let client = Arc::new(Client { addr, tx });
    state.register_client(client.clone()).await;

    // Spawn task to send messages to client
    let write_task = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            // Write length prefix (4 bytes)
            let len_bytes = (data.len() as u32).to_le_bytes();
            if write_half.write_all(&len_bytes).await.is_err() {
                break;
            }

            // Write data
            if write_half.write_all(&data).await.is_err() {
                break;
            }
        }
    });

    // Read messages from client
    let mut len_buffer = [0u8; 4];
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        // Read packet length
        match read_half.read_exact(&mut len_buffer).await {
            Ok(_) => {
                let packet_len = u32::from_le_bytes(len_buffer) as usize;

                if packet_len > BUFFER_SIZE {
                    error!("Packet too large from {}: {} bytes", addr, packet_len);
                    break;
                }

                // Read packet data
                match read_half.read_exact(&mut buffer[..packet_len]).await {
                    Ok(_) => {
                        // Decode packet
                        match HorusPacket::decode(&buffer[..packet_len]) {
                            Ok(packet) => {
                                match packet.msg_type {
                                    MessageType::RouterSubscribe => {
                                        state.subscribe(packet.topic, client.clone()).await;
                                    }
                                    MessageType::RouterUnsubscribe => {
                                        state.unsubscribe(&packet.topic, addr).await;
                                    }
                                    MessageType::RouterPublish | MessageType::Fragment => {
                                        // Forward to all subscribers
                                        state.publish(&packet.topic, buffer[..packet_len].to_vec()).await;
                                    }
                                    _ => {
                                        warn!("Unexpected message type from {}: {:?}", addr, packet.msg_type);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to decode packet from {}: {}", addr, e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read packet data from {}: {}", addr, e);
                        break;
                    }
                }
            }
            Err(e) => {
                info!("Client {} disconnected: {}", addr, e);
                break;
            }
        }
    }

    // Cleanup on disconnect
    state.remove_client(addr).await;
    write_task.abort();
}

/// Handle plain TCP client
async fn handle_tcp_client(stream: TcpStream, addr: SocketAddr, state: Arc<RouterState>) {
    handle_client_inner(stream, addr, state).await;
}

/// Handle TLS client
#[cfg(feature = "tls")]
async fn handle_tls_client(stream: TlsStream<TcpStream>, addr: SocketAddr, state: Arc<RouterState>) {
    handle_client_inner(stream, addr, state).await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    let filter = if args.verbose {
        "horus_router=debug,info"
    } else {
        "horus_router=info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    // Create router state
    let state = Arc::new(RouterState::new());

    // Bind to address
    let bind_addr = format!("{}:{}", args.bind, args.port);
    let listener = TcpListener::bind(&bind_addr).await?;

    #[cfg(feature = "tls")]
    let tls_enabled = args.tls;
    #[cfg(not(feature = "tls"))]
    let tls_enabled = false;

    if tls_enabled {
        info!("HORUS Router listening on {} (TLS enabled)", bind_addr);
    } else {
        info!("HORUS Router listening on {} (plain TCP)", bind_addr);
    }
    info!("Ready to accept client connections");

    // Create TLS acceptor if enabled
    #[cfg(feature = "tls")]
    let tls_acceptor = if tls_enabled {
        let mut tls_config = TlsCertConfig::new();

        if let (Some(cert), Some(key)) = (args.tls_cert, args.tls_key) {
            tls_config = tls_config.with_files(cert, key);
        } else {
            tls_config = tls_config.with_auto_generate();
        }

        match tls_config.create_acceptor() {
            Ok(acceptor) => Some(acceptor),
            Err(e) => {
                error!("Failed to create TLS acceptor: {}", e);
                return Err(e.into());
            }
        }
    } else {
        None
    };

    // Accept connections
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let state = state.clone();

                #[cfg(feature = "tls")]
                if let Some(ref acceptor) = tls_acceptor {
                    let acceptor = acceptor.clone();
                    tokio::spawn(async move {
                        match acceptor.accept(stream).await {
                            Ok(tls_stream) => {
                                handle_tls_client(tls_stream, addr, state).await;
                            }
                            Err(e) => {
                                error!("TLS handshake failed for {}: {}", addr, e);
                            }
                        }
                    });
                    continue;
                }

                // Plain TCP connection
                tokio::spawn(async move {
                    handle_tcp_client(stream, addr, state).await;
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}
