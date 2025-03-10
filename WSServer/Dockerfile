# Use the official Rust image as a base
FROM rust:1.85 AS builder

# Set the working directory
WORKDIR /app

# Copy the Cargo manifest files first
COPY Cargo.toml Cargo.lock ./

# Create a dummy src directory and build dependencies first
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Copy the actual source code
COPY src ./src

# Build the actual application
RUN cargo build --release

# Use a lightweight runtime image
FROM debian:bookworm-slim

# Set environment variables (to be overridden by docker-compose)
ENV JWT_PASSWORD=""

# Install dependencies needed for running Rust binaries
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
RUN apt-get update && apt-get install -y strace

# Set the working directory
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/WSServer /app/WSServer

# Expose the necessary port for the warp server
EXPOSE 3030

# Set the command to run the application
CMD ["/app/WSServer"]

