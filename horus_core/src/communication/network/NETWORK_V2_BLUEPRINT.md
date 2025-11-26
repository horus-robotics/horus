# HORUS Network v2 Blueprint: Beating Zenoh and MQTT

## Target Performance

| Metric | MQTT | Zenoh | HORUS v2 Target |
|--------|------|-------|-----------------|
| Local latency | N/A | 5 µs (pico) | **3-5 µs** |
| LAN latency | 45 µs | 16 µs (P2P) | **8-12 µs** |
| Throughput | 1 Gbps | 67 Gbps | **50+ Gbps** |
| API complexity | Medium | High | **Simple** |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    User API (Unchanged)                     │
│         hub.publish(data)  /  hub.recv()                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Smart Transport Selector                        │
│    Auto-selects optimal backend based on endpoint           │
└─────────────────────────────────────────────────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         ▼                    ▼                    ▼
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│  Local SHM  │      │  LAN Fast   │      │  WAN/Cloud  │
│  (200-500ns)│      │  (5-15µs)   │      │  (reliable) │
│             │      │ io_uring/UDP│      │    QUIC     │
└─────────────┘      └─────────────┘      └─────────────┘
```

---

## Phase 1: UDP Fast Path

### 1.1 Batch System Calls (sendmmsg/recvmmsg)

**Problem:** Current HORUS makes one syscall per packet.

**Solution:** Batch 64-256 packets per syscall.

```rust
pub struct BatchUdpSender {
    socket: RawFd,
    batch_size: usize,           // 64-256 packets
    pending: Vec<PendingPacket>,
    flush_interval: Duration,    // 100µs auto-flush
}

impl BatchUdpSender {
    #[inline]
    pub fn send(&mut self, data: &[u8], addr: SocketAddr) {
        self.pending.push(PendingPacket { data: data.to_vec(), addr });

        if self.pending.len() >= self.batch_size {
            self.flush();
        }
    }

    fn flush(&mut self) {
        // Single syscall for all pending packets
        let mut msgs: Vec<libc::mmsghdr> = self.pending.iter()
            .map(|p| build_mmsghdr(p))
            .collect();

        unsafe {
            libc::sendmmsg(self.socket, msgs.as_mut_ptr(), msgs.len() as u32, 0);
        }
        self.pending.clear();
    }
}
```

**Expected gain:** 35K → 200K+ packets/sec per core

### 1.2 Socket Optimizations

```rust
fn optimize_socket(fd: RawFd) {
    unsafe {
        // 4MB buffers for burst handling
        let buf_size: i32 = 4 * 1024 * 1024;
        libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_SNDBUF,
                         &buf_size as *const _ as *const _, 4);
        libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_RCVBUF,
                         &buf_size as *const _ as *const _, 4);

        // Busy polling (reduces latency by ~5µs)
        let busy_poll: i32 = 50; // 50µs
        libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_BUSY_POLL,
                         &busy_poll as *const _ as *const _, 4);
    }
}
```

### 1.3 Multi-Socket Scaling (SO_REUSEPORT)

```rust
pub struct ScalableUdpBackend {
    sockets: Vec<UdpSocket>,  // One per CPU core
}

impl ScalableUdpBackend {
    pub fn new(port: u16, num_cores: usize) -> Result<Self> {
        let mut sockets = Vec::new();

        for core_id in 0..num_cores {
            let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
            socket.set_reuse_port(true)?;
            socket.bind(&format!("0.0.0.0:{}", port).parse()?)?;

            // Pin receiver thread to specific core
            let socket_clone = socket.try_clone()?;
            std::thread::spawn(move || {
                set_affinity(core_id);
                receive_loop(socket_clone);
            });

            sockets.push(socket.into());
        }

        Ok(Self { sockets })
    }
}
```

**Expected throughput:** 1M+ packets/sec with 8 cores

---

## Phase 2: io_uring Zero-Copy

### 2.1 Real io_uring Integration

Replace current simulation with actual io_uring:

```rust
use io_uring::{IoUring, opcode, types::Fd};

pub struct IoUringNetwork {
    ring: IoUring,
    socket_fd: RawFd,
    registered_buffers: Vec<Vec<u8>>,
    free_buffers: VecDeque<usize>,
}

