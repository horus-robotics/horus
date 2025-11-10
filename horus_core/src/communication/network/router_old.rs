/// Router client backend
///
/// Connects to a central HORUS router service for many-to-many communication.
/// The router acts as a message broker: publishers send to the router, which
/// forwards to all subscribers of that topic.
///
/// Usage: Hub::new("camera@router") or Hub::new("camera@192.168.1.100:7777")

use crate::communication::network::protocol::{HorusPacket, MessageType};
use crate::communication::network::fragmentation::{Fragment, FragmentManager};
use crate::error::HorusResult;
use std::collections::VecDeque;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::io::{Read, Write};

const DEFAULT_ROUTER_PORT: u16 = 7777;
const RECV_QUEUE_SIZE: usize = 128;
const BUFFER_SIZE: usize = 65536;

/// Router client backend
pub struct RouterBackend<T> {
    topic_name: String,
    router_addr: SocketAddr,
    connection: Arc<Mutex<Option<TcpStream>>>,
    sequence: Arc<Mutex<u32>>,
    recv_queue: Arc<Mutex<VecDeque<T>>>,
    fragment_manager: Arc<FragmentManager>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> RouterBackend<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
{
    /// Create a new router backend connecting to the default router
    pub fn new(topic: &str) -> HorusResult<Self> {
        // Use localhost router on default port
        Self::new_with_addr(topic, "127.0.0.1".parse().unwrap(), DEFAULT_ROUTER_PORT)
    }

    /// Create a new router backend with custom router address
    pub fn new_with_addr(topic: &str, host: IpAddr, port: u16) -> HorusResult<Self> {
        let router_addr = SocketAddr::new(host, port);

        // Connect to router
        let stream = TcpStream::connect(router_addr)
            .map_err(|e| format!("Failed to connect to router at {}: {}", router_addr, e))?;

        stream.set_nonblocking(true)
            .map_err(|e| format!("Failed to set nonblocking: {}", e))?;

        let mut connection_clone = stream.try_clone()
            .map_err(|e| format!("Failed to clone stream: {}", e))?;

        // Send subscribe message
        let subscribe_packet = HorusPacket::new_router_subscribe(topic.to_string());
        let mut buffer = Vec::with_capacity(1024);
        subscribe_packet.encode(&mut buffer);

        // Write packet length first (4 bytes, little-endian)
        let len_bytes = (buffer.len() as u32).to_le_bytes();
        connection_clone.write_all(&len_bytes)
            .map_err(|e| format!("Failed to send subscribe length: {}", e))?;
        connection_clone.write_all(&buffer)
            .map_err(|e| format!("Failed to send subscribe: {}", e))?;

        let backend = Self {
            topic_name: topic.to_string(),
            router_addr,
            connection: Arc::new(Mutex::new(Some(stream))),
            sequence: Arc::new(Mutex::new(0)),
            recv_queue: Arc::new(Mutex::new(VecDeque::with_capacity(RECV_QUEUE_SIZE))),
            fragment_manager: Arc::new(FragmentManager::default()),
            _phantom: std::marker::PhantomData,
        };

        // Spawn receiver thread
        backend.spawn_receiver();

        Ok(backend)
    }

    fn spawn_receiver(&self) {
        let connection = Arc::clone(&self.connection);
        let recv_queue = Arc::clone(&self.recv_queue);
        let topic_name = self.topic_name.clone();
        let fragment_manager = Arc::clone(&self.fragment_manager);

        std::thread::spawn(move || {
            let mut buffer = vec![0u8; BUFFER_SIZE];
            let mut len_buffer = [0u8; 4];

            loop {
                let mut conn = connection.lock().unwrap();
                if let Some(ref mut stream) = *conn {
                    // Read packet length
                    match stream.read_exact(&mut len_buffer) {
                        Ok(()) => {
                            let packet_len = u32::from_le_bytes(len_buffer) as usize;
                            if packet_len > BUFFER_SIZE {
                                eprintln!("Packet too large: {}", packet_len);
                                continue;
                            }

                            // Read packet data
                            match stream.read_exact(&mut buffer[..packet_len]) {
                                Ok(()) => {
                                    drop(conn);  // Release lock before processing

                                    // Decode packet
                                    match HorusPacket::decode(&buffer[..packet_len]) {
                                        Ok(packet) => {
                                            // Check topic matches
                                            if packet.topic != topic_name {
                                                continue;
                                            }

                                            // Handle message
                                            match packet.msg_type {
                                                MessageType::RouterPublish => {
                                                    // Deserialize payload
                                                    match bincode::deserialize::<T>(&packet.payload) {
                                                        Ok(msg) => {
                                                            let mut queue = recv_queue.lock().unwrap();
                                                            if queue.len() < RECV_QUEUE_SIZE {
                                                                queue.push_back(msg);
                                                            } else {
                                                                queue.pop_front();
                                                                queue.push_back(msg);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            eprintln!("Deserialization error: {}", e);
                                                        }
                                                    }
                                                }
                                                MessageType::Fragment => {
                                                    // Decode fragment
                                                    match Fragment::decode(&packet.payload) {
                                                        Ok(fragment) => {
                                                            if let Some(complete_data) = fragment_manager.reassemble(fragment) {
                                                                match bincode::deserialize::<T>(&complete_data) {
                                                                    Ok(msg) => {
                                                                        let mut queue = recv_queue.lock().unwrap();
                                                                        if queue.len() < RECV_QUEUE_SIZE {
                                                                            queue.push_back(msg);
                                                                        } else {
                                                                            queue.pop_front();
                                                                            queue.push_back(msg);
                                                                        }
                                                                    }
                                                                    Err(e) => {
                                                                        eprintln!("Deserialization error after reassembly: {}", e);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            eprintln!("Fragment decode error: {:?}", e);
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    // Ignore other message types
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("Packet decode error: {}", e);
                                        }
                                    }
                                }
                                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    drop(conn);
                                    std::thread::sleep(std::time::Duration::from_micros(100));
                                }
                                Err(e) => {
                                    eprintln!("TCP read error: {}", e);
                                    drop(conn);
                                    std::thread::sleep(std::time::Duration::from_millis(10));
                                }
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            drop(conn);
                            std::thread::sleep(std::time::Duration::from_micros(100));
                        }
                        Err(e) => {
                            eprintln!("TCP read error: {}", e);
                            drop(conn);
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                    }
                } else {
                    drop(conn);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        });
    }

    /// Send a message via the router
    pub fn send(&self, msg: &T) -> HorusResult<()> {
        // Serialize payload
        let payload = bincode::serialize(msg)
            .map_err(|e| format!("Serialization error: {}", e))?;

        // Fragment if needed
        let fragments = self.fragment_manager.fragment(&payload);

        // Send fragments
        let mut conn = self.connection.lock().unwrap();
        if let Some(ref mut stream) = *conn {
            let mut seq = self.sequence.lock().unwrap();

            for fragment in fragments {
                let packet = if fragment.total == 1 {
                    HorusPacket::new_router_publish(self.topic_name.clone(), fragment.data, *seq)
                } else {
                    let fragment_data = fragment.encode();
                    HorusPacket::new_fragment(self.topic_name.clone(), fragment_data, *seq)
                };
                *seq = seq.wrapping_add(1);

                // Encode packet
                let mut buffer = Vec::with_capacity(2048);
                packet.encode(&mut buffer);

                // Write length + packet
                let len_bytes = (buffer.len() as u32).to_le_bytes();
                stream.write_all(&len_bytes)
                    .map_err(|e| format!("TCP send error (len): {}", e))?;
                stream.write_all(&buffer)
                    .map_err(|e| format!("TCP send error (data): {}", e))?;
            }

            Ok(())
        } else {
            Err(crate::error::HorusError::Communication(
                "Not connected to router".to_string(),
            ))
        }
    }

    /// Receive a message from the router
    pub fn recv(&self) -> Option<T> {
        let mut queue = self.recv_queue.lock().unwrap();
        queue.pop_front()
    }

    /// Get the topic name
    pub fn topic_name(&self) -> &str {
        &self.topic_name
    }

    /// Get the router address
    pub fn router_addr(&self) -> SocketAddr {
        self.router_addr
    }
}

impl<T> std::fmt::Debug for RouterBackend<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouterBackend")
            .field("topic_name", &self.topic_name)
            .field("router_addr", &self.router_addr)
            .finish()
    }
}

impl<T> Drop for RouterBackend<T> {
    fn drop(&mut self) {
        // Send unsubscribe on drop
        let mut conn = self.connection.lock().unwrap();
        if let Some(ref mut stream) = *conn {
            let unsubscribe = HorusPacket::new_router_unsubscribe(self.topic_name.clone());
            let mut buffer = Vec::with_capacity(1024);
            unsubscribe.encode(&mut buffer);

            let len_bytes = (buffer.len() as u32).to_le_bytes();
            let _ = stream.write_all(&len_bytes);
            let _ = stream.write_all(&buffer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestMessage {
        data: u64,
    }

    impl crate::core::LogSummary for TestMessage {
        fn log_summary(&self) -> String {
            format!("TestMessage({})", self.data)
        }
    }

    #[test]
    fn test_router_backend_creation() {
        // This will fail if no router is running, which is expected
        // Just testing that the API compiles
        let result = RouterBackend::<TestMessage>::new_with_addr(
            "test_router",
            "127.0.0.1".parse().unwrap(),
            17777,  // Use different port to avoid conflicts
        );
        // Expected to fail without a running router
        assert!(result.is_err());
    }
}
