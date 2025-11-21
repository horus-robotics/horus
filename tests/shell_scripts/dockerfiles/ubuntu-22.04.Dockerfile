FROM ubuntu:22.04

# Minimal clean Ubuntu system - no dev tools installed
ENV DEBIAN_FRONTEND=noninteractive

# Only basic utilities needed for testing
RUN apt-get update && apt-get install -y \
    curl \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create test user (simulates regular user, not root)
RUN useradd -m -s /bin/bash testuser && \
    echo "testuser ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

USER testuser
WORKDIR /home/testuser

# This is intentionally minimal - scripts should handle missing dependencies