impl IoUringNetwork {
    pub fn new(socket: UdpSocket, config: IoUringConfig) -> io::Result<Self> {
        let mut ring = IoUring::builder()
            .setup_sqpoll(2000)           // Kernel-side polling thread
            .setup_coop_taskrun()          // Reduce interrupts
            .build(config.sq_entries)?;

        // Register fixed buffers for zero-copy
        let buffers: Vec<Vec<u8>> = (0..config.num_buffers)
            .map(|_| vec![0u8; config.buffer_size])
            .collect();

        unsafe {
            ring.submitter().register_buffers(
                buffers.iter().map(|b| libc::iovec {
                    iov_base: b.as_ptr() as *mut _,
                    iov_len: b.len(),
                }).collect::<Vec<_>>().as_slice()
            )?;
        }

        Ok(Self {
            ring,
            socket_fd: socket.as_raw_fd(),
            registered_buffers: buffers,
            free_buffers: (0..config.num_buffers).collect(),
        })
    }

    /// Zero-copy send
    pub fn send_zc(&mut self, data: &[u8]) -> io::Result<u64> {
        let buf_idx = self.free_buffers.pop_front()
            .ok_or_else(|| io::Error::new(io::ErrorKind::WouldBlock, "No buffers"))?;

        // Copy to registered buffer (kernel DMAs directly from here)
        self.registered_buffers[buf_idx][..data.len()].copy_from_slice(data);

        let sqe = opcode::SendZc::new(Fd(self.socket_fd), data.as_ptr(), data.len() as u32)
            .buf_index(buf_idx as u16)
            .build()
            .user_data(buf_idx as u64);

        unsafe { self.ring.submission().push(&sqe)?; }
        self.ring.submit()?;

        Ok(buf_idx as u64)
    }

    /// Process completions
    pub fn poll(&mut self) -> Vec<Completion> {
        let mut completions = Vec::new();

        for cqe in self.ring.completion() {
            let buf_idx = cqe.user_data() as usize;
            self.free_buffers.push_back(buf_idx);

            completions.push(Completion {
                user_data: cqe.user_data(),
                result: cqe.result(),
            });
        }

        completions
    }
}
```

### 2.2 SQPOLL Mode

With SQPOLL, the kernel polls the submission queue without syscalls:

```rust
let ring = IoUring::builder()
    .setup_sqpoll(2000)      // Poll for 2ms before sleeping
    .setup_sqpoll_cpu(0)     // Pin kernel thread to CPU 0
    .build(256)?;
```

**Expected latency:** 3-5µs (matches Zenoh-pico)

---

## Phase 3: QUIC Transport

### 3.1 Why QUIC?

| Feature | TCP | UDP | QUIC |
|---------|-----|-----|------|
| Connection setup | 3 RTT | 0 | 0-1 RTT |
| Head-of-line blocking | Yes | No | No |
| Packet loss recovery | Slow | None | Fast |
| Encryption | Optional | No | Built-in |

### 3.2 Implementation with Quinn

```rust
use quinn::{Endpoint, Connection, TransportConfig};

pub struct QuicBackend {
    endpoint: Endpoint,
    connections: DashMap<SocketAddr, Connection>,
}

impl QuicBackend {
    pub async fn new(bind_addr: SocketAddr, certs: Vec<Certificate>) -> Result<Self> {
        let mut transport = TransportConfig::default();
        transport.max_idle_timeout(Some(Duration::from_secs(30).try_into()?));
        transport.keep_alive_interval(Some(Duration::from_secs(5)));

        let mut server_config = quinn::ServerConfig::with_single_cert(certs, key)?;
        server_config.transport_config(Arc::new(transport));

        let endpoint = Endpoint::server(server_config, bind_addr)?;

        Ok(Self {
            endpoint,
            connections: DashMap::new(),
        })
    }

    /// Send with 0-RTT (no handshake for cached connections)
    pub async fn send(&self, addr: SocketAddr, data: &[u8]) -> Result<()> {
        let conn = self.get_or_connect(addr).await?;
        let mut stream = conn.open_uni().await?;
        stream.write_all(data).await?;
        stream.finish().await?;
        Ok(())
    }

