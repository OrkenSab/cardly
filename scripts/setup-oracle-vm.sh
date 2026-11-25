#!/usr/bin/env bash
# Copyright: Cardly contributors
# License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html
#
# Cardly — Automated Oracle Cloud VM (1 GB RAM, 1 vCPU) Provisioning Script
# Sets up 2 GB swap, installs Docker, opens firewall (iptables/ufw),
# configures Caddy automated HTTPS, and launches Cardly in Demo Mode.

set -euo pipefail

# ANSI Styling
BOLD='\033[1m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}${BOLD}================================================================${NC}"
echo -e "${BLUE}${BOLD}  Cardly — Oracle Cloud VM Setup (1 GB RAM / 1 vCPU Demo Mode)  ${NC}"
echo -e "${BLUE}${BOLD}================================================================${NC}\n"

# 1. Root privileges check
if [[ $EUID -ne 0 ]]; then
  echo -e "${RED}Error: This setup script must be run as root (or with sudo).${NC}"
  echo -e "Usage: sudo bash $0 [DOMAIN_NAME]"
  exit 1
fi

DOMAIN="${1:-}"
if [[ -z "$DOMAIN" ]]; then
  echo -e "${YELLOW}Please enter your domain or DuckDNS subdomain for HTTPS.${NC}"
  echo -e "Example: ${BOLD}cardly-demo.duckdns.org${NC} or ${BOLD}demo.yourdomain.com${NC}"
  read -rp "Domain: " DOMAIN
fi

if [[ -z "$DOMAIN" ]]; then
  echo -e "${RED}Error: Domain cannot be empty.${NC}"
  exit 1
fi

DEPLOY_DIR="/opt/cardly"

# 2. Swap configuration (Crucial for 1 GB RAM instance)
echo -e "\n${BLUE}[1/6] Checking system memory and configuring 2 GB swap...${NC}"
TOTAL_SWAP_KB=$(grep SwapTotal /proc/meminfo | awk '{print $2}')
if [[ $TOTAL_SWAP_KB -lt 1500000 ]]; then
  echo -e "${YELLOW}Swap is less than 1.5 GB. Creating 2 GB swapfile...${NC}"
  if [[ ! -f /swapfile ]]; then
    fallocate -l 2G /swapfile || dd if=/dev/zero of=/swapfile bs=1M count=2048
    chmod 600 /swapfile
    mkswap /swapfile
  fi
  swapon /swapfile 2>/dev/null || true
  if ! grep -q "/swapfile" /etc/fstab; then
    echo "/swapfile none swap sw 0 0" >> /etc/fstab
  fi
  # Tune swappiness for low-memory container workload
  mkdir -p /etc/sysctl.d
  echo "vm.swappiness=10" > /etc/sysctl.d/99-cardly.conf
  echo "vm.vfs_cache_pressure=50" >> /etc/sysctl.d/99-cardly.conf
  sysctl -p /etc/sysctl.d/99-cardly.conf >/dev/null 2>&1 || true
  echo -e "${GREEN}✓ 2 GB swap file activated and persisted in /etc/fstab.${NC}"
else
  echo -e "${GREEN}✓ Sufficient swap memory detected ($((TOTAL_SWAP_KB / 1024)) MB).${NC}"
fi

# 3. Host Firewall Configuration (Ubuntu on Oracle Cloud)
echo -e "\n${BLUE}[2/6] Configuring instance firewall (iptables & UFW)...${NC}"
# Oracle Cloud Ubuntu images include default iptables rejecting non-SSH ports
if command -v iptables >/dev/null 2>&1; then
  # Insert ACCEPT rules for ports 80 and 443 before reject lines
  iptables -I INPUT 6 -p tcp --dport 80 -m state --state NEW,ESTABLISHED -j ACCEPT 2>/dev/null || iptables -A INPUT -p tcp --dport 80 -j ACCEPT
  iptables -I INPUT 6 -p tcp --dport 443 -m state --state NEW,ESTABLISHED -j ACCEPT 2>/dev/null || iptables -A INPUT -p tcp --dport 443 -j ACCEPT
  iptables -I INPUT 6 -p udp --dport 443 -m state --state NEW,ESTABLISHED -j ACCEPT 2>/dev/null || iptables -A INPUT -p udp --dport 443 -j ACCEPT

  if command -v netfilter-persistent >/dev/null 2>&1; then
    netfilter-persistent save >/dev/null 2>&1 || true
  elif command -v iptables-save >/dev/null 2>&1; then
    mkdir -p /etc/iptables
    iptables-save > /etc/iptables/rules.v4 2>/dev/null || true
  fi
  echo -e "${GREEN}✓ iptables rules updated for ports 80/tcp, 443/tcp, 443/udp.${NC}"
