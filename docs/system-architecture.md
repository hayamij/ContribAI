# System Architecture

**Version:** 3.0.2 | **Last Updated:** 2026-03-28

---

## High-Level Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                    ContribAI Pipeline (v3.0.2)                  │
└─────────────────────────────────────────────────────────────────┘

Input: GitHub Repository (URL or discovery)
   ▼
┌─────────────────────────────────────────────────────────────────┐
│ 1. DISCOVERY                                                    │
│ ├─ GitHub Search API (language, stars, activity)               │
│ ├─ Hunt Mode: Multi-round discovery with delays                │
│ ├─ Issue-driven: Fetch open issues from repo                   │
│ └─ Duplicate check: Skip if already analyzed                   │
└────────────────────────┬────────────────────────────────────────┘
   ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. MIDDLEWARE CHAIN (Pre-processing)                            │
│ ├─ RateLimitMiddleware: Check daily PR limit + API rate        │
│ ├─ ValidationMiddleware: Validate repo data exists             │
│ ├─ RetryMiddleware: 2 retries with exponential backoff         │
│ ├─ DCOMiddleware: Compute Signed-off-by signature              │
│ └─ QualityGateMiddleware: Score check (min 0.6/1.0)            │
└────────────────────────┬────────────────────────────────────────┘
   ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. ANALYSIS                                                     │
│ ├─ Language/Framework detection                                │
│ ├─ Progressive skill loading (17 skills, on-demand)            │
│ ├─ 7 Multi-strategy analyzers (parallel):                      │
│ │  ├─ SecurityStrategy (hardcoded secrets, SQL injection, XSS) │
│ │  ├─ CodeQualityStrategy (dead code, error handling)          │
│ │  ├─ PerformanceStrategy (N+1 queries, blocking calls)        │
│ │  ├─ DocumentationStrategy (missing docstrings, READMEs)      │
│ │  ├─ UIUXStrategy (accessibility, responsive design)          │
│ │  ├─ RefactoringStrategy (unused imports, non-null checks)    │
│ │  └─ FrameworkStrategy (Django/Flask/FastAPI/React patterns)  │
│ ├─ Deep validation: LLM validates findings against file context│
│ └─ Result: List of Findings with severity + description        │
└────────────────────────┬────────────────────────────────────────┘
   ▼
┌─────────────────────────────────────────────────────────────────┐
│ 4. GENERATION                                                   │
│ ├─ For each finding:                                           │
│ │  ├─ LLM generates code fix (with retry on failure)           │
│ │  ├─ Self-review: LLM validates own fix                       │
│ │  ├─ Quality scoring: 7-check gate (correctness, style, etc.) │
│ │  ├─ Syntax validation (balanced brackets, no-op detection)   │
│ │  └─ Result: Contribution with confidence score               │
│ ├─ Cross-file detection: Find same pattern across files        │
│ └─ Filter: Keep only score >= 0.6                              │
└────────────────────────┬────────────────────────────────────────┘
   ▼
┌─────────────────────────────────────────────────────────────────┐
│ 5. PR CREATION (Unless dry-run)                                │
│ ├─ Fork repository (or use existing fork)                      │
│ ├─ Create feature branch (naming: contribai/finding-type-repo) │
│ ├─ Commit changes with DCO signoff                             │
│ ├─ Create PR with detailed description                         │
│ ├─ Auto-sign CLA if required (CLA-Assistant, EasyCLA)          │
│ ├─ Record PR in memory (submitted_prs table)                   │
│ └─ Result: PR URL + number                                     │
└────────────────────────┬────────────────────────────────────────┘
   ▼
┌─────────────────────────────────────────────────────────────────┐
│ 6. POST-PROCESSING                                              │
│ ├─ Event emission (PRCreated, PipelineCompleted)               │
│ ├─ Notification dispatch (Slack, Discord, Telegram)            │
│ ├─ Memory update (record outcomes)                             │
│ ├─ PR Patrol monitoring (async, background)                    │
│ └─ CI status tracking (auto-close on failure)                  │
└────────────────────────┬────────────────────────────────────────┘
   ▼