    async fn get_or_connect(&self, addr: SocketAddr) -> Result<Connection> {
        if let Some(conn) = self.connections.get(&addr) {
            return Ok(conn.clone());
        }

        let conn = self.endpoint
            .connect(addr, "horus")?
            .await?;

        self.connections.insert(addr, conn.clone());
        Ok(conn)
    }
}
```

---

## Phase 4: Smart Transport Selection

### 4.1 Automatic Selection

```rust
impl<T> Hub<T> {
    pub fn new(endpoint: &str) -> Result<Self> {
        let parsed = parse_endpoint(endpoint)?;

        let transport = match &parsed {
            // Same machine → shared memory
            Endpoint::Local(_) => Transport::SharedMemory,

            // Local network → fastest available
            Endpoint::Udp(addr) | Endpoint::Tcp(addr) if is_local_network(addr) => {
                if cfg!(target_os = "linux") && is_io_uring_available() {
                    Transport::IoUring
                } else {
                    Transport::BatchUdp
                }
            }

            // WAN or explicit QUIC → reliable transport
            Endpoint::Quic(addr) => Transport::Quic,

            // Default fallback
            _ => Transport::BatchUdp,
        };

        Self::with_transport(parsed, transport)
    }
}

fn is_local_network(addr: &SocketAddr) -> bool {
    match addr.ip() {
        IpAddr::V4(ip) => {
            ip.is_loopback() ||
            ip.is_private() ||           // 10.x, 172.16-31.x, 192.168.x
            ip.is_link_local()           // 169.254.x
        }
        IpAddr::V6(ip) => {
            ip.is_loopback() ||
            (ip.segments()[0] & 0xfe00) == 0xfc00  // Unique local
        }
    }
}
```

### 4.2 User API (Unchanged)

```rust
// Simple usage - transport auto-selected
let hub = Hub::<SensorData>::new("udp://192.168.1.100:9000")?;
hub.publish(&data)?;

// Power user - explicit transport
let hub = Hub::<SensorData>::builder("sensor/imu")
    .transport(Transport::IoUring)
    .batch_size(128)
    .compression(Compression::Lz4)
    .build()?;
```

---

## Phase 5: Implementation Plan

### Timeline

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| Phase 1: UDP Fast Path | 2-3 weeks | sendmmsg, SO_BUSY_POLL, SO_REUSEPORT |
| Phase 2: io_uring | 3-4 weeks | Zero-copy send/recv, SQPOLL |
| Phase 3: QUIC | 2-3 weeks | Quinn integration, 0-RTT |
| Phase 4: Smart Selection | 1-2 weeks | Auto-select, same API |
| Phase 5: Polish | 1 week | Benchmarks, docs |

**Total: ~10 weeks**

### Dependencies

```toml
[dependencies]
# io_uring (Linux only)
io-uring = "0.6"

# QUIC
quinn = "0.11"
rustls = "0.23"
rcgen = "0.13"  # For self-signed certs

# Socket options
socket2 = "0.5"

# Existing
crossbeam = "0.8"
tokio = { version = "1", features = ["rt-multi-thread", "net", "io-util"] }
```

### Files to Create/Modify

```
horus_core/src/communication/network/
├── mod.rs                 # Add new exports
├── batch_udp.rs          # NEW: sendmmsg/recvmmsg batching
├── io_uring.rs           # MODIFY: Real io_uring (replace simulation)
├── quic.rs               # NEW: QUIC transport
├── smart_selector.rs     # NEW: Auto transport selection
└── socket_opts.rs        # NEW: Platform-specific optimizations
```

---

## Benchmarking Targets

### Phase 1 Complete
- [ ] 500K+ packets/sec single socket
- [ ] 1M+ packets/sec with SO_REUSEPORT (8 cores)
- [ ] <20µs latency

### Phase 2 Complete
- [ ] 3-5µs latency with io_uring
- [ ] Saturate 10Gb NIC on single core
- [ ] Zero-copy verified (no memcpy in profile)

### Phase 3 Complete
- [ ] 0-RTT working for cached connections
- [ ] <50µs latency over WAN
- [ ] Graceful fallback on packet loss

### Final
- [ ] Beat Zenoh-pico (5µs) on local
- [ ] Beat Zenoh P2P (16µs) on LAN
- [ ] 50+ Gbps throughput on 100Gb NIC
