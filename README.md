# ContribAI

> **Autonomous AI agent that discovers, analyzes, and submits Pull Requests to open source projects on GitHub**

[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-335%20passed-brightgreen)](#testing)
[![Version](https://img.shields.io/badge/version-5.2.0-blue)](https://github.com/tang-vu/ContribAI/releases)

### 🏆 Results

| Metric | Count |
|--------|-------|
| **PRs Submitted** | 43+ |
| **PRs Merged** | 9 |
| **Repos Contributed** | 21+ |
| **Notable Repos** | Worldmonitor (45k⭐), Maigret (19k⭐), AI-Research-SKILLs (6k⭐), s-tui (5k⭐) |

> Set it up once, wake up to merged PRs. See the [**Hall of Fame →**](HALL_OF_FAME.md)

ContribAI discovers open source repos, analyzes code for improvements, generates fixes, and submits Pull Requests — all autonomously. **v5.2.0 is written in Rust** for ~10–50× faster analysis and a ~4.5 MB single binary.

```
  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
  │ Discovery│───▶│ Analysis │───▶│Generator │───▶│ PR + CI  │───▶│ Patrol   │
  │          │    │ 17 skills│    │ LLM +    │    │ Fork,    │    │ Auto-fix │
  │ Find repos│    │ Security │    │ self-    │    │ commit,  │    │ review   │
  │ by lang, │    │ quality, │    │ review,  │    │ create   │    │ feedback │
  │ stars    │    │ perf     │    │ scoring  │    │ PR + CLA │    │ & reply  │
  └──────────┘    └──────────┘    └──────────┘    └──────────┘    └──────────┘
```

## Quick Start

```bash
# One-line install (auto-detects OS/arch)
curl -fsSL https://raw.githubusercontent.com/tang-vu/ContribAI/main/install.sh | bash
# Windows: irm https://raw.githubusercontent.com/tang-vu/ContribAI/main/install.ps1 | iex

# Or build from source
git clone https://github.com/tang-vu/ContribAI.git && cd ContribAI
cargo install --path crates/contribai-rs

# Configure
contribai init                  # Interactive setup wizard
contribai login                 # Verify auth + switch providers

# Run
contribai hunt              # Autonomous: discover repos → analyze → PR
contribai target <repo_url> # Target a specific repo
contribai hunt --dry-run    # Preview without creating PRs
contribai interactive       # Browse PRs/repos in ratatui TUI
```

## Features

| Category | Highlights |
|----------|-----------|
| **Analysis** | Security (SQLi, XSS, resource leak), code quality, performance, docs, complexity |
| **LLM** | Gemini, OpenAI, Anthropic, Ollama, Vertex AI — smart task routing across model tiers |
| **Hunt Mode** | Multi-round autonomous hunting, issue-first strategy, cross-file fixes |
| **PR Patrol** | Monitors PRs for review feedback, auto-responds and pushes fixes |
| **Interactive TUI** | ratatui 4-tab browser: Dashboard / PRs / Repos / Actions |
| **MCP Server** | 21 tools for Claude Desktop via stdio JSON-RPC |
| **Safety** | AI policy detection, CLA auto-signing, quality gate, duplicate prevention |
| **Platform** | Web dashboard (axum), scheduler, webhooks, Docker, profiles, plugins |
| **Notifications** | Real HTTP to Slack, Discord, Telegram — testable with `contribai notify-test` |

## Usage (22 Commands)

```bash
# Hunt & contribute
contribai hunt                        # Autonomous discovery + PRs
contribai hunt --dry-run              # Analyze only, no PRs
contribai run                         # Single pipeline run
contribai target <url>                # Target specific repo
contribai analyze <url>               # Dry-run analysis
contribai solve <url>                 # Solve open issues

# Monitor
contribai patrol                      # Respond to PR reviews
contribai status                      # PR status table
contribai stats                       # Contribution statistics
contribai leaderboard                 # Merge rate by repo
contribai system-status               # DB, rate limits, scheduler

# Interactive
contribai                             # Interactive menu (22 items)
contribai interactive                 # ratatui TUI browser
contribai init                        # Setup wizard
contribai login                       # Interactive auth + provider config

# Config
contribai config-list
contribai config-get llm.provider
contribai config-set llm.provider openai
contribai profile security-focused    # Named profile

# Servers
contribai web-server                  # Dashboard at :8787
contribai schedule                    # Cron scheduler
contribai mcp-server                  # MCP stdio server
contribai cleanup                     # Remove stale forks
contribai notify-test                 # Test Slack/Discord/Telegram
```

## Configuration

```yaml
# config.yaml
github:
  token: "ghp_your_token"       # or set GITHUB_TOKEN env var

llm:
  provider: "gemini"            # gemini | openai | anthropic | ollama
  model: "gemini-2.5-flash"
  api_key: "your_api_key"

discovery:
  languages: [python, javascript, rust, go]
  stars_range: [100, 5000]
```

See [`config.yaml.template`](config.yaml.template) for all options.

## Architecture

```
ContribAI/
├── crates/contribai-rs/src/   ← PRIMARY: Rust v5.2.0
│   ├── cli/                   # 22 commands + ratatui TUI
│   ├── core/                  # Config, events, middleware
│   ├── github/                # REST + GraphQL client
│   ├── analysis/              # 17 progressive skills
│   ├── generator/             # LLM fix generation + scoring
│   ├── orchestrator/          # Pipeline + SQLite memory (72h TTL)
│   ├── pr/                    # PR lifecycle + patrol
│   ├── llm/                   # Multi-provider LLM + 5 sub-agents
│   ├── mcp/                   # 21-tool MCP server (stdio)
│   ├── web/                   # axum dashboard + webhooks
│   ├── sandbox/               # Docker + local fallback
│   └── scheduler/             # Tokio cron
│
└── python/                    # Legacy v4.1.0 (reference only)
```

See [`docs/system-architecture.md`](docs/system-architecture.md) for details.

## Testing

```bash
# Rust (primary)
cargo test                  # 335 tests
cargo test -- --nocapture   # with stdout

# Python legacy
cd python && pytest tests/ -v
```

## MCP — Use from Claude Desktop / Antigravity IDE

```json
// ~/.config/claude/claude_desktop_config.json
// or ~/.gemini/antigravity/mcp_config.json
{
  "mcpServers": {
    "contribai": {
      "command": "contribai",
      "args": ["mcp-server"]
    }
  }
}
```

21 tools available: repo analysis, PR management, GitHub search, issue solving, memory queries.

## Docker

```bash
docker compose up -d dashboard            # Dashboard at :8787
docker compose run --rm runner run        # One-shot run
docker compose up -d dashboard scheduler  # Dashboard + scheduler
```

## Documentation

| Doc | Description |
|-----|-------------|
| [`HALL_OF_FAME.md`](HALL_OF_FAME.md) | **9 merged** · **14 closed** across 21+ repos — real results |
| [`AGENTS.md`](AGENTS.md) | AI agent guide — architecture, patterns, CLI reference |
| [`deployment-guide.md`](docs/deployment-guide.md) | Install, Docker, config, all 22 CLI commands |
| [`system-architecture.md`](docs/system-architecture.md) | Pipeline, middleware, events, LLM routing |
| [`codebase-summary.md`](docs/codebase-summary.md) | Module map, tech stack, data structures |
| [`project-roadmap.md`](docs/project-roadmap.md) | Version history and future plans |
| [`python/README_PYTHON.md`](python/README_PYTHON.md) | Legacy Python v4.1.0 reference |

## License

AGPL-3.0 + Commons Clause — see [LICENSE](LICENSE) for details.