fi

if command -v ufw >/dev/null 2>&1; then
  ufw allow 22/tcp >/dev/null 2>&1 || true
  ufw allow 80/tcp >/dev/null 2>&1 || true
  ufw allow 443/tcp >/dev/null 2>&1 || true
  ufw allow 443/udp >/dev/null 2>&1 || true
  echo -e "${GREEN}✓ UFW rules allowed for ports 80 & 443.${NC}"
fi

# 4. Install Docker & Docker Compose
echo -e "\n${BLUE}[3/6] Verifying Docker Engine and Compose plugin...${NC}"
if ! command -v docker >/dev/null 2>&1; then
  echo -e "${YELLOW}Installing Docker from official repository...${NC}"
  curl -fsSL https://get.docker.com | sh
  systemctl enable --now docker
  echo -e "${GREEN}✓ Docker installed successfully.${NC}"
else
  echo -e "${GREEN}✓ Docker is already installed: $(docker --version)${NC}"
fi

# Ensure compose plugin is ready
if ! docker compose version >/dev/null 2>&1; then
  apt-get update && apt-get install -y docker-compose-plugin
fi

# 5. Create Deploy Directory & Production Config
echo -e "\n${BLUE}[4/6] Creating deployment files in ${DEPLOY_DIR}...${NC}"
mkdir -p "${DEPLOY_DIR}/deploy"

