# Deployment Guide

**Version:** 3.0.6 | **Last Updated:** 2026-03-28

---

## Quick Start

### Option 1: Local Installation (Development)

```bash
# Clone repository
git clone https://github.com/tang-vu/ContribAI.git
cd ContribAI

# Install with dev dependencies
pip install -e ".[dev]"

# Configure
cp config.example.yaml config.yaml
# Edit config.yaml with your API keys

# Run
contribai hunt
contribai serve  # Start web dashboard at :8787
```

### Option 2: Docker (Production-Ready)

```bash
# Copy configuration
cp config.example.yaml config.yaml
# Edit config.yaml

# Start web dashboard
docker compose up -d dashboard

# Run single analysis
docker compose run --rm runner run

# Start scheduler for automation
docker compose up -d dashboard scheduler
```

### Option 3: Kubernetes (Enterprise)

```bash
# Create namespace
kubectl create namespace contribai

# Deploy
kubectl apply -k kubernetes/overlays/production/

# Check status
kubectl get pods -n contribai
kubectl logs -n contribai -l app=contribai-runner
```

---

## Installation Methods

### Method 1: Pip (Recommended for Development)

```bash
# From GitHub
git clone https://github.com/tang-vu/ContribAI.git
cd ContribAI

# Development install (editable)
pip install -e ".[dev]"

# Production install (minimal deps)
pip install -e "."

# Specific version
pip install contribai==3.0.6
```

**Verification:**

```bash
contribai --version
# ContribAI 3.0.6

contribai --help
# Shows all commands
```

### Method 2: Docker (Recommended for Production)

**Dockerfile included. Build:**

```bash
# Build image locally
docker build -t contribai:3.0.6 .

# Or pull from registry
docker pull tang-vu/contribai:3.0.6
```

**Key Docker images:**

```dockerfile
# Base: Python 3.11 slim
FROM python:3.11-slim

# Install: contribai + dependencies
RUN pip install contribai==3.0.6

# Expose: 8787 (web dashboard)
EXPOSE 8787

# Entry: contribai CLI or server
ENTRYPOINT ["contribai"]
```

### Method 3: Kubernetes Helm (Enterprise)

```bash
# Add Helm repo (future)
helm repo add contribai https://charts.contribai.dev
helm repo update

# Install
helm install contribai contribai/contribai \
  --namespace contribai \
  --create-namespace \
  --values values.yaml

# Check deployment
kubectl get deployment -n contribai
kubectl logs -n contribai deployment/contribai-runner
```

---

## Configuration

### Configuration File Structure

Create `config.yaml` from `config.example.yaml`:

```bash
cp config.example.yaml config.yaml
```

**Full configuration schema:**

```yaml
# GitHub Configuration
github:
  token: "ghp_..."                    # Personal Access Token (required)
  max_prs_per_day: 15                 # Safety limit (1-100)
  rate_limit_margin: 100              # Buffer before hitting API limit
  fork_timeout_seconds: 30            # Timeout for fork operation

# LLM Configuration
llm:
  provider: "gemini"                  # gemini | openai | anthropic | ollama | vertex
  model: "gemini-2.5-flash"           # Model ID
  api_key: "your_api_key"             # API key for provider
  temperature: 0.5                    # Creativity (0.0-2.0)
  max_tokens: 2000                    # Max response length
  timeout_seconds: 60                 # Request timeout

# Discovery Configuration
discovery:
  languages:                          # Target languages
    - python
    - javascript
  stars_range:                        # Star range filter
    - 100
    - 5000
  min_activity_days: 180              # Require recent commits
  exclude_repos: []                   # Repos to skip

# Analysis Configuration
analysis:
  enabled_analyzers:                  # Which analyzers to run
    - security
    - code_quality
    - performance
    - documentation
    - ui_ux
    - refactoring
  max_file_size_kb: 50                # Skip large files
  skip_patterns:                      # File patterns to skip
    - "*.md"
    - "*.yaml"
    - "*.json"

# Contribution Configuration
contribution:
  pr_style: "professional"            # professional | minimal
  commit_format: "conventional"       # conventional | simple
  include_explanation: true           # Add explanation to PR body

# Pipeline Configuration
pipeline:
  concurrent_repos: 3                 # Max simultaneous repos
  retry_attempts: 2                   # Retry on failure
  retry_backoff_seconds: 2            # Backoff multiplier
  timeout_seconds: 300                # Operation timeout

# Multi-Model Task Routing
multi_model:
  task_routing:
    analysis: "economy"               # Fast + cheap
    generation: "performance"         # Powerful + expensive
    review: "balanced"                # Medium tier

# Notifications
notifications:
  enabled: true
  channels:
    slack: "https://hooks.slack.com/services/..."
    discord: "https://discord.com/api/webhooks/..."
    telegram: "https://api.telegram.org/bot..."

# Web Dashboard
web:
  host: "0.0.0.0"
  port: 8787
  debug: false
  api_auth_key: "your-secret-key"

# Scheduler
scheduler:
  enabled: true
  timezone: "UTC"
  max_concurrent_jobs: 3
```