Output: PipelineResult (repos_analyzed, prs_created, findings_count)
```

---

## Middleware Chain

Middlewares wrap the core processing loop, applying cross-cutting concerns in order:

### Chain Order & Behavior

| Order | Middleware | Purpose | Example Decision |
|-------|-----------|---------|------------------|
| 1 | `RateLimitMiddleware` | Check daily limits + API rate | Skip if PR count >= 15/day |
| 2 | `ValidationMiddleware` | Validate repo structure exists | Skip if no src dir found |
| 3 | `RetryMiddleware` | Auto-retry on transient failure | Retry on 502/503/504 (2x) |
| 4 | `DCOMiddleware` | Compute Signed-off-by | Add to every commit |
| 5 | `QualityGateMiddleware` | Min quality score threshold | Skip if avg score < 0.6 |

**Pattern:** Each middleware calls `await next_handler()` to continue chain.

```python
async def process_repo(repo: Repository) -> PipelineResult:
    """Execution flow through middleware chain."""
    # Step 1: RateLimitMiddleware checks limits
    if self.daily_pr_count >= 15:
        raise RateLimitError("Daily PR limit reached")

    # Step 2: ValidationMiddleware validates repo
    if not await self.github_client.repo_exists(repo):
        raise ValidationError("Repo not accessible")

    # Step 3: RetryMiddleware wraps analysis/generation
    for attempt in range(3):
        try:
            findings = await analyzer.analyze(repo)
            break
        except Exception as e:
            if attempt == 2:
                raise
            await asyncio.sleep(2 ** attempt)  # Exponential backoff

    # Step 4: DCOMiddleware adds signoff
    commit_message = f"...\n\nSigned-off-by: {self.dco_signature()}"

    # Step 5: QualityGateMiddleware scores contributions
    contributions = await generator.generate_fixes(findings)
    high_quality = [c for c in contributions if c.score >= 0.6]

    # Create PRs for high-quality contributions
    prs = await pr_manager.create_prs(repo, high_quality)
    return PipelineResult(repo=repo, prs=prs)
```

---

## Sub-Agent Registry

ContribAI uses a DeerFlow/AgentScope-inspired agent architecture with 4 specialized agents:

### Agent Overview

| Agent | Role | Wraps | Max Concurrent |
|-------|------|-------|----------------|
| `AnalyzerAgent` | Code analysis | `CodeAnalyzer` | 3 |
| `GeneratorAgent` | Fix generation | `ContributionGenerator` | 3 |
| `PatrolAgent` | PR monitoring | `PRPatrol` | 1 |
| `ComplianceAgent` | CLA/DCO/CI | `PRManager` | 3 |

### Parallel Execution Model

```python
# Launch agents in parallel for independent tasks
tasks = [
    analyzer_agent.analyze(repo),
    generator_agent.generate(findings),
    patrol_agent.monitor_prs(),
]

results = await asyncio.gather(*tasks, return_exceptions=True)

# Limit concurrency to 3 repos at a time
semaphore = asyncio.Semaphore(3)

async def process_repo_with_limit(repo):
    async with semaphore:
        return await pipeline.process_repo(repo)

# Map over all repos with concurrency control
tasks = [process_repo_with_limit(repo) for repo in repos]
await asyncio.gather(*tasks, return_exceptions=True)
```

---

## Event Bus System

Typed event system with async subscribers and JSONL logging.

### 15 Built-in Events

```python
# Discovery events
RepositoryDiscovered(repo: Repository, timestamp: datetime)

# Analysis events
RepositoryAnalyzed(repo: Repository, findings_count: int, timestamp: datetime)
FindingDetected(repo: Repository, finding: Finding, timestamp: datetime)

# Generation events
ContributionGenerated(finding: Finding, contribution: Contribution, timestamp: datetime)
CodeChangeGenerated(repo: Repository, file: str, change: str, timestamp: datetime)

# PR events
PRCreated(repo: Repository, pr_number: int, url: str, timestamp: datetime)
PRMerged(repo: Repository, pr_number: int, time_to_merge_hours: float, timestamp: datetime)
PRClosed(repo: Repository, pr_number: int, reason: str, timestamp: datetime)

# Patrol events
PRPatrolStarted(repo: Repository, open_pr_count: int, timestamp: datetime)
ReviewFound(repo: Repository, pr_number: int, review: str, timestamp: datetime)