# Write docker-compose.prod.yml
cat << 'EOF' > "${DEPLOY_DIR}/deploy/docker-compose.prod.yml"
# Cardly Production Docker Compose for Oracle Cloud (1 GB RAM / 1 vCPU)
services:
  cardly:
    image: ${CARDLY_IMAGE:-ghcr.io/cardly-anki/cardly:latest}
    container_name: cardly-app
    restart: unless-stopped
    env_file:
      - path: ../.env
        required: true
    environment:
      CARDLY_HOST: "0.0.0.0"
      CARDLY_PORT: "8080"
      CARDLY_DATA_DIR: "/data"
      CARDLY_UPSTREAM_WEB_DIR: "/app/web-upstream"
      CARDLY_SHELL_DIR: "/app/web-shell"
      CARDLY_COOKIE_SECURE: "true"
    volumes:
      - cardly_data:/data
    deploy:
      resources:
        limits:
          cpus: "0.80"
          memory: 450M
        reservations:
          memory: 128M
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
    healthcheck:
      test: ["CMD", "/app/cardly-server", "--healthcheck"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    networks:
      - cardly_internal

  caddy:
    image: caddy:2-alpine
    container_name: cardly-caddy
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
      - "443:443/udp"
    environment:
      DOMAIN: ${DOMAIN:-localhost}
      CADDY_EMAIL: ${CADDY_EMAIL:-}
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    deploy:
      resources:
        limits:
          cpus: "0.20"
          memory: 100M
        reservations:
          memory: 32M
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
    depends_on:
      cardly:
        condition: service_healthy
    networks:
      - cardly_internal

networks:
  cardly_internal:
    driver: bridge

volumes:
  cardly_data:
  caddy_data:
  caddy_config:
EOF

# Write Caddyfile
cat << 'EOF' > "${DEPLOY_DIR}/deploy/Caddyfile"
# Caddyfile for Cardly production reverse proxy with automatic TLS
{
    # Global Caddy options
}

{$DOMAIN:localhost} {
    encode gzip zstd

    # Reverse proxy to Cardly container
    reverse_proxy cardly:8080 {
        header_up X-Forwarded-Proto {scheme}
        header_up X-Forwarded-Host {host}
        header_up X-Real-IP {remote_host}
    }

    # Standard security headers
    header {
        Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
        X-Content-Type-Options "nosniff"
        Referrer-Policy "strict-origin-when-cross-origin"
        -Server
    }
}
EOF

# Write .env if not exists
if [[ ! -f "${DEPLOY_DIR}/.env" ]]; then
  SESSION_SECRET=$(openssl rand -hex 32 2>/dev/null || tr -dc 'a-zA-Z0-9' < /dev/urandom | head -c 64)
  cat << EOF > "${DEPLOY_DIR}/.env"
DOMAIN=${DOMAIN}
CADDY_EMAIL=
CARDLY_IMAGE=ghcr.io/cardly-anki/cardly:latest
CARDLY_HOST=0.0.0.0
CARDLY_PORT=8080
CARDLY_DATA_DIR=/data
CARDLY_COOKIE_SECURE=true
CARDLY_SESSION_SECRET=${SESSION_SECRET}
CARDLY_USER1=demo:demo123
PASSWORDS_HASHED=0

# Demo Mode Settings
CARDLY_DEMO_MODE=true
CARDLY_DEMO_USER=demo:demo123
CARDLY_DEMO_RESET_INTERVAL_SECS=7200
CARDLY_DEMO_AI_ENABLED=false
CARDLY_MAX_UPLOAD_MB=10

RUST_LOG=cardly_server=info,anki=info
EOF
  chmod 600 "${DEPLOY_DIR}/.env"
  echo -e "${GREEN}✓ Created ${DEPLOY_DIR}/.env configuration.${NC}"
else
  echo -e "${GREEN}✓ Preserving existing ${DEPLOY_DIR}/.env.${NC}"
fi

# 6. Configure Systemd Autostart Service
echo -e "\n${BLUE}[5/6] Registering systemd cardly.service...${NC}"
cat << EOF > /etc/systemd/system/cardly.service
[Unit]
Description=Cardly Anki Web Demo & Sync Server
Requires=docker.service
After=docker.service network-online.target
Wants=network-online.target

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=${DEPLOY_DIR}
ExecStart=/usr/bin/docker compose -f deploy/docker-compose.prod.yml up -d
ExecStop=/usr/bin/docker compose -f deploy/docker-compose.prod.yml down
TimeoutStartSec=180

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable cardly.service >/dev/null 2>&1

# 7. Pull image and start containers
echo -e "\n${BLUE}[6/6] Pulling pre-built Docker image and launching services...${NC}"
cd "${DEPLOY_DIR}"
docker compose -f deploy/docker-compose.prod.yml pull
docker compose -f deploy/docker-compose.prod.yml up -d

echo -e "\n${GREEN}${BOLD}🎉 Cardly Demo Instance successfully deployed!${NC}\n"
echo -e "  • ${BOLD}URL:${NC}               ${BLUE}https://${DOMAIN}${NC}"
echo -e "  • ${BOLD}Demo Mode:${NC}         Active (Instant guest login, resets every 2 hours)"
echo -e "  • ${BOLD}Healthcheck:${NC}       curl -f https://${DOMAIN}/healthz"
echo -e "  • ${BOLD}Follow logs:${NC}       docker compose -f ${DEPLOY_DIR}/deploy/docker-compose.prod.yml logs -f"
echo -e "  • ${BOLD}Update image:${NC}      cd ${DEPLOY_DIR} && docker compose -f deploy/docker-compose.prod.yml pull && docker compose -f deploy/docker-compose.prod.yml up -d"
echo -e "  • ${BOLD}Rollback:${NC}          docker compose -f ${DEPLOY_DIR}/deploy/docker-compose.prod.yml down && CARDLY_IMAGE=<PREV_TAG> docker compose -f deploy/docker-compose.prod.yml up -d\n"

echo -e "${YELLOW}${BOLD}IMPORTANT ORACLE CLOUD NOTICE:${NC}"
echo -e "Ensure Ingress ports ${BOLD}80${NC} and ${BOLD}443${NC} (TCP) are open in your Oracle Cloud Console:"
echo -e "  Compute → Virtual Cloud Networks → Security Lists → Default Security List → Add Ingress Rules"
echo -e "  Source CIDR: 0.0.0.0/0 | IP Protocol: TCP | Destination Port Range: 80,443\n"