### Environment Variables (Override YAML)

All config keys can be set via environment variables with `CONTRIBAI_` prefix:

```bash
export CONTRIBAI_GITHUB_TOKEN="ghp_..."
export CONTRIBAI_LLM_PROVIDER="gemini"
export CONTRIBAI_LLM_API_KEY="your_api_key"
export CONTRIBAI_LLM_MODEL="gemini-2.5-flash"
export CONTRIBAI_GITHUB_MAX_PRS_PER_DAY="15"
export CONTRIBAI_DISCOVERY_LANGUAGES="python,javascript"
export CONTRIBAI_WEB_PORT="8787"
```

**Precedence:** CLI flags > Env vars > config.yaml > Defaults

### Profile Presets

Use pre-configured profiles instead of editing config:

```bash
# List available profiles
contribai profile list

# Run with profile
contribai profile security-focused
contribai profile docs-focused
contribai profile full-scan
contribai profile gentle
```

**Profile definitions in `contribai/core/profiles.py`:**

```python
PROFILES = {
    "security-focused": {
        "enabled_analyzers": ["security"],
        "max_prs_per_day": 5,
        "temperature": 0.2,
    },
    "docs-focused": {
        "enabled_analyzers": ["documentation"],
        "max_prs_per_day": 10,
    },
    "full-scan": {
        "enabled_analyzers": [
            "security", "code_quality", "performance",
            "documentation", "ui_ux", "refactoring"
        ],
        "max_prs_per_day": 20,
    },
    "gentle": {
        "enabled_analyzers": ["code_quality"],
        "max_prs_per_day": 3,
        "temperature": 0.3,
    },
}
```

---

## Environment Variables

### Required

| Variable | Description | Example |
|----------|-------------|---------|
| `CONTRIBAI_GITHUB_TOKEN` | GitHub PAT (required) | `ghp_abc123...` |
| `CONTRIBAI_LLM_API_KEY` | LLM provider API key | `AIzaSy...` (Gemini) |

### Optional (Override config.yaml)

| Variable | Description | Example |
|----------|-------------|---------|
| `CONTRIBAI_LLM_PROVIDER` | LLM provider | `gemini`, `openai`, `anthropic` |
| `CONTRIBAI_LLM_MODEL` | Model ID | `gemini-2.5-flash` |
| `CONTRIBAI_GITHUB_MAX_PRS_PER_DAY` | Daily PR limit | `15` |
| `CONTRIBAI_WEB_PORT` | Dashboard port | `8787` |
| `CONTRIBAI_DISCOVERY_LANGUAGES` | Target languages | `python,javascript` |
| `CONTRIBAI_SCHEDULER_ENABLED` | Enable scheduler | `true`, `false` |

### System

| Variable | Default | Description |
|----------|---------|-------------|
| `CONTRIBAI_HOME` | `~/.contribai` | Config + data directory |
| `CONTRIBAI_LOG_LEVEL` | `INFO` | Logging level |
| `CONTRIBAI_LOG_FILE` | `~/.contribai/app.log` | Log file path |

---

## Docker Deployment

