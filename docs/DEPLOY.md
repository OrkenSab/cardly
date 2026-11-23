# Cardly Production Deployment Guide

This guide provides battle-tested, step-by-step instructions for deploying Cardly to production across three environments:
1. **Self-Hosted VPS** ($4–$5/mo on Hetzner, DigitalOcean, Linode, or Vultr) with automated HTTPS via Caddy.
2. **Fly.io** (Serverless container with persistent NVMe volume).
3. **Railway** (PaaS one-click git-push deployment).

---

## 🖥 Hardware Requirements

Cardly's Rust backend (`rslib`) and static Svelte 5 frontend are designed for high efficiency:
- **CPU**: 1 vCPU (shared is sufficient).
- **RAM**: 512 MB minimum, 1 GB recommended for collections with >100,000 cards.
- **Disk**: 5 GB–10 GB SSD / NVMe (depending on media collection size).
- **Network**: Ports `80/tcp` (HTTP) and `443/tcp` (HTTPS / HTTP/3 QUIC).

---

## 1. Deploying on a Cheap VPS ($4–$5/mo) with Docker & Caddy

A $4–$5/mo VPS (such as a Hetzner Cloud CX22 or DigitalOcean Basic Droplet) is the recommended setup for individuals, families, and study groups.

### Step 1: Server Preparation & Firewall
Connect to your fresh Ubuntu 22.04 or 24.04 LTS instance:
```bash
ssh root@<YOUR-SERVER-IP>
apt-get update && apt-get upgrade -y
apt-get install -y ca-certificates curl git ufw

# Configure firewall
ufw default deny incoming
ufw default allow outgoing
ufw allow 22/tcp    # SSH
ufw allow 80/tcp    # HTTP (Let's Encrypt challenge & redirect)
ufw allow 443/tcp   # HTTPS
ufw allow 443/udp   # HTTP/3 QUIC
ufw enable
```

### Step 2: Install Docker & Docker Compose
```bash
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh
```

### Step 3: Clone Repository
```bash
git clone --recurse-submodules https://github.com/cardly-app/cardly.git /opt/cardly
cd /opt/cardly
```

### Step 4: Configure Production Environment (`.env`)
Create `/opt/cardly/.env`:
```ini
# Domain configuration for Caddy automatic TLS
DOMAIN=anki.example.com
CADDY_EMAIL=admin@example.com

# Cardly container settings
CARDLY_PORT=8080
CARDLY_HOST=0.0.0.0
CARDLY_DATA_DIR=/data
CARDLY_COOKIE_SECURE=true

# User accounts (username:password)
CARDLY_USER1=alice:MakeThisAStrongPassword123!
# Additional accounts:
# CARDLY_USER2=bob:AnotherStrongPassword456!

# Upload limit in MiB
CARDLY_MAX_UPLOAD_MB=512

# Tracing log filter
RUST_LOG=cardly_server=info,anki=info
```

### Step 5: Start Services with Automated HTTPS
Launch Cardly and Caddy using `deploy/docker-compose.prod.yml`:
```bash
docker compose -f deploy/docker-compose.prod.yml up -d
```
Caddy will automatically:
- Request and install a TLS certificate from Let's Encrypt or ZeroSSL.
- Redirect all port 80 HTTP requests to port 443 HTTPS.
- Reverse proxy traffic to the Cardly backend container.

### Step 6: Verify Deployment
Visit your domain in any modern browser:
```text
https://anki.example.com
```
And check server health:
```bash
curl -f https://anki.example.com/healthz
# Output: {"status":"ok"}
```

### Step 7: Systemd Autostart Service
To ensure Cardly starts automatically on server reboot:
```bash
cat << 'EOF' > /etc/systemd/system/cardly.service
[Unit]
Description=Cardly Anki Web & Sync Server
Requires=docker.service
After=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=/opt/cardly
ExecStart=/usr/bin/docker compose -f deploy/docker-compose.prod.yml up -d
ExecStop=/usr/bin/docker compose -f deploy/docker-compose.prod.yml down
TimeoutStartSec=0

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable cardly.service
```

---

## 2. Deploying to Fly.io

Fly.io runs Docker containers directly on Firecracker microVMs near your users with persistent NVMe volumes.

### Step 1: Install Flyctl & Log In
```bash
curl -L https://fly.io/install.sh | sh
fly auth login
```

### Step 2: Initialize App
Inside the Cardly repository root:
```bash
# Initialize fly configuration without deploying yet
fly launch --no-deploy
```

