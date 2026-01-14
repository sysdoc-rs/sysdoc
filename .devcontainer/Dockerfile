FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# Install additional tools needed for development
RUN apt-get update && apt-get install -y \
    libseccomp-dev \
    && rm -rf /var/lib/apt/lists/*

# The landlock and seccomp crates need these for building
