#!/usr/bin/env bash
# Copyright: Cardly contributors
# License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html
#
# Cardly — Clean-Machine One-Command Quickstart Script

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# ANSI color styling
BOLD='\033[1m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

print_banner() {
  cat << "EOF"
  ____                _ _       
 / ___|__ _ _ __   __| | |_   _ 
| |   / _` | '__| / _` | | | | |
| |__| (_| | |   | (_| | | |_| |
 \____\__,_|_|    \__,_|_|\__, |
                          |___/ 
Open-Source Anki Desktop Web Client & Sync Server
===================================================
EOF
}

usage() {
  cat << EOF
Usage: ./scripts/quickstart.sh [OPTIONS]

Options:
  --build      Force build of Docker image locally
  --check      Verify environment, tools, and configuration without starting
  --help       Show this help message

Quickstart:
  $ ./scripts/quickstart.sh
EOF
}

FORCE_BUILD=false
CHECK_ONLY=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --build)
      FORCE_BUILD=true
      shift
      ;;
    --check)
      CHECK_ONLY=true
      shift
      ;;
    --help|-h)
      print_banner
      usage
      exit 0
      ;;
    *)
      echo -e "${RED}Unknown option: $1${NC}"
      usage
      exit 1
      ;;
  esac
done

print_banner

echo -e "${BLUE}[1/5] Checking prerequisites...${NC}"

# Check Docker CLI
if ! command -v docker >/dev/null 2>&1; then
  echo -e "${RED}Error: 'docker' is not installed or not in PATH.${NC}"
  echo "Please install Docker: https://docs.docker.com/get-docker/"
  exit 1
fi

# Check Docker Compose (v2 plugin or standalone)
COMPOSE_CMD=""
if docker compose version >/dev/null 2>&1; then
  COMPOSE_CMD="docker compose"
elif command -v docker-compose >/dev/null 2>&1; then
  COMPOSE_CMD="docker-compose"
else
  echo -e "${RED}Error: Neither 'docker compose' nor 'docker-compose' found.${NC}"
  echo "Please install Docker Compose: https://docs.docker.com/compose/install/"
  exit 1
fi
echo -e "${GREEN}✓ Docker and Compose detected: ${COMPOSE_CMD}${NC}"

# Check upstream git submodule initialization
if [[ ! -f "anki-upstream/ninja" ]]; then
  echo -e "${YELLOW}Notice: Upstream Anki submodule not detected. Initializing git submodules...${NC}"
  git submodule update --init --recursive
  echo -e "${GREEN}✓ Initialized anki-upstream submodule.${NC}"
fi

echo -e "\n${BLUE}[2/5] Checking configuration (.env)...${NC}"
if [[ ! -f ".env" ]]; then
  if [[ -f ".env.example" ]]; then
    echo -e "${YELLOW}Notice: .env not found. Creating from .env.example with secure defaults...${NC}"
    cp .env.example .env
    # Ensure default user alice exists and is configured
    if ! grep -q "^CARDLY_USER1=" .env; then
      echo "CARDLY_USER1=alice:cardly123" >> .env
    fi
    echo -e "${GREEN}✓ Created .env file (default account: alice:cardly123)${NC}"
  else
    echo -e "${RED}Error: Neither .env nor .env.example was found in ${ROOT_DIR}${NC}"
    exit 1
  fi
else
  echo -e "${GREEN}✓ Existing .env file found.${NC}"
fi

# Read CARDLY_PORT and users from .env
CARDLY_PORT="$(grep -E '^CARDLY_PORT=' .env 2>/dev/null | cut -d'=' -f2- | tr -d ' "\r' || echo "")"
CARDLY_PORT="${CARDLY_PORT:-8080}"

PRIMARY_USER="$(grep -E '^CARDLY_USER1=' .env 2>/dev/null | cut -d'=' -f2- | tr -d ' "\r' || echo "")"
if [[ -z "$PRIMARY_USER" ]]; then
  PRIMARY_USER="$(grep -E '^SYNC_USER1=' .env 2>/dev/null | cut -d'=' -f2- | tr -d ' "\r' || echo "")"
fi

echo -e "\n${BLUE}[3/5] Validating docker-compose configuration...${NC}"
$COMPOSE_CMD config >/dev/null
echo -e "${GREEN}✓ Compose configuration is valid.${NC}"

if [[ "$CHECK_ONLY" == "true" ]]; then
  echo -e "\n${GREEN}✓ All checks passed! System is ready to run Cardly.${NC}"
  echo -e "Run without --check to start the service:"
  echo -e "  ${BOLD}./scripts/quickstart.sh${NC}"
  exit 0
fi

echo -e "\n${BLUE}[4/5] Starting Cardly containers...${NC}"
COMPOSE_ARGS=("up" "-d")
if [[ "$FORCE_BUILD" == "true" ]]; then
  COMPOSE_ARGS+=("--build")
fi

$COMPOSE_CMD "${COMPOSE_ARGS[@]}"

echo -e "\n${BLUE}[5/5] Waiting for Cardly server to become healthy...${NC}"
HEALTH_URL="http://127.0.0.1:${CARDLY_PORT}/healthz"
MAX_ATTEMPTS=45
ATTEMPT=0
HEALTHY=false

while [[ $ATTEMPT -lt $MAX_ATTEMPTS ]]; do
  ATTEMPT=$((ATTEMPT + 1))
  if curl -s -f "$HEALTH_URL" >/dev/null 2>&1; then
    HEALTHY=true
    break
  fi
  printf "."
  sleep 2
done
printf "\n"

if [[ "$HEALTHY" != "true" ]]; then
  echo -e "${RED}Warning: Server did not respond within $((MAX_ATTEMPTS * 2)) seconds.${NC}"
  echo "Check container logs using:"
  echo -e "  ${BOLD}${COMPOSE_CMD} logs -f${NC}"
  exit 1
fi

HOST_IP="$(hostname -I 2>/dev/null | awk '{print $1}' || echo "127.0.0.1")"

echo -e "\n${GREEN}${BOLD}🎉 Cardly is up and running!${NC}\n"
echo -e "  ${BOLD}Web Application:${NC}           ${CYAN}http://localhost:${CARDLY_PORT}${NC}"
if [[ "$HOST_IP" != "127.0.0.1" ]]; then
  echo -e "  ${BOLD}Local Network URL:${NC}         ${CYAN}http://${HOST_IP}:${CARDLY_PORT}${NC}"
fi

if [[ -n "$PRIMARY_USER" ]]; then
  IFS=':' read -r USER_NAME USER_PASS <<< "$PRIMARY_USER"
  echo -e "  ${BOLD}Default User:${NC}              ${YELLOW}${USER_NAME}${NC}"
fi

echo -e "\n${BOLD}Mobile Sync Server Configuration:${NC}"
echo -e "  • ${BOLD}AnkiDroid / AnkiMobile Custom Sync URL:${NC} ${CYAN}http://${HOST_IP}:${CARDLY_PORT}/sync/${NC}"
echo -e "  • ${BOLD}Media Sync URL:${NC}                         ${CYAN}http://${HOST_IP}:${CARDLY_PORT}/msync/${NC}"

echo -e "\n${BOLD}Useful Commands:${NC}"
echo -e "  • Follow logs:   ${BOLD}${COMPOSE_CMD} logs -f${NC}"
echo -e "  • Stop Cardly:   ${BOLD}${COMPOSE_CMD} down${NC}"
echo -e "  • Restart:       ${BOLD}${COMPOSE_CMD} restart${NC}"
echo ""
