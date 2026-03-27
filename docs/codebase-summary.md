# ContribAI Codebase Summary

**Version:** 3.0.2 | **Total LOC:** ~5,500+ | **Modules:** 14 | **Test Files:** 25+

---

## Quick Navigation

```
contribai/
├── core/              # Foundational abstractions (1,100 LOC)
├── llm/               # Multi-provider LLM routing (900 LOC)
├── github/            # GitHub API + discovery (550 LOC)
├── analysis/          # 7 analyzers + 17 skills (700 LOC)
├── generator/         # Code fix generation (300 LOC)
├── orchestrator/      # Pipeline + hunt + memory (500 LOC)
├── pr/                # PR lifecycle management (542 LOC)
├── issues/            # Issue-driven contributions (339 LOC)
├── agents/            # Sub-agent registry (158 LOC)
├── tools/             # Tool protocol (59 LOC)
├── mcp_server.py      # MCP stdio server (180 LOC)
├── web/               # FastAPI dashboard (324 LOC)
├── cli/               # Click CLI + TUI (150+ LOC)
├── scheduler/         # APScheduler wrapper (100 LOC)
├── plugins/           # Plugin system (161 LOC)
├── templates/         # Contribution templates (96 LOC)
├── notifications/     # Slack/Discord/Telegram (248 LOC)
├── sandbox/           # Docker code validation (244 LOC)
└── mcp/               # MCP client (180 LOC)
```

---

## Module Responsibilities

| Module | Purpose | Key Classes/Functions | LOC |
|--------|---------|----------------------|-----|
| **core** | Config, models, middleware, events, exceptions, utilities | `Config`, `Middleware`, `EventBus`, `Repository`, `Finding`, `Contribution` | 1,100 |
| **llm** | Multi-provider routing, token budgeting, formatting | `LLMProvider`, `TaskRouter`, `ContextManager`, `Formatter` | 900 |
| **github** | Async GitHub client, repo discovery, guidelines parsing | `GitHubClient`, `RepoDiscovery`, `GuidelineParser` | 550 |
| **analysis** | Multi-strategy code analysis, skill loading | `CodeAnalyzer`, `SkillLoader`, `SecurityStrategy`, `CodeQualityStrategy` | 700 |
| **generator** | LLM-powered fix generation, self-review, quality scoring | `ContributionGenerator`, `QualityScorer` | 300 |
| **orchestrator** | Pipeline coordination, hunt mode, outcome memory | `Pipeline`, `HuntMode`, `Memory` | 500 |
| **pr** | PR creation, patrol monitoring, CLA/DCO handling | `PRManager`, `PRPatrol`, `CLAHandler` | 542 |
| **issues** | Issue discovery and solving | `IssueSolver` | 339 |
| **agents** | Sub-agent registry with parallel execution | `SubAgentRegistry`, `AnalyzerAgent`, `GeneratorAgent` | 158 |
| **tools** | Tool protocol (MCP-inspired) | `Tool`, `ToolResult`, `GitHubTool`, `LLMTool` | 59 |
| **mcp_server** | MCP stdio server (14 tools for Claude) | `MCPServer` | 180 |
| **web** | FastAPI REST API, webhooks, dashboard | `app`, `api_routes`, `webhook_handler` | 324 |
| **cli** | Click-based CLI, Rich TUI | `main`, `tui` | 150+ |
| **scheduler** | APScheduler wrapper for cron automation | `Scheduler` | 100 |
| **plugins** | Entry-point plugin system | `AnalyzerPlugin`, `GeneratorPlugin` | 161 |
| **templates** | YAML-based contribution templates | `TemplateRegistry` | 96 |
| **notifications** | Slack/Discord/Telegram integrations | `Notifier` | 248 |
| **sandbox** | Docker-based code validation | `Sandbox` | 244 |
| **mcp** | MCP JSON-RPC client | `MCPClient` | 180 |

---

## Dependency Graph

```
                     ┌──────────────────┐
                     │   CLI / Web      │
                     └────────┬─────────┘
                              │
                   ┌──────────┴──────────┐
                   ▼                     ▼
            ┌─────────────┐      ┌──────────────┐
            │ Orchestrator│      │  Scheduler   │
            │  + Pipeline │      │              │
            └─────┬───────┘      └──────────────┘
                  │
        ┌─────────┼─────────┬──────────┐
        ▼         ▼         ▼          ▼
    ┌────────┐┌────────┐┌────────┐┌────────┐
    │Analysis││Generator││  PR    ││ Issues │
    │        ││         ││Manager ││ Solver │
    └────┬───┘└────┬───┘└────┬───┘└────┬───┘
         │         │         │        │
         └─────────┼─────────┴────────┘
                   │
        ┌──────────┴──────────┐
        ▼                     ▼
    ┌─────────┐         ┌──────────┐
    │   LLM   │         │  GitHub  │
    │ Routing │         │  Client  │
    └────┬────┘         └────┬─────┘
         │                   │
         └───────┬───────────┘
                 ▼
         ┌──────────────┐
         │    CORE      │
         │ (Models,     │
         │  Config,     │
         │  Middleware) │
         └──────────────┘
```