# System events
ConfigLoaded(config_file: str, timestamp: datetime)
PipelineStarted(mode: str, repo_count: int, timestamp: datetime)
PipelineCompleted(status: str, repos_processed: int, prs_created: int, timestamp: datetime)
ErrorOccurred(error: str, module: str, timestamp: datetime)
RateLimitExceeded(service: str, reset_time: int, timestamp: datetime)
```

### Event Subscription Example

```python
from contribai.core.events import EventBus, PRCreated

event_bus = EventBus()

# Subscribe to event
@event_bus.on(PRCreated)
async def send_slack_notification(event: PRCreated):
    await notifier.send(f"PR created: {event.url}")

# Emit event
event_bus.emit(PRCreated(
    repo=repo,
    pr_number=123,
    url="https://github.com/...",
    timestamp=datetime.now()
))

# Log all events to JSONL
event_bus.enable_jsonl_logging("~/.contribai/events.jsonl")
```

---

## LLM Routing & Multi-Model Support

### Provider Architecture

```
┌─────────────────┐
│  LLM Config     │
│ (provider, key, │
│  model, temp)   │
└────────┬────────┘
         ▼
┌─────────────────────────────┐
│ LLM Provider Factory        │
└────────┬────────────────────┘
         │
    ┌────┴────┬────────┬──────────┬──────────┐
    ▼         ▼        ▼          ▼          ▼
┌────────┐┌────────┐┌────────┐┌────────┐┌────────┐
│Gemini  ││OpenAI  ││Anthropic│Ollama   │ Vertex  │
│Provider││Provider││Provider │Provider │  AI     │
└────────┘└────────┘└────────┘└────────┘└────────┘
    │         │        │          │          │
    └─────────┴────────┴──────────┴──────────┘
              │
         ┌────▼────────┐
         │ TaskRouter  │
         │ (Route by   │
         │  task type) │
         └─────┬───────┘
               │
   ┌───────────┼───────────┐
   ▼           ▼           ▼
Analysis   Generation   Review
(fast)     (powerful)   (balanced)
```

### Task Routing Strategies

| Strategy | Model Selection | Use Case |
|----------|-----------------|----------|
| **Economy** | Cheapest + fastest (Gemini Flash) | Triage, classification |
| **Balanced** | Mid-tier model (Gemini Pro) | Code generation, analysis |
| **Performance** | Most capable (Gemini Ultra, GPT-4) | Complex generation, review |

**Configuration:**

```yaml
llm:
  provider: gemini
  model: gemini-2.5-flash
  temperature: 0.5
  max_tokens: 2000

multi_model:
  task_routing:
    analysis: "economy"      # Use fast model
    generation: "performance" # Use powerful model
    review: "balanced"        # Use mid-tier model
```

### Token-Aware Context Management

- **Budget per analysis:** 30,000 tokens
- **Soft limit:** Compress context at 25,000 tokens
- **Hard limit:** Stop processing at 30,000 tokens

**Context compression strategy:**

```python
class ContextManager:
    def __init__(self, budget: int = 30000):
        self.budget = budget
        self.used = 0

    async def compress_if_needed(self, context: str) -> str:
        """Compress context if approaching limit."""
        if self.used > self.budget * 0.85:  # 85% of budget
            # Use LLM to summarize context
            summarized = await self.llm.summarize(context)
            return summarized
        return context
```

---

## Memory & Persistence Layer

### SQLite Schema (6 Tables)

```sql
-- Track analyzed repositories
CREATE TABLE analyzed_repos (
    id INTEGER PRIMARY KEY,
    repo_id TEXT UNIQUE,
    owner TEXT,
    name TEXT,
    url TEXT,
    language TEXT,
    last_analyzed TIMESTAMP,
    findings_count INTEGER,
    status TEXT -- 'success', 'failed', 'skipped'
);

-- All PRs created by ContribAI
CREATE TABLE submitted_prs (
    id INTEGER PRIMARY KEY,
    repo_id TEXT,
    pr_number INTEGER,
    url TEXT,
    title TEXT,
    status TEXT -- 'open', 'merged', 'closed', 'draft'
    created_at TIMESTAMP,
    merged_at TIMESTAMP
);

