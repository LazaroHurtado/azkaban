FROM --platform=linux/amd64 ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

# Install base dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    curl \
    ca-certificates \
    build-essential \
    openssh-client \
    gnupg \
    libicu-dev \
    sudo \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js 22.x (LTS)
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install Azure CLI
RUN curl -sL https://aka.ms/InstallAzureCLIDeb | bash

# Install yq for YAML parsing in entrypoint
RUN curl -fsSL https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64 -o /usr/local/bin/yq \
    && chmod +x /usr/local/bin/yq

# CLI tools are installed via install_cmd from config.yaml at container startup.
# To pre-install tools in the image, add RUN lines here or use a custom Dockerfile.

# Create non-root user (replace ubuntu user that comes with UID 1000)
ARG USER_UID=1000
ARG USER_GID=1000
RUN userdel -r ubuntu 2>/dev/null || true \
    && (groupadd --gid $USER_GID azkaban 2>/dev/null || groupmod -n azkaban $(getent group $USER_GID | cut -d: -f1) 2>/dev/null || true) \
    && useradd --uid $USER_UID --gid $USER_GID -m azkaban \
    && echo "azkaban ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers \
    && mkdir -p /workspace /sessions \
    && chown -R azkaban:azkaban /workspace /sessions

COPY --chmod=755 entrypoint.sh /entrypoint.sh

USER azkaban
WORKDIR /workspace

ENTRYPOINT ["/entrypoint.sh"]