**Dependency Flow:** core ← github/llm ← analysis/generator ← orchestrator ← cli/web

---

## Key Entry Points

### CLI Entry Point
- **File:** `contribai/cli/main.py`
- **Function:** `cli()` (Click group)
- **Main Commands:**
  - `hunt` — Autonomous multi-round hunting
  - `run` — Single full pipeline run
  - `target` — Analyze specific repo
  - `solve` — Solve issues in a repo
  - `serve` — Start web dashboard
  - `schedule` — Start scheduler

### Web Entry Point
- **File:** `contribai/web/server.py`
- **Class:** `app` (FastAPI application)
- **Key Routes:**
  - `GET /api/stats` — Overall statistics
  - `GET /api/repos` — Analyzed repos list
  - `POST /api/run` — Trigger pipeline
  - `GET /dashboard` — Web UI
  - `POST /webhooks/github` — GitHub webhook receiver

### Orchestrator Entry Point
- **File:** `contribai/orchestrator/pipeline.py`
- **Class:** `Pipeline`
- **Key Methods:**
  - `run()` — Execute pipeline on single repo
  - `hunt()` — Execute hunt mode
  - `process_repo()` — Core analysis → generation → PR flow

### MCP Server Entry Point
- **File:** `contribai/mcp_server.py`
- **Class:** `MCPServer`
- **Protocol:** stdio JSON-RPC
- **Exposed Tools:** 14 (GitHub read/write, safety, maintenance)

---

## Critical Data Structures

### Core Models (Pydantic)

| Class | Purpose | Key Fields |
|-------|---------|-----------|
| `Repository` | GitHub repo metadata | `owner`, `name`, `url`, `stars`, `language`, `last_commit` |
| `Finding` | Detected issue | `type`, `file`, `line`, `description`, `severity`, `context` |
| `Contribution` | Proposed fix | `finding_id`, `code_change`, `explanation`, `confidence_score` |
| `PRResult` | PR outcome | `pr_number`, `url`, `status`, `feedback`, `time_to_merge` |
| `Config` | Application config | `github`, `llm`, `discovery`, `analysis`, `pipeline` |

### Database Schema (SQLite)

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| `analyzed_repos` | Track analyzed repos | repo_id, timestamp, status |
| `submitted_prs` | All created PRs | repo_id, pr_number, url, status |
| `findings_cache` | Cached analysis results | repo_id, findings_json, timestamp |
| `run_log` | Pipeline execution history | timestamp, status, repo_count, pr_count |
| `pr_outcomes` | PR merge/close outcomes | pr_number, outcome, feedback, time_to_close |
| `repo_preferences` | Learned repo patterns | repo_id, preferred_types, rejected_types |

### Event Types (15 total)

```python
RepositoryDiscovered | RepositoryAnalyzed | FindingDetected | ContributionGenerated |
PRCreated | PRMerged | PRClosed | PRPatrolStarted | ReviewFound | CodeChangeGenerated |
ConfigLoaded | PipelineStarted | PipelineCompleted | ErrorOccurred | RateLimitExceeded
```

---

## Technology Stack

| Category | Technologies |
|----------|--------------|
| **Language** | Python 3.11+ |
| **Web** | FastAPI, Uvicorn, Jinja2 |
| **GitHub** | GitPython, httpx (async) |
| **LLM** | google-genai, openai, anthropic |
| **Data** | Pydantic, aiosqlite, SQLite |
| **CLI** | Click, Rich |
| **Scheduling** | APScheduler |
| **Task Runtime** | asyncio |
| **Code Validation** | Docker (optional), ast.parse (fallback) |
| **Testing** | pytest, pytest-asyncio, pytest-cov |
| **Linting** | ruff |
| **Type Checking** | pyright (implicit) |

---

## File Organization Principles

### Module Structure

Each module follows this pattern:

```
module/
├── __init__.py          # Public API exports
├── main_class.py        # Primary class (e.g., analyzer.py)
├── sub_component.py     # Supporting components
└── exceptions.py        # Module-specific exceptions (optional)
```

### Configuration

- **Source:** `contribai/core/config.py` (Pydantic model)
- **File:** `config.yaml` (YAML with schema validation)
- **Overrides:** Environment variables (prefix: `CONTRIBAI_`)
- **Presets:** Named profiles (YAML files in `contribai/core/profiles.py`)

### Database

- **Location:** `~/.contribai/memory.db` (auto-initialized)
- **Type:** SQLite 3.x
- **Async:** aiosqlite
- **Migrations:** Embedded in `Memory.init()` method

### Events

- **Emit Location:** Throughout codebase
- **Handling:** `core/events.py` (EventBus)
- **Logging:** `~/.contribai/events.jsonl` (append-only)
- **Consumption:** Notifications, webhooks, monitoring

---

