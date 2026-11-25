# Cardly Production & Demo Deployment Guide

This guide provides step-by-step instructions for deploying Cardly with automated HTTPS.
The primary production setup is tailored for an **Oracle Cloud Always Free VM** (1 GB RAM, 1 vCPU) running behind automated Let's Encrypt / ZeroSSL TLS via Caddy.

---

## 🖥 Hardware Requirements

Cardly's Rust backend (`rslib`) and static Svelte 5 frontend are designed for high efficiency:
- **CPU**: 1 vCPU (shared is sufficient).
- **RAM**: 512 MB minimum, 1 GB recommended.
- **Swap**: 1–2 GB swap space (critical on 1 GB RAM VMs to prevent OOM spikes).
- **Disk**: 5 GB–10 GB SSD / NVMe.
- **Network**: Ports `80/tcp` (HTTP / ACME challenge) and `443/tcp` (HTTPS / HTTP/3 QUIC).

> [!WARNING]
> **Do NOT compile Rust or run `cargo build` on a 1 GB RAM VM.**
> Compiling upstream Anki's `rslib` and Cardly requires 3–4 GB of memory during linking and will immediately crash low-memory VMs with an Out Of Memory (OOM) kernel panic. Always pull pre-built Docker containers from **GitHub Container Registry (`ghcr.io/cardly-anki/cardly:latest`)**.

---

## 1. Oracle Cloud VM Deployment (1 GB RAM / 1 vCPU)

### Step 1: Ingress Ports in Oracle Cloud Console
Oracle Cloud Compute instances sit behind a Virtual Cloud Network (VCN) firewall. You **must** open ports 80 and 443 in the console:

