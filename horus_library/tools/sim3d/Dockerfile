# Multi-stage Docker build for Sim3D
# Stage 1: Build stage
FROM rust:1.75-slim-bullseye as builder

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libudev-dev \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxkbcommon-dev \
    libfontconfig1-dev \
    cmake \
    git \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./
COPY horus_core/Cargo.toml ./horus_core/
COPY horus_library/Cargo.toml ./horus_library/
COPY horus_library/tools/sim3d/Cargo.toml ./horus_library/tools/sim3d/

# Create dummy source files to cache dependencies
RUN mkdir -p horus_core/src && \
    echo "fn main() {}" > horus_core/src/main.rs && \
    mkdir -p horus_library/src && \
    echo "" > horus_library/src/lib.rs && \
    mkdir -p horus_library/tools/sim3d/src && \
    echo "" > horus_library/tools/sim3d/src/lib.rs

# Build dependencies (cached layer)
RUN cargo build --release -p sim3d --features headless

# Remove dummy files
RUN rm -rf horus_core/src horus_library/src horus_library/tools/sim3d/src

# Copy actual source code
COPY horus_core/ ./horus_core/
COPY horus_library/ ./horus_library/

# Build the actual project
RUN cargo build --release -p sim3d --features headless

# Stage 2: Runtime stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    libudev1 \
    libfontconfig1 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 sim3d && \
    mkdir -p /app /data /recordings && \
    chown -R sim3d:sim3d /app /data /recordings

# Copy binary from builder
COPY --from=builder /build/target/release/sim3d /app/sim3d

# Set working directory
WORKDIR /app

# Switch to non-root user
USER sim3d

# Environment variables
ENV RUST_LOG=info
ENV SIM3D_DATA_DIR=/data
ENV SIM3D_RECORDINGS_DIR=/recordings

# Expose ports (if needed for web interface/API)
EXPOSE 8080

# Volume mount points
VOLUME ["/data", "/recordings"]

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD [ "test", "-f", "/app/sim3d" ]

# Default command
ENTRYPOINT ["/app/sim3d"]
CMD ["--help"]