### Step 3: Create Persistent Volume
Cardly stores SQLite collections and media files in `/data`. Create a persistent volume:
```bash
fly volumes create cardly_data --size 5 --region iad
```

### Step 4: Set Environment Secrets
```bash
fly secrets set \
  CARDLY_USER1="alice:YourSecurePassword123" \
  PASSWORDS_HASHED="0"
```

### Step 5: Deploy
Deploy using the configuration in [`deploy/fly.toml`](../deploy/fly.toml):
```bash
fly deploy --config deploy/fly.toml
```

### Step 6: Custom Domain & Free SSL
```bash
fly certs add anki.yourdomain.com
# Follow the DNS CNAME/A instructions provided by flyctl
```

---

## 3. Deploying to Railway

Railway offers frictionless GitHub integration and managed deployments.

### Step 1: Create a Railway Project
1. Log in to [Railway](https://railway.app/).
2. Click **New Project** → **Deploy from GitHub repo**.
3. Select your Cardly repository.

### Step 2: Attach Persistent Volume
1. Go to the service settings in Railway.
2. Under **Volumes**, click **Add Volume**.
3. Set **Mount Path** to `/data`.

### Step 3: Configure Variables
In the service **Variables** tab, set:
- `CARDLY_HOST`: `0.0.0.0`
- `CARDLY_PORT`: `8080`
- `CARDLY_DATA_DIR`: `/data`
- `CARDLY_USER1`: `alice:YourPasswordHere`
- `CARDLY_COOKIE_SECURE`: `true`

Railway automatically provisions an SSL certificate and routes HTTPS traffic to port `8080`.

---

## 4. Enabling Demo Mode (`CARDLY_DEMO_MODE=true`)

Cardly features a built-in **Demo Mode** suitable for public showcase instances and classroom trials:
- Seeds rich sample decks on startup (General Knowledge, Languages, Science with Cloze and MathJax).
- Automatically resets the demo collection back to pristine condition **every 60 minutes**.
- Enforces rate limiting (120 req/min) and blocks dangerous administrative actions (backup restoration, collection overwriting).
- Surfaces a subtle indicator badge in the web UI.

### Activation:
In your `.env` (or environment secrets):
```ini
CARDLY_DEMO_MODE=true
CARDLY_DEMO_USER=demo:demo123
```
Users can sign in with username `demo` and password `demo123` to test all Cardly features without risk of modifying private collections.

---

## 5. Mobile Synchronization (AnkiDroid & AnkiMobile)

Cardly contains a native Anki-compatible sync server implementation. Once deployed with HTTPS, configure your mobile devices:

### AnkiDroid (Android)
1. Open **AnkiDroid** → **Settings** (`⋮`) → **Syncing** → **Custom sync server**.
2. Set **Sync URL**: `https://anki.example.com/sync/`
3. Set **Media sync URL**: `https://anki.example.com/msync/`
4. Set **Username** and **Password** to match `CARDLY_USER1`.
5. Tap **Sync** on the main deck screen.

### AnkiMobile (iOS)
1. Open **AnkiMobile** → **Preferences** (gear icon) → **Syncing**.
2. Tap **Custom Sync Server**.
3. Enter `https://anki.example.com/sync/` for the sync server.
4. Log in with your credentials.

---

## 6. Backup & Disaster Recovery

Cardly automatically creates backups and keeps the 50 most recent snapshots under `/data/users/<username>/backups/*.colpkg`.

### Automated Offsite Backups Script
To mirror backups offsite daily (e.g. to AWS S3, Cloudflare R2, or local storage):
```bash
cat << 'EOF' > /opt/cardly/backup.sh
#!/bin/bash
BACKUP_DIR="/var/lib/docker/volumes/cardly_cardly_data/_data/users"
DEST_DIR="/backup/cardly"
mkdir -p "$DEST_DIR"
rsync -av --include="*/" --include="backups/**" --exclude="*" "$BACKUP_DIR/" "$DEST_DIR/"
EOF
chmod +x /opt/cardly/backup.sh
```
Add to crontab (`crontab -e`):
```cron
0 3 * * * /opt/cardly/backup.sh >/dev/null 2>&1
```

### Manual Restoration
To restore a `.colpkg` snapshot:
1. Open Cardly Web → **Settings / Maintenance** → **Backups**.
2. Click **Restore** next to any historical snapshot.
3. Or manually copy the snapshot into a fresh `/data/users/<username>/` directory and restart the container.