1. In the [Oracle Cloud Console](https://cloud.oracle.com/), navigate to **Networking** → **Virtual Cloud Networks**.
2. Select your VCN, then click **Security Lists** → **Default Security List for `<your-vcn>`**.
3. Under **Ingress Rules**, click **Add Ingress Rules**:
   - **Source CIDR**: `0.0.0.0/0`
   - **IP Protocol**: `TCP`
   - **Destination Port Range**: `80,443`
   - **Description**: `Allow HTTP/HTTPS for Cardly and Caddy`
4. Click **Add Ingress Rules**.

---

### Step 2: Configure Free Dynamic DNS (DuckDNS)
Cardly requires HTTPS for progressive web app (PWA) installation and offline service workers on iOS Safari and Android Chrome.

If you don't have a custom domain:
1. Go to [duckdns.org](https://www.duckdns.org/) and log in (via GitHub, Google, etc.).
2. In the **domains** section, create a free subdomain (e.g., `cardly-demo`).
3. In the **current ip** field, enter the **Public IPv4 address** of your Oracle Cloud VM.
4. Click **update ip**.
5. Your instance will now resolve at `<YOUR_SUBDOMAIN>.duckdns.org`.

---

### Step 3: Connect via SSH & Harden Access
Connect to your VM using your SSH private key:
```bash
ssh -i ~/.ssh/id_rsa ubuntu@<VM_PUBLIC_IP>
```

#### Basic SSH Hardening:
Disable password-based SSH authentication to prevent brute force attacks:
```bash
sudo sed -i 's/^#*PasswordAuthentication .*/PasswordAuthentication no/' /etc/ssh/sshd_config
sudo systemctl restart ssh || sudo systemctl restart sshd
```

---

### Step 4: Run the Automated Setup Script
Run the automated provisioning script directly on the VM:

```bash
# 1. Download setup script
curl -fsSL https://raw.githubusercontent.com/cardly-anki/cardly/main/scripts/setup-oracle-vm.sh -o setup-oracle-vm.sh

# 2. Execute with your domain
sudo bash setup-oracle-vm.sh <YOUR_SUBDOMAIN>.duckdns.org
```

The script automatically:
1. Configures a **2 GB swap file** (`/swapfile`) with optimized swappiness (`vm.swappiness=10`).
2. Opens ports `80` and `443` in both the Linux kernel **`iptables`** rules and **`ufw`**.
3. Installs Docker Engine and the Docker Compose plugin.
4. Creates `/opt/cardly` with production `docker-compose.prod.yml` and `Caddyfile`.
5. Enforces resource safety limits (`mem_limit: 450M`, `cpus: 0.80`, JSON log rotation).
6. Configures **Demo Mode** (`CARDLY_DEMO_MODE=true`, resets every 2 hours, AI disabled for safety).
7. Pulls the pre-built image from GHCR and starts Caddy + Cardly with automatic TLS.
8. Registers a systemd service (`cardly.service`) so Cardly starts automatically on VM reboot.

---

### Step 5: Verification
Visit your domain in any browser:
```text
https://<YOUR_SUBDOMAIN>.duckdns.org
```

Check the health status:
```bash
curl -f https://<YOUR_SUBDOMAIN>.duckdns.org/healthz
# Response: {"status":"ok"}
```

Check container memory usage:
```bash
docker stats --no-stream
```
*Expected: `cardly-app` consumes ~40–80 MB idle, `cardly-caddy` consumes ~20–35 MB.*

---

## 2. Maintenance, Updates & Rollbacks

### Updating Cardly to the Latest Version:
```bash
cd /opt/cardly
docker compose -f deploy/docker-compose.prod.yml pull
docker compose -f deploy/docker-compose.prod.yml up -d --remove-orphans
```

### Viewing Logs:
```bash
cd /opt/cardly
docker compose -f deploy/docker-compose.prod.yml logs -f --tail=100
```

### Rollback Procedure:
If a newly pulled image causes issues, roll back to a specific previous container image tag:
```bash
cd /opt/cardly
docker compose -f deploy/docker-compose.prod.yml down
# Edit CARDLY_IMAGE in /opt/cardly/.env or override on command line:
CARDLY_IMAGE=ghcr.io/cardly-anki/cardly:<PREVIOUS_GIT_SHA> docker compose -f deploy/docker-compose.prod.yml up -d
```

---

## 3. Automated CI/CD Deploy Workflow (`deploy-demo.yml`)

The repository includes an opt-in GitHub Actions workflow (`.github/workflows/deploy-demo.yml`).
Whenever changes land on the `main` branch:
1. GitHub Actions builds the multi-stage Docker container and publishes it to `ghcr.io/cardly-anki/cardly:latest`.
2. If SSH repository secrets are configured, it automatically logs into the Oracle Cloud VM and restarts the services.

### Setting up Repository Secrets (Optional):
In your GitHub repository, navigate to **Settings** → **Secrets and variables** → **Actions** and add:
- `SSH_HOST`: The public IPv4 address of your Oracle VM.
- `SSH_USER`: The SSH user (usually `ubuntu`).
- `SSH_KEY`: The private SSH key matching the public key on the VM.
- `SSH_PORT`: (Optional) Custom SSH port (defaults to `22`).

*If secrets are not provided, the workflow safely skips the deployment step.*

---

## 4. Alternative PaaS Deployments

### Fly.io
Fly.io runs Docker containers directly on Firecracker microVMs with persistent NVMe volumes:
```bash
fly launch --no-deploy
fly volumes create cardly_data --size 5 --region iad
fly secrets set CARDLY_USER1="alice:YourSecurePassword123"
fly deploy --config deploy/fly.toml
fly certs add anki.yourdomain.com
```

### Railway
Deploy via Railway PaaS:
1. Create a **New Project** → **Deploy from GitHub repo**.
2. Add a persistent volume mounted at `/data`.
3. Set environment variables `CARDLY_HOST=0.0.0.0`, `CARDLY_PORT=8080`, `CARDLY_COOKIE_SECURE=true`.

---

## 5. Demo Mode Architecture

When `CARDLY_DEMO_MODE=true` is set:
- **Instant Guest Login**: Any unauthenticated visitor requesting `/api/auth/me` is immediately issued a signed session cookie as the demo user.
- **Preloaded Sample Decks**: Generates 3 rich sample decks on startup (General Knowledge, Languages, Science & Math with Cloze & MathJax).
- **Automated Data Reset**: Re-seeds the collection and wipes uploaded media on server startup and every `CARDLY_DEMO_RESET_INTERVAL_SECS` (default: 2 hours).
- **Abuse Prevention**:
  - Request upload body capped at 10 MiB (`DEMO_MAX_UPLOAD_BYTES`).
  - Total collection disk space capped at 50 MiB (`DEMO_MAX_STORAGE_BYTES`).
  - `.colpkg` collection replacement and backup restoration are blocked.
  - User deletion and password changes are rejected.
  - AI card generation is disabled by default to protect API quotas.
- **AGPL-3.0 Compliance**: Surfaces a visible banner and footer with direct links to the source repository and non-affiliation disclaimers.

---

## 6. Mobile Synchronization (AnkiDroid & AnkiMobile)

Uses Anki's own sync protocol (verified with Anki Desktop). AnkiDroid and AnkiMobile: not yet tested, reports welcome (see [SYNC_TESTING.md](./SYNC_TESTING.md)).

Once deployed behind HTTPS:
- **Custom Sync Server URL:** `https://<YOUR_DOMAIN>/sync/`
- **Media Sync URL:** `https://<YOUR_DOMAIN>/msync/`
- **Credentials:** Username and password configured in `CARDLY_USER1`.
