FROM fedora:39

# Clean Fedora system
RUN dnf install -y \
    curl \
    git \
    ca-certificates \
    sudo \
    && dnf clean all

RUN useradd -m -s /bin/bash testuser && \
    echo "testuser ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

USER testuser
WORKDIR /home/testuser
