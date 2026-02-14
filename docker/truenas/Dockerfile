# Investor OS HRM - TrueNAS Deployment with RTX 3090
# 
# Base image with CUDA support for RTX 3090 (Compute Capability 8.6)
# TrueNAS 25.10.1 compatible

FROM nvidia/cuda:12.2.0-runtime-ubuntu22.04 as base

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    libpq-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Set CUDA environment
ENV CUDA_HOME=/usr/local/cuda
ENV PATH=${CUDA_HOME}/bin:${PATH}
ENV LD_LIBRARY_PATH=${CUDA_HOME}/lib64:${LD_LIBRARY_PATH}

# Verify CUDA installation
RUN nvidia-smi || echo "NVIDIA runtime not available at build time"

# =============================================================================
# Build stage
# =============================================================================
FROM base as builder

WORKDIR /build

# Copy Cargo files first for caching
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo

# Copy source code
COPY src ./src
COPY tests ./tests
COPY migrations ./migrations
COPY config ./config

# Build with CUDA support
RUN cargo build --release --features cuda

# =============================================================================
# Runtime stage
# =============================================================================
FROM nvidia/cuda:12.2.0-runtime-ubuntu22.04 as runtime

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    libssl3 \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set CUDA environment
ENV CUDA_HOME=/usr/local/cuda
ENV PATH=${CUDA_HOME}/bin:${PATH}
ENV LD_LIBRARY_PATH=${CUDA_HOME}/lib64:${LD_LIBRARY_PATH}

# NVIDIA Container Toolkit configuration for RTX 3090
ENV NVIDIA_VISIBLE_DEVICES=all
ENV NVIDIA_DRIVER_CAPABILITIES=compute,utility

# Application configuration
ENV RUST_LOG=info
ENV INVESTOR_OS_ENV=production
ENV HRM_DEVICE=cuda
ENV HRM_CUDA_VISIBLE_DEVICES=0

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/investor-os /app/investor-os

# Copy configuration
COPY --from=builder /build/config /app/config

# Copy HRM weights (if available)
COPY --from=builder /build/models /app/models

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Expose ports
EXPOSE 3000

# Run application
CMD ["/app/investor-os"]