-- Cached analysis findings
CREATE TABLE findings_cache (
    id INTEGER PRIMARY KEY,
    repo_id TEXT,
    findings_json TEXT,
    timestamp TIMESTAMP,
    ttl_expires TIMESTAMP
);

-- Pipeline execution history
CREATE TABLE run_log (
    id INTEGER PRIMARY KEY,
    timestamp TIMESTAMP,
    status TEXT,
    repos_analyzed INTEGER,
    prs_created INTEGER,
    errors_count INTEGER
);

-- PR outcomes for learning
CREATE TABLE pr_outcomes (
    id INTEGER PRIMARY KEY,
    repo_id TEXT,
    pr_number INTEGER,
    outcome TEXT -- 'merged', 'closed', 'rejected'
    feedback TEXT,
    time_to_close_hours FLOAT
);

-- Learned repo preferences
CREATE TABLE repo_preferences (
    id INTEGER PRIMARY KEY,
    repo_id TEXT UNIQUE,
    preferred_types TEXT, -- JSON array
    rejected_types TEXT,  -- JSON array
    merge_rate FLOAT,
    avg_review_hours FLOAT
);
```

### Working Memory (Per-Repo Cache)

- **Storage:** SQLite in `findings_cache` table
- **Key:** `repo_id`
- **TTL:** 72 hours (auto-expire)
- **Auto-load:** When analyzing same repo twice
- **Auto-save:** After each analysis

```python
class Memory:
    async def get_cached_findings(self, repo_id: str) -> Optional[List[Finding]]:
        """Load findings from cache (if < 72h old)."""
        result = await self.db.query(
            "SELECT findings_json FROM findings_cache WHERE repo_id = ? AND ttl_expires > ?",
            (repo_id, datetime.now())
        )
        if result:
            return json.loads(result[0]["findings_json"])
        return None

    async def save_findings(self, repo_id: str, findings: List[Finding]):
        """Save findings to cache (72h TTL)."""
        now = datetime.now()
        expires = now + timedelta(hours=72)
        await self.db.insert("findings_cache", {
            "repo_id": repo_id,
            "findings_json": json.dumps([f.dict() for f in findings]),
            "timestamp": now,
            "ttl_expires": expires
        })
```

### Learning from PR Outcomes

Memory tracks PR success/failure to learn repo preferences:

```python
async def record_outcome(
    self,
    repo: Repository,
    pr_number: int,
    outcome: str,  # 'merged', 'closed', 'rejected'
    feedback: str,
    time_to_close_hours: float
):
    """Record PR outcome and update repo preferences."""
    # Save outcome
    await self.db.insert("pr_outcomes", {
        "repo_id": repo.id,
        "pr_number": pr_number,
        "outcome": outcome,
        "feedback": feedback,
        "time_to_close_hours": time_to_close_hours
    })

    # Learn from outcome
    if outcome == "merged":
        await self.mark_preferred_type(repo.id, finding_type)
    elif outcome == "rejected":
        await self.mark_rejected_type(repo.id, finding_type)
```

---

## MCP Server (Model Context Protocol)

Claude Desktop integration via stdio JSON-RPC.

### 14 Exposed Tools

**GitHub Read (5 tools):**
- `search_repos` — Search GitHub by language/stars
- `get_repo_info` — Fetch repo metadata
- `get_file_tree` — List repo structure
- `get_file_content` — Read file contents
- `get_open_issues` — List open issues

**GitHub Write (4 tools):**
- `fork_repo` — Fork a repository
- `create_branch` — Create feature branch
- `push_file_change` — Commit changes
- `create_pr` — Create pull request

**Safety (2 tools):**
- `check_duplicate_pr` — Detect if PR already exists
- `check_ai_policy` — Check if repo bans AI contributions

**Maintenance (3 tools):**
- `patrol_prs` — Monitor open PRs for feedback
- `cleanup_forks` — Remove stale forks
- `get_stats` — Return overall statistics

### Tool Protocol (Extensible)

```python
from typing import Protocol

class Tool(Protocol):
    """MCP-compatible tool interface."""
    name: str
    description: str

    async def execute(self, **kwargs) -> ToolResult:
        """Execute tool with arguments."""
        ...

