# ContribAI

> **AI Agent that automatically contributes to open source projects on GitHub**

ContribAI discovers open source repositories, analyzes them for improvement opportunities, generates high-quality fixes, and submits them as Pull Requests — all autonomously.

[![Python 3.11+](https://img.shields.io/badge/python-3.11+-blue.svg)](https://www.python.org/downloads/)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-400%2B%20passed-brightgreen)](#)
[![Version](https://img.shields.io/badge/version-3.0.0-blue)](#)

---

## Features

### Core Pipeline
- **Smart Discovery** – Finds contribution-friendly repos by language, stars, activity
- **Security Analysis** – Detects hardcoded secrets, SQL injection, XSS
- **Code Quality** – Finds dead code, missing error handling, complexity issues
- **Performance** – String allocation, blocking calls, N+1 queries
- **Documentation** – Catches missing docstrings, incomplete READMEs
- **UI/UX** – Identifies accessibility issues, responsive design gaps
- **Refactoring** – Unused imports, non-null assertions, encoding issues
- **Multi-LLM** – Gemini (primary), OpenAI, Anthropic, Ollama, Vertex AI
- **Auto-PR** – Forks, branches, commits, and creates PRs automatically

### Hunt Mode (v0.11.0+)
- **Autonomous hunting** – Discovers repos across GitHub and creates PRs at scale
- **Sequential processing** – Configurable inter-repo delay to avoid API rate limits (v2.6.0)
- **Code validation** – Pre-self-review syntax checks (empty edits, no-ops, balanced brackets)
- **Multi-round** – Runs N rounds with configurable delay between rounds
- **Cross-file fixes** – Detects the same pattern across multiple files and fixes all at once
- **Duplicate prevention** – Title similarity matching prevents duplicate PRs
- **Post-PR CI monitoring** – Auto-closes PRs that fail CI checks

### Resilience & Safety (v2.0.0)
- **AI policy detection** – Skips repos that ban AI-generated contributions
- **CLA auto-signing** – Detects CLAAssistant/EasyCLA and auto-signs
- **Smart validation** – Deep finding validation reduces false positives
- **Rate limiting** – Max 2 findings per repo to avoid spamming
- **API retry with backoff** – Auto-retries on 502/503/504 errors (3 attempts, exponential backoff)
- **Code-only modifications** – Skips `.md`, `.yaml`, `.json`, `.toml` and meta files (LICENSE, CONTRIBUTING.md)
- **Fork cleanup** – `contribai cleanup` removes stale forks with no open PRs
- **Clean PR format** – Professional PR body, no unnecessary boilerplate

### PR Patrol (v2.2.0+)
- **Review monitoring** – Scans open PRs for maintainer feedback and auto-responds
- **Bot context awareness** – Reads bot review analysis (Coderabbit, etc.) when maintainers reference them
- **Smart classification** – LLM classifies feedback as CODE_CHANGE, QUESTION, STYLE_FIX, APPROVE, REJECT
- **Auto code fix** – Generates and pushes fixes via GitHub API based on review feedback
- **Rate limit retry** – Exponential backoff (5s/10s/20s) for rate limited API calls
- **Assigned issue detection** – Scans repos for issues assigned to the user
- **DCO auto-signoff** – Automatically appends `Signed-off-by` to all commits
- **Bot filtering** – Filters 11+ known review bots to avoid false feedback classification

### MCP Server (v2.6.0)
- **14 MCP tools** – Expose ContribAI to Claude Desktop via stdio protocol
- **GitHub Read** – search_repos, get_repo_info, get_file_tree, get_file_content, get_open_issues
- **GitHub Write** – fork_repo, create_branch, push_file_change, create_pr, close_pr
- **Safety** – check_duplicate_pr, check_ai_policy
- **Maintenance** – patrol_prs, cleanup_forks, get_stats
- **Resource safe** – Proper cleanup on shutdown, fork delete guard

### Agent Architecture (v2.7.0-v2.8.0)
- **Context Compression** – LLM-driven structured summarization + truncation-based compression (30k token budget)
- **Working Memory** – Auto-load/save per-repo analysis context with 72h TTL
- **Event Bus** – 15 typed events with async subscribers and JSONL file logging
- **Sandbox Execution** – Docker-based code validation with local `ast.parse` fallback
- **Inspired by** – AgentScope, DeerFlow, SWE-agent, OpenHands

### Multi-Model Agent (v0.7.0+)
- **Task routing** – Routes analysis/generation/review to different models
- **Model tiers** – Fast models for triage, powerful for generation
- **Vertex AI** – Google Cloud Vertex AI provider support
- **Env var fallback** – Token/API key from environment variables

### Platform (v0.4.0-v0.5.0)
- **Web Dashboard** – FastAPI REST API + static dashboard at `:8787`
- **Scheduler** – APScheduler cron-based automated runs
- **Parallel Processing** – `asyncio.gather` + Semaphore (3 concurrent repos)
- **Templates** – 5 built-in contribution templates
- **Profiles** – Named presets: `security-focused`, `docs-focused`, `full-scan`, `gentle`
- **Plugin System** – Entry-point based plugins for custom analyzers/generators
- **Webhooks** – GitHub webhook receiver for auto-triggering on issues/push
- **Usage Quotas** – Track GitHub + LLM API calls with daily limits
- **API Auth** – API key authentication for dashboard mutation endpoints
- **Docker** – Dockerfile + docker-compose (dashboard, scheduler, runner)

## Architecture

```
                     Middleware Chain
 Discovery → [RateLimit → Validation → Retry → DCO → QualityGate]
     │                                  │
     ▼                                  ▼
  GitHub         ┌──────────Sub-Agent Registry──────────┐
  Search         │  Analyzer │ Generator │ Patrol │ Compliance │ MCP │
  + Hunt         └────┬──────────┬──────────┬────────┬──┘
  + Webhooks          │          │          │        │
                 ┌────▼────┐ ┌───▼───┐ ┌───▼───┐ ┌─▼──┐
                 │ Skills  │ │  LLM  │ │GitHub │ │DCO │
                 │(17 on-  │ │+ Tool │ │+ Tool │ │Sign│
                 │ demand) │ │Protocol│ │       │ │off │
                 └────┬────┘ └───┬───┘ └───┬───┘ └─┬──┘
                      └──────────┴─────────┴───────┘
                                  │
                          Outcome Memory (SQLite)
                          6 tables + learning
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed architecture documentation.

## Installation

```bash
git clone https://github.com/tang-vu/ContribAI.git
cd ContribAI
pip install -e ".[dev]"
```

### Docker

```bash
docker compose up -d dashboard          # Dashboard at :8787
docker compose run --rm runner run      # One-shot run
docker compose up -d dashboard scheduler  # Dashboard + scheduler
```

## Configuration

```bash
cp config.example.yaml config.yaml
```

Edit `config.yaml`:

```yaml
github:
  token: "ghp_your_token_here"

llm:
  provider: "gemini"
  model: "gemini-2.5-flash"
  api_key: "your_api_key"

discovery:
  languages: [python, javascript]
  stars_range: [100, 5000]
```

## Usage

### Hunt mode (autonomous mass contribution)

```bash
contribai hunt                             # Hunt for repos and contribute
contribai hunt --rounds 5 --delay 15       # 5 rounds, 15min delay
contribai hunt --mode analysis             # Code analysis only (no issues)
contribai hunt --mode issues               # Issue solving only
contribai hunt --mode both                 # Both analysis + issues (default)
```

### Target a specific repo

```bash
contribai target https://github.com/owner/repo
contribai target https://github.com/owner/repo --dry-run
```

### Auto-discover and contribute

```bash
contribai run                              # Full autonomous run
contribai run --dry-run                    # Preview without creating PRs
contribai run --language python            # Filter by language
```

### Solve open issues

```bash
contribai solve https://github.com/owner/repo
```

### Web Dashboard & Scheduler

```bash
contribai serve                            # Dashboard at :8787
contribai serve --port 9000                # Custom port
contribai schedule --cron "0 */6 * * *"    # Auto-run every 6h
```

### Templates & Profiles

```bash
contribai templates                        # List contribution templates
contribai profile list                     # List profiles
contribai profile security-focused         # Run with profile
```

### Status, stats & cleanup

```bash
contribai status        # Check submitted PRs
contribai stats         # Overall statistics
contribai info          # System info
contribai cleanup       # Remove stale forks with no open PRs
```

## Plugin System

Create custom analyzers as Python packages:

```python
from contribai.plugins.base import AnalyzerPlugin

class MyAnalyzer(AnalyzerPlugin):
    @property
    def name(self): return "my-analyzer"

    async def analyze(self, context):
        return findings
```

Register via entry points in `pyproject.toml`:

```toml
[project.entry-points."contribai.analyzers"]
my_analyzer = "my_package:MyAnalyzer"
```

## Project Structure

```
contribai/
├── core/              # Config, models, middleware chain
├── llm/               # Multi-provider LLM (Gemini, OpenAI, Anthropic, Ollama, Vertex)
├── github/            # GitHub API client, repo discovery, guidelines
├── analysis/          # 7 analyzers + progressive skill loading (17 skills)
├── agents/            # Sub-agent registry (Analyzer, Generator, Patrol, Compliance)
├── tools/             # MCP-inspired tool protocol (GitHubTool, LLMTool)
├── mcp_server.py      # MCP stdio server (14 tools for Claude Desktop)
├── generator/         # Contribution generator + self-review + quality scorer
├── issues/            # Issue-driven contribution solver
├── pr/                # PR lifecycle manager + patrol + CLA handler
├── orchestrator/      # Pipeline orchestrator, hunt mode, outcome memory
├── notifications/     # Slack, Discord, Telegram notifications
├── plugins/           # Plugin system (analyzer/generator extensions)
├── templates/         # Contribution templates (5 built-in YAML)
├── scheduler/         # APScheduler cron-based automation
├── web/               # FastAPI dashboard, auth, webhooks
└── cli/               # Rich CLI + interactive TUI

docs/
└── ARCHITECTURE.md    # Detailed architecture documentation

AGENTS.md              # AI agent guide (for Copilot, Claude, Coderabbit, etc.)
```

## Testing

```bash
pytest tests/ -v                  # Run all 370+ tests
pytest tests/ -v --cov=contribai  # With coverage
ruff check contribai/             # Lint
ruff format contribai/            # Format
```

## Safety

- **Daily PR limit** – Configurable max PRs per day (default: 15)
- **Quality scorer** – 7-check gate prevents low-quality PRs
- **Deep validation** – LLM validates findings against full file context
- **AI policy detection** – Skips repos that ban AI contributions
- **Duplicate prevention** – Title similarity matching prevents spam
- **CI monitoring** – Auto-closes PRs that fail CI checks
- **API quotas** – Track and limit GitHub + LLM usage daily
- **Dry run mode** – Preview everything without creating PRs
- **5xx retry with backoff** – Auto-retries on GitHub 502/503/504 (3x, 2s/4s/8s)
- **Code-only modifications** – Never modifies docs, configs, or meta files
- **Fork cleanup** – Removes stale forks after PRs are merged/closed

## License

AGPL-3.0 + Commons Clause – see [LICENSE](LICENSE) for details.

---

**Made with ❤️ for the open source community**
