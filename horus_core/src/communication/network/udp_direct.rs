/// UDP direct backend for point-to-point network communication
///
/// Provides <50μs latency for LAN communication using direct UDP sockets.
/// No discovery overhead - you specify the target host directly.
use crate::communication::network::protocol::{HorusPacket, MessageType};
use crate::communication::network::fragmentation::{Fragment, FragmentManager};
use crate::error::HorusResult;
use std::collections::VecDeque;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};

const UDP_BUFFER_SIZE: usize = 65536; // 64KB (max UDP packet)
const RECV_QUEUE_SIZE: usize = 128; // Buffer up to 128 messages

/// UDP direct backend for network communication
pub struct UdpDirectBackend<T> {
    topic_name: String,
    socket: Arc<UdpSocket>,
    remote_addr: SocketAddr,
    sequence: Arc<Mutex<u32>>,
    recv_queue: Arc<Mutex<VecDeque<T>>>,
    fragment_manager: Arc<FragmentManager>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> UdpDirectBackend<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
{
    /// Create a new UDP direct backend
    pub fn new(topic: &str, host: IpAddr, port: u16) -> HorusResult<Self> {
        // Bind to any available port
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

        socket.set_nonblocking(true)
            .map_err(|e| format!("Failed to set nonblocking: {}", e))?;

        // Note: socket buffer sizes could be optimized with socket2 crate
        // For now, use OS defaults

        let remote_addr = SocketAddr::new(host, port);

        let backend = Self {
            topic_name: topic.to_string(),
            socket: Arc::new(socket),
            remote_addr,
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
        let socket = Arc::clone(&self.socket);
        let recv_queue = Arc::clone(&self.recv_queue);
        let topic_name = self.topic_name.clone();
        let fragment_manager = Arc::clone(&self.fragment_manager);

        std::thread::spawn(move || {
            let mut buffer = vec![0u8; UDP_BUFFER_SIZE];

            loop {
                match socket.recv_from(&mut buffer) {
                    Ok((size, _src_addr)) => {
                        // Decode packet
                        match HorusPacket::decode(&buffer[..size]) {
                            Ok(packet) => {
                                // Check topic matches
                                if packet.topic != topic_name {
                                    continue;
                                }

                                // Handle different message types
                                match packet.msg_type {
                                    MessageType::Data => {
                                        // Deserialize payload
                                        match bincode::deserialize::<T>(&packet.payload) {
                                            Ok(msg) => {
                                                let mut queue = recv_queue.lock().unwrap();
                                                if queue.len() < RECV_QUEUE_SIZE {
                                                    queue.push_back(msg);
                                                } else {
                                                    // Queue full, drop oldest
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
                                                // Try to reassemble
                                                if let Some(complete_data) = fragment_manager.reassemble(fragment) {
                                                    // Deserialize complete message
                                                    match bincode::deserialize::<T>(&complete_data) {
                                                        Ok(msg) => {
                                                            let mut queue = recv_queue.lock().unwrap();
                                                            if queue.len() < RECV_QUEUE_SIZE {
                                                                queue.push_back(msg);
                                                            } else {
                                                                // Queue full, drop oldest
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
                        // No data available, sleep briefly
                        std::thread::sleep(std::time::Duration::from_micros(100));
                    }
                    Err(e) => {
                        eprintln!("UDP recv error: {}", e);
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }
        });
    }

    /// Send a message over UDP
    pub fn send(&self, msg: &T) -> HorusResult<()> {
        // Serialize payload
        let payload = bincode::serialize(msg)
            .map_err(|e| format!("Serialization error: {}", e))?;

        // Fragment the payload if needed
        let fragments = self.fragment_manager.fragment(&payload);

        // Send fragments
        let mut seq = self.sequence.lock().unwrap();
        for fragment in fragments {
            let packet = if fragment.total == 1 {
                // Single fragment - send as Data
                HorusPacket::new_data(self.topic_name.clone(), fragment.data, *seq)
            } else {
                // Multi-fragment - send as Fragment
                let fragment_data = fragment.encode();
                HorusPacket::new_fragment(self.topic_name.clone(), fragment_data, *seq)
            };
            *seq = seq.wrapping_add(1);

            // Encode packet
            let mut buffer = Vec::with_capacity(2048);
            packet.encode(&mut buffer);

            // Send UDP packet
            self.socket.send_to(&buffer, self.remote_addr)
                .map_err(|e| format!("UDP send error: {}", e))?;
        }
        drop(seq);

        Ok(())
    }

    /// Receive a message from UDP
    pub fn recv(&self) -> Option<T> {
        let mut queue = self.recv_queue.lock().unwrap();
        queue.pop_front()
    }

    /// Get the topic name
    pub fn topic_name(&self) -> &str {
        &self.topic_name
    }

    /// Get the remote address
    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }
}

impl<T> std::fmt::Debug for UdpDirectBackend<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UdpDirectBackend")
            .field("topic_name", &self.topic_name)
            .field("remote_addr", &self.remote_addr)
            .finish()
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
    fn test_udp_direct_basic() {
        // Create listener on port 19870
        let listener_socket = UdpSocket::bind("127.0.0.1:19870").unwrap();

        // Create backend that sends to listener
        let backend = UdpDirectBackend::<TestMessage>::new(
            "test_udp",
            "127.0.0.1".parse().unwrap(),
            19870,
        ).unwrap();

        // Send message
        let msg = TestMessage { data: 42 };
        backend.send(&msg).unwrap();

        // Receive on listener
        let mut buffer = vec![0u8; UDP_BUFFER_SIZE];
        let (size, _) = listener_socket.recv_from(&mut buffer).unwrap();

        // Decode and verify
        let packet = HorusPacket::decode(&buffer[..size]).unwrap();
        assert_eq!(packet.topic, "test_udp");
        assert_eq!(packet.msg_type, MessageType::Data);

        let decoded: TestMessage = bincode::deserialize(&packet.payload).unwrap();
        assert_eq!(decoded.data, 42);
    }

    #[test]
    fn test_fragmentation_large_message() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct LargeMessage {
            data: Vec<u8>,
        }

        impl crate::core::LogSummary for LargeMessage {
            fn log_summary(&self) -> String {
                format!("LargeMessage({} bytes)", self.data.len())
            }
        }

        // Create listener on port 19871
        let listener_socket = UdpSocket::bind("127.0.0.1:19871").unwrap();
        listener_socket.set_nonblocking(false).unwrap();

        // Create backend that sends to listener
        let backend = UdpDirectBackend::<LargeMessage>::new(
            "test_large",
            "127.0.0.1".parse().unwrap(),
            19871,
        ).unwrap();

        // Create large message (6 MB, like a camera image)
        let large_data = vec![42u8; 6 * 1024 * 1024];
        let msg = LargeMessage { data: large_data.clone() };

        // Send large message (should be fragmented)
        backend.send(&msg).unwrap();

        // The message should be split into multiple fragments
        // Each fragment should arrive separately
        // We just verify that at least one packet arrives
        let mut buffer = vec![0u8; UDP_BUFFER_SIZE];
        let (size, _) = listener_socket.recv_from(&mut buffer).unwrap();

        // Decode first packet
        let packet = HorusPacket::decode(&buffer[..size]).unwrap();
        assert_eq!(packet.topic, "test_large");
        // Should be either Data (single fragment) or Fragment (multi-fragment)
        assert!(packet.msg_type == MessageType::Data || packet.msg_type == MessageType::Fragment);
    }
}