@dataclass
class ToolResult:
    success: bool
    data: Any
    error: Optional[str] = None
```

---

## Configuration Structure

### Configuration Hierarchy

```yaml
# config.yaml (full structure)

github:
  token: "ghp_..."
  max_prs_per_day: 15
  rate_limit_margin: 100
  fork_timeout_seconds: 30

llm:
  provider: "gemini"  # gemini, openai, anthropic, ollama, vertex
  model: "gemini-2.5-flash"
  api_key: "GEMINI_API_KEY"
  temperature: 0.5
  max_tokens: 2000
  timeout_seconds: 60

discovery:
  languages: ["python", "javascript"]
  stars_range: [100, 5000]
  min_activity_days: 180
  exclude_repos: []

analysis:
  enabled_analyzers:
    - security
    - code_quality
    - performance
    - documentation
    - ui_ux
    - refactoring
  max_file_size_kb: 50
  skip_patterns: ["*.md", "*.yaml", "*.json"]

contribution:
  pr_style: "professional"  # or "minimal"
  commit_format: "conventional"
  include_explanation: true

pipeline:
  concurrent_repos: 3
  retry_attempts: 2
  retry_backoff_seconds: 2
  timeout_seconds: 300

multi_model:
  task_routing:
    analysis: "economy"
    generation: "performance"
    review: "balanced"

notifications:
  enabled: true
  channels:
    slack: "SLACK_WEBHOOK_URL"
    discord: "DISCORD_WEBHOOK_URL"
```

---

## Error Handling Strategy

### Exception Hierarchy

```python
# All exceptions inherit from ContribAIError

ContribAIError (base)
├── AnalysisError
│   ├── SecurityCheckError
│   └── FrameworkDetectionError
├── GenerationError
│   ├── CodeSyntaxError
│   └── QualityCheckError
├── GitHubError
│   ├── RepoNotFoundError
│   ├── PermissionError
│   └── RateLimitError
├── LLMError
│   ├── ProviderUnavailableError
│   └── ContextLimitExceeded
├── ConfigError
└── ValidationError
```

### Failure Recovery

| Error Type | Handling | Recovery |
|-----------|----------|----------|
| **GitHub 5xx** | Log warning | Retry up to 2x with backoff |
| **LLM timeout** | Log error | Retry with shorter context |
| **Rate limit** | Log warning | Skip repo, continue to next |
| **Invalid config** | Log error | Fail fast, exit with error |
| **Database error** | Log error | Crash & restart (systemd) |

---

## Dependency Flow Diagram

```
┌──────────────────────────────────────────────────┐
│                 CLI / Web / Scheduler            │
│              (contribai.cli, .web, .scheduler)   │
└──────────────────┬───────────────────────────────┘
                   │
        ┌──────────┴──────────┐
        ▼                     ▼
   ┌──────────┐         ┌──────────────┐
   │Orchestrator       │ Agents       │
   │(Pipeline,        │(Registry with 4
   │Hunt, Memory)    │ sub-agents)
   └──────┬───┘         └──────┬───────┘
          │                    │
    ┌─────┴────┬────┬──────────┤
    │           │    │         │
    ▼           ▼    ▼         ▼
┌────────┐┌─────────┐┌──┐┌─────────┐
│Analysis││Generator││PR││ Issues  │
│        ││         ││Mgr│ Solver  │
└───┬────┘└────┬────┘└─┬┘└────┬────┘
    │          │       │     │
    └──────────┼───────┴─────┘
               │
        ┌──────┴──────┐
        ▼             ▼
    ┌────────┐   ┌──────────┐
    │  LLM   │   │  GitHub  │
    │Provider│   │  Client  │
    └────┬───┘   └────┬─────┘
         │             │
         └─────┬───────┘
               ▼
         ┌──────────────┐
         │   CORE       │
         │ (Config,     │
         │  Models,     │
         │  Events,     │
         │  Exceptions) │
         └──────────────┘
```

**All arrows point downward (acyclic dependency graph).**

---

## Document Metadata

- **Created:** 2026-03-28
- **Last Updated:** 2026-03-28
- **Owner:** Tech Lead / Architecture Team
- **References:** README.md, docs/ARCHITECTURE.md, docs/codebase-summary.md
