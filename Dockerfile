# Build stage
FROM rust:1-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    libseccomp-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY sysdoc/Cargo.toml ./sysdoc/

# Copy external dependencies (fonts, etc.) needed at build time
COPY external ./external

# Create dummy main to build dependencies
RUN mkdir -p sysdoc/src && echo "fn main() {}" > sysdoc/src/main.rs
RUN cargo build --release
RUN rm -rf sysdoc/src

# Copy actual source and build
COPY sysdoc/src ./sysdoc/src
RUN touch sysdoc/src/main.rs  # Update timestamp
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies (minimal)
RUN apt-get update && apt-get install -y \
    libseccomp2 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --user-group sysdoc

COPY --from=builder /build/target/release/sysdoc /usr/local/bin/sysdoc

# Create standard directories
RUN mkdir -p /input /output && chown sysdoc:sysdoc /input /output

USER sysdoc
WORKDIR /home/sysdoc

# Default to requiring sandbox since we're in a controlled environment
ENV SYSDOC_REQUIRE_SANDBOX=true

ENTRYPOINT ["sysdoc"]
CMD ["--help"]