## Async/Concurrency Model

### Async First Design

- **All I/O:** async (GitHub API, LLM API, file operations)
- **Concurrency Control:** `asyncio.gather()` with `Semaphore` (max 3 concurrent repos)
- **Database Access:** aiosqlite (async SQLite)
- **Rate Limiting:** Middleware intercepts, enforces delays

### Key Async Patterns

```python
# Pipeline: async repo processing
for repo in repos:
    task = pipeline.process_repo(repo)
    tasks.append(task)
await asyncio.gather(*tasks, return_exceptions=True)

# LLM: async with retry
async with retry_handler(max_retries=3, backoff=exponential):
    response = await llm_provider.complete(prompt)

# Database: async transaction
async with memory.transaction():
    await memory.add_finding(finding)
    await memory.increment_stats()
```

---

## Configuration Hierarchy

1. **Defaults** — Hardcoded in `Config` class
2. **File** — `config.yaml` (overrides defaults)
3. **Profiles** — Named presets (override file)
4. **Environment** — Env vars `CONTRIBAI_*` (override profiles)
5. **CLI Flags** — Command-line args (override all)

**Load Order:** CLI flags → Env vars → Profile → YAML → Defaults

---

## Testing Structure

```
tests/
├── unit/                   # Isolated module tests
│   ├── test_analyzer.py
│   ├── test_generator.py
│   ├── test_github_client.py
│   ├── test_llm_provider.py
│   ├── test_memory.py
│   ├── test_middleware.py
│   ├── test_pr_manager.py
│   └── ... (25+ test files)
├── integration/            # End-to-end pipeline tests
│   └── test_pipeline.py
└── conftest.py             # Shared fixtures
```

**Test Coverage:** > 85% (enforced in CI)

**Test Tools:**
- `pytest` — Test framework
- `pytest-asyncio` — Async test support
- `pytest-cov` — Coverage reporting
- `unittest.mock` — Mocking

---

## Common Code Patterns

### Pattern 1: Async Context Manager for Resources

```python
async with GitHubClient(token) as client:
    repos = await client.search_repos(language="python")
```

### Pattern 2: Pydantic Model + Validation

```python
class Config(BaseModel):
    github: GitHubConfig
    llm: LLMConfig
    discovery: DiscoveryConfig

    model_config = ConfigDict(validate_assignment=True)
```

### Pattern 3: Middleware Chain

```python
@middleware("rate_limit")
@middleware("validation")
@middleware("retry")
async def process_repo(repo: Repository) -> PipelineResult:
    ...
```

### Pattern 4: Provider Factory + Strategy

```python
def create_llm_provider(config: LLMConfig) -> LLMProvider:
    if config.provider == "gemini":
        return GeminiProvider(config)
    elif config.provider == "openai":
        return OpenAIProvider(config)
```

### Pattern 5: Event Bus + Typed Events

```python
event_bus.emit(RepositoryAnalyzed(
    repo=repo,
    findings_count=len(findings),
    timestamp=datetime.now()
))
```

---

## Import Conventions

### Relative Imports (Within Modules)
```python
from .analyzer import CodeAnalyzer
from .skills import load_skills
```

### Absolute Imports (Cross-Module)
```python
from contribai.core.models import Repository, Finding
from contribai.llm.provider import create_llm_provider
from contribai.analysis.analyzer import CodeAnalyzer
```

### Avoid Circular Imports
Use TYPE_CHECKING for type hints:
```python
from typing import TYPE_CHECKING
if TYPE_CHECKING:
    from contribai.core.models import Repository
```

---

## Performance Considerations

| Component | Optimization | Details |
|-----------|--------------|---------|
| **LLM Context** | Token budgeting | Max 30k tokens per analysis |
| **GitHub API** | Rate limit respect | Check limits before burst requests |
| **Database** | Batch inserts | Insert 100+ findings in one transaction |
| **Analysis** | Progressive skills | Load only needed skills (by language/framework) |
| **Concurrency** | Semaphore(3) | Max 3 repos processed simultaneously |
| **Caching** | 72h TTL | Cache analysis results per repo |

---

## Security Considerations

### Secrets Management
- **Never log** API keys, tokens, or credentials
- **Use env vars** for sensitive config
- **Validate LLM output** before code execution
- **Sandbox execution** (Docker or ast.parse)

### External Code Safety
- **Syntax validation** before commit
- **Balanced bracket check** before parsing
- **ast.parse fallback** if Docker unavailable
- **Manual review gate** (optional) for high-risk changes

### Access Control
- **GitHub token** required (PAT scope: `repo`, `workflow`)
- **API key auth** on dashboard endpoints
- **HMAC validation** on webhooks
- **No direct shell exec** (use GitPython, httpx)

---

## Document Metadata

- **Created:** 2026-03-28
- **Last Updated:** 2026-03-28
- **Owner:** Technical Writer / Documentation Team
- **References:** README.md, docs/ARCHITECTURE.md, docs/system-architecture.md