### Docker Compose (Easiest)

```yaml
# docker-compose.yml (included)
version: '3.8'

services:
  dashboard:
    image: tang-vu/contribai:3.0.6
    command: serve
    ports:
      - "8787:8787"
    environment:
      CONTRIBAI_GITHUB_TOKEN: ${GITHUB_TOKEN}
      CONTRIBAI_LLM_API_KEY: ${LLM_API_KEY}
    volumes:
      - ./config.yaml:/app/config.yaml
      - contribai-data:/root/.contribai
    restart: unless-stopped

  runner:
    image: tang-vu/contribai:3.0.6
    command: run
    environment:
      CONTRIBAI_GITHUB_TOKEN: ${GITHUB_TOKEN}
      CONTRIBAI_LLM_API_KEY: ${LLM_API_KEY}
    volumes:
      - ./config.yaml:/app/config.yaml
      - contribai-data:/root/.contribai
    depends_on:
      - dashboard
    restart: unless-stopped

  scheduler:
    image: tang-vu/contribai:3.0.6
    command: schedule --cron "0 */6 * * *"  # Every 6 hours
    environment:
      CONTRIBAI_GITHUB_TOKEN: ${GITHUB_TOKEN}
      CONTRIBAI_LLM_API_KEY: ${LLM_API_KEY}
    volumes:
      - ./config.yaml:/app/config.yaml
      - contribai-data:/root/.contribai
    depends_on:
      - dashboard
    restart: unless-stopped

volumes:
  contribai-data:
```

**Usage:**

```bash
# Copy config
cp config.example.yaml config.yaml

# Set environment variables
export GITHUB_TOKEN="ghp_..."
export LLM_API_KEY="AIzaSy..."

# Start dashboard
docker compose up -d dashboard

# View logs
docker compose logs -f dashboard

# Run one-shot analysis
docker compose run --rm runner run

# Stop services
docker compose down

# Clean up data
docker compose down -v  # Remove volumes too
```

### Docker Build (Manual)

```bash
# Build image
docker build -t contribai:latest .

# Run dashboard
docker run -d \
  -p 8787:8787 \
  -e CONTRIBAI_GITHUB_TOKEN="ghp_..." \
  -e CONTRIBAI_LLM_API_KEY="AIzaSy..." \
  -v $(pwd)/config.yaml:/app/config.yaml \
  -v contribai-data:/root/.contribai \
  contribai:latest serve

# Run analysis
docker run --rm \
  -e CONTRIBAI_GITHUB_TOKEN="ghp_..." \
  -e CONTRIBAI_LLM_API_KEY="AIzaSy..." \
  -v $(pwd)/config.yaml:/app/config.yaml \
  -v contribai-data:/root/.contribai \
  contribai:latest run
```

---

## CLI Commands Reference

### Core Commands

```bash
# Autonomous hunting (multi-round discovery + contributions)
contribai hunt                                # Hunt for repos and contribute
contribai hunt --rounds 5                     # 5 rounds
contribai hunt --delay 15                     # 15min inter-round delay
contribai hunt --mode both                    # analysis + issues (default)
contribai hunt --mode analysis                # analysis only
contribai hunt --mode issues                  # issues only

# Single full pipeline run
contribai run                                 # Full pipeline on discovered repos
contribai run --dry-run                       # Preview without PRs
contribai run --language python               # Filter by language

# Target specific repo
contribai target https://github.com/owner/repo
contribai target https://github.com/owner/repo --dry-run

# Solve issues
contribai solve https://github.com/owner/repo # Fetch & solve open issues
```

### Web & Automation

```bash
# Start web dashboard
contribai serve                               # Dashboard at :8787
contribai serve --port 9000                   # Custom port
contribai serve --host 0.0.0.0                # Listen on all interfaces

# Start scheduler
contribai schedule --cron "0 */6 * * *"       # Every 6 hours
contribai schedule --cron "0 9 * * *"         # Daily at 9 AM
contribai schedule --once                     # Run once, then exit
```

### Templates & Profiles

```bash
# List templates
contribai templates

# List profiles
contribai profile list

# Run with profile
contribai run --profile security-focused
contribai hunt --profile docs-focused
```

