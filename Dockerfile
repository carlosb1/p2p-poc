# Use Rust official image as the builder
FROM rust:1.87

# Set working directory
WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    awscli \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first
COPY Cargo.toml .
COPY Cargo.lock .

# Now copy full source
COPY . .

# Build the specific binary
RUN cargo build --release


# Run the application
CMD ["./target/release/messages-p2p"]