### Status & Cleanup

```bash
# Check overall status
contribai status                              # Active PRs + stats
contribai stats                               # Summary statistics
contribai info                                # System information

# Cleanup stale forks
contribai cleanup                             # Remove forks with no open PRs
contribai cleanup --dry-run                   # Preview
```

### Utilities

```bash
# Check config
contribai config                              # Show effective config

# Test LLM connection
contribai test-llm                            # Verify LLM provider

# Test GitHub access
contribai test-github                         # Verify GitHub token

# View help
contribai --help
contribai run --help
```

---

## Web Dashboard

### Access

- **URL:** `http://localhost:8787`
- **API:** `http://localhost:8787/api`
- **Status:** Check `GET /api/stats`

### Web Routes

| Route | Method | Purpose |
|-------|--------|---------|
| `GET /` | GET | Dashboard UI |
| `GET /api/stats` | GET | Overall statistics |
| `GET /api/repos` | GET | Analyzed repositories |
| `GET /api/prs` | GET | Created pull requests |
| `GET /api/runs` | GET | Pipeline execution history |
| `POST /api/run` | POST | Trigger pipeline run |
| `POST /api/stop` | POST | Stop running pipeline |
| `GET /api/health` | GET | Health check |
| `POST /webhooks/github` | POST | GitHub webhook receiver |

### API Authentication

Dashboard API requires authentication for mutations:

```bash
# Set API key in config.yaml
api_auth_key: "your-secret-key"

# Or via environment
export CONTRIBAI_WEB_API_AUTH_KEY="your-secret-key"

# Use in API calls
curl -H "X-API-Key: your-secret-key" -X POST http://localhost:8787/api/run
```

### Webhook Setup (GitHub)

1. Go to repo settings → Webhooks
2. Payload URL: `http://your-server:8787/webhooks/github`
3. Content type: `application/json`
4. Events: `push`, `pull_request`, `issues`
5. Secret: (optional, validated if set)

**Webhook triggers:**
- On issue created → Solve issue
- On PR comment → Patrol + auto-fix
- On push → Reanalyze changed files

---

## Scheduler Setup

### Cron Syntax

```bash
# Every 6 hours
contribai schedule --cron "0 */6 * * *"

# Daily at 9 AM UTC
contribai schedule --cron "0 9 * * *"

# Every Monday at 6 PM
contribai schedule --cron "0 18 * * 1"

# Every 30 minutes
contribai schedule --cron "*/30 * * * *"

# Every weekday at 8 AM
contribai schedule --cron "0 8 * * 1-5"
```

### Systemd Integration (Linux)

```ini
# /etc/systemd/system/contribai-scheduler.service
[Unit]
Description=ContribAI Scheduler
After=network.target

[Service]
Type=simple
User=contribai
WorkingDirectory=/home/contribai/contribai
ExecStart=/usr/local/bin/contribai schedule --cron "0 */6 * * *"
Restart=always
RestartSec=10

Environment="CONTRIBAI_GITHUB_TOKEN=ghp_..."
Environment="CONTRIBAI_LLM_API_KEY=AIzaSy..."

[Install]
WantedBy=multi-user.target
```

**Enable & start:**

```bash
sudo systemctl daemon-reload
sudo systemctl enable contribai-scheduler
sudo systemctl start contribai-scheduler
sudo systemctl status contribai-scheduler
```

### Kubernetes CronJob

```yaml
# kubernetes/cronjob.yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: contribai-run
spec:
  schedule: "0 */6 * * *"  # Every 6 hours
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: contribai
            image: tang-vu/contribai:3.0.6
            command: ["contribai", "run"]
            env:
            - name: CONTRIBAI_GITHUB_TOKEN
              valueFrom:
                secretKeyRef:
                  name: contribai-secrets
                  key: github-token
            - name: CONTRIBAI_LLM_API_KEY
              valueFrom:
                secretKeyRef:
                  name: contribai-secrets
                  key: llm-api-key
            volumeMounts:
            - name: config
              mountPath: /etc/contribai
          volumes:
          - name: config
            configMap:
              name: contribai-config
          restartPolicy: OnFailure
```

---

## Troubleshooting

### Common Issues

| Issue | Cause | Fix |
|-------|-------|-----|
| `401 Unauthorized (GitHub)` | Invalid token | Check token format, ensure scopes |
| `429 Too Many Requests` | API rate limit hit | Reduce concurrent repos, add delays |
| `RESOURCE_EXHAUSTED (LLM)` | LLM rate limit | Use economy model, reduce max_tokens |
| `Connection refused` | Service not running | Start service: `contribai serve` |
| `Database locked` | Concurrent access conflict | Wait, then retry |
| `ImportError: No module named 'contribai'` | Not installed | `pip install -e ".[dev]"` |

### Debug Mode

```bash
# Enable debug logging
contribai run -v                              # Verbose mode
contribai run -vv                             # Very verbose

# Set environment
export CONTRIBAI_LOG_LEVEL=DEBUG
contribai run

# Check logs
tail -f ~/.contribai/app.log
```

### Health Checks

```bash
# Dashboard health
curl http://localhost:8787/api/health

# LLM provider
contribai test-llm

# GitHub access
contribai test-github

# Database
python -c "
import asyncio
from contribai.orchestrator.memory import Memory
async def check():
    mem = Memory()
    await mem.init()
    stats = await mem.get_stats()
    print(stats)
asyncio.run(check())
"
```

### Performance Tuning

```yaml
# config.yaml optimizations

# Reduce concurrent processing
pipeline:
  concurrent_repos: 1  # Instead of 3

# Faster LLM responses
llm:
  model: "gemini-2.5-flash"  # Fast model
  temperature: 0.3           # Less creative = faster
  max_tokens: 1000           # Shorter responses

# Skip slow analyzers
analysis:
  enabled_analyzers:
    - code_quality           # Fast
    - security               # Fast
  # Skip: performance (slow), ui_ux (slow)

# Reduce discovery scope
discovery:
  languages: ["python"]      # Single language
  stars_range: [1000, 10000] # Narrower range
```

---

## Scaling & Production

### Single-Instance Production

**Recommended setup:**

```yaml
# config.yaml
github:
  max_prs_per_day: 20      # Conservative limit
  rate_limit_margin: 200   # Higher safety margin

llm:
  provider: "gemini"
  model: "gemini-2.5-flash"
  temperature: 0.3         # Deterministic output

pipeline:
  concurrent_repos: 3      # 3 parallel repos
  retry_attempts: 3        # More retry attempts
  timeout_seconds: 600     # 10 min timeout

notifications:
  enabled: true            # Alert on issues
  channels:
    slack: "..."           # Monitor channel
```

**Monitoring:**

```bash
# Check status regularly
watch -n 60 'curl http://localhost:8787/api/stats | jq'

# Monitor logs
journalctl -u contribai-scheduler -f

# Database integrity
sqlite3 ~/.contribai/memory.db "PRAGMA integrity_check;"
```

### Multi-Instance Deployment (Future v3.1)

```yaml
# Planned: PostgreSQL backend
database:
  type: "postgresql"
  url: "postgresql://user:pass@db-host:5432/contribai"

# Planned: Redis for distributed rate limiting
cache:
  type: "redis"
  url: "redis://cache-host:6379"
```

---

## Security Checklist

- [ ] GitHub token stored in env vars (not in code)
- [ ] LLM API key stored in env vars (not in code)
- [ ] API auth key set for web dashboard
- [ ] GitHub webhook secret configured
- [ ] HTTPS enabled (in reverse proxy, not in app)
- [ ] Firewall restricts access to :8787
- [ ] Database backups configured
- [ ] Logs rotated (don't accumulate indefinitely)
- [ ] Dependencies audited (`pip-audit`)
- [ ] No secrets in config.yaml template

---

## Document Metadata

- **Created:** 2026-03-28
- **Last Updated:** 2026-03-28
- **Owner:** DevOps / Deployment Team
- **References:** README.md, docs/code-standards.md, docker-compose.yml, Dockerfile
