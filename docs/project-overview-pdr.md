# ContribAI — Project Overview & PDR

**Version:** 3.0.2 | **License:** AGPL-3.0 + Commons Clause | **Status:** Active Development

---

## Executive Summary

**ContribAI** is an autonomous AI agent that discovers open source repositories on GitHub, analyzes them for improvement opportunities, generates high-quality code fixes, and submits Pull Requests — all without human intervention. It bridges the gap between maintainer bandwidth constraints and contributor availability by delivering production-grade contributions at scale.

---

## Product Definition

### Target Users

1. **Open Source Maintainers** — Reduce issue backlog via autonomous quality contributions
2. **Project Leaders** — Accelerate project evolution with AI-powered improvements
3. **Enterprise Operators** — Deploy in-house for internal projects or customer repos
4. **AI/ML Researchers** — Study autonomous contribution patterns, agent design, code generation quality

### Core Value Proposition

Autonomous, safe, high-quality code contributions that:
- **Reduce manual labor** — No human reviews needed for obvious improvements
- **Increase contribution velocity** — 10-15 PRs per day per instance
- **Maintain quality standards** — 7-check gate prevents low-quality submissions
- **Respect maintainer intent** — Learns from PR outcomes, avoids rejected patterns
- **Operate safely** — Rate limiting, AI policy detection, duplicate prevention

### Key Features

#### Core Pipeline (v0.1+)
- **Smart Discovery** — Finds contribution-friendly repos by language, stars, activity
- **Multi-Strategy Analysis** — 7 analyzers (security, code quality, docs, UI/UX, performance, refactoring, framework-specific)
- **High-Quality Generation** — LLM-powered fixes with self-review and quality scoring
- **Autonomous PR Creation** — Fork, branch, commit, PR — all automated with DCO signoff

#### Hunt Mode (v0.11.0+)
- Autonomous hunting across GitHub at scale (configurable rounds + delays)
- Cross-file pattern detection and bulk fixes
- Duplicate PR prevention via title similarity
- Post-PR CI monitoring with auto-close on failures

#### Resilience & Safety (v2.0.0+)
- AI policy detection (respects contributor bans)
- CLA/DCO auto-signing
- Deep validation reduces false positives
- Rate limiting (max 2 findings per repo)
- API retry with exponential backoff
- Code-only modifications (skips docs/config/meta files)

#### PR Patrol (v2.2.0+)
- Reviews open PRs for maintainer feedback
- Bot-aware feedback classification (11+ known bots filtered)
- Auto-generates fixes based on review feedback
- DCO auto-signoff on commits

#### Multi-LLM Support (v0.7.0+)
- **Primary:** Google Gemini (Flash, Pro)
- **Alternates:** OpenAI, Anthropic, Ollama, Google Vertex AI
- Task routing (performance/balanced/economy strategies)
- Token-aware context budgeting (30k token limit)

#### Agent Architecture (v2.7.0+)
- Sub-agent registry with 4 built-in agents (Analyzer, Generator, Patrol, Compliance)
- Event bus (15 typed events, JSONL logging)
- Working memory (per-repo analysis context, 72h TTL)
- Context compression (LLM-driven + truncation)
- DeerFlow/AgentScope-inspired design

#### Platform Features (v0.4.0+)
- **Web Dashboard** — FastAPI REST API + static dashboard (`:8787`)
- **Scheduler** — APScheduler cron-based automation
- **Templates** — 5 built-in contribution templates (YAML-based)
- **Profiles** — Named presets (security-focused, docs-focused, full-scan, gentle)
- **Plugins** — Entry-point based system for custom analyzers/generators
- **Webhooks** — GitHub webhook receiver for auto-triggering
- **Quotas** — Daily limits on GitHub + LLM API usage
- **Docker** — Full docker-compose setup (dashboard, scheduler, runner)

#### MCP Server (v2.6.0+)
- 14 tools exposed to Claude Desktop
- Safe read/write operations on GitHub
- Rate limit handling, cleanup utilities

---

## Technical Requirements

### Core Requirements

| Requirement | Details |
|------------|---------|
| **Python** | 3.11+ (3.12, 3.13 supported) |
| **GitHub** | Token (PAT) with `repo` + `workflow` scopes |
| **LLM API** | Gemini/OpenAI/Anthropic/Vertex AI API key |
| **Database** | SQLite 3.x (auto-initialized) |
| **OS** | Linux, macOS, Windows (via WSL) |

### Core Dependencies

- **Web Framework:** FastAPI, Uvicorn, Jinja2
- **GitHub Client:** GitPython, httpx (async)
- **LLM Providers:** google-genai, openai, anthropic
- **Data:** Pydantic, aiosqlite
- **CLI:** Click, Rich (TUI)
- **Tasks:** APScheduler (cron)
- **Notifications:** Slack/Discord/Telegram SDKs
- **Code Validation:** Docker (optional), ast.parse (fallback)

### Optional Dependencies

- **Docker** — For sandbox code validation
- **Redis** — For distributed rate limiting (future)
- **PostgreSQL** — For multi-instance deployments (future)

---

## Functional Requirements

### Discovery (FR-D)

| ID | Requirement | Implementation |
|----|----|---|
| FR-D.1 | Search GitHub by language + star range | GitHub Search API with configurable filters |
| FR-D.2 | Filter inactive repos | Skip if last commit > 6 months ago |
| FR-D.3 | Detect contribution bans | Parse CONTRIBUTING.md for AI policy |
| FR-D.4 | Prevent duplicate analysis | SQLite `analyzed_repos` table |
| FR-D.5 | Support multiple languages | Language detection via file extensions + linguist |

### Analysis (FR-A)

| ID | Requirement | Implementation |
|----|----|---|
| FR-A.1 | Detect security issues | Security analyzer + skills |
| FR-A.2 | Detect code quality issues | Code quality analyzer + rules |
| FR-A.3 | Detect documentation gaps | Doc analyzer |
| FR-A.4 | Detect UI/UX issues | UI/UX analyzer |
| FR-A.5 | Detect performance problems | Performance analyzer |
| FR-A.6 | Framework-specific detection | Auto-detect Django/Flask/FastAPI/React/etc. |
| FR-A.7 | Progressive skill loading | Load 17 skills on-demand by language |
| FR-A.8 | Deep validation | LLM validates findings against full file context |

### Generation (FR-G)

| ID | Requirement | Implementation |
|----|----|---|
| FR-G.1 | Generate code fixes | LLM-powered generation with retry |
| FR-G.2 | Self-review generated code | LLM reviews own fixes before submission |
| FR-G.3 | Quality scoring | 7-check gate (min 0.6/1.0 score) |
| FR-G.4 | Syntax validation | Pre-submit checks (balanced brackets, no-op detection) |
| FR-G.5 | Support multiple languages | Generate code in Python, JavaScript, Go, Rust, Java, etc. |
| FR-G.6 | Cross-file fixes | Detect same pattern across files, fix all |

### PR Management (FR-PR)

| ID | Requirement | Implementation |
|----|----|---|
| FR-PR.1 | Auto-fork repo | GitHub API fork operation |
| FR-PR.2 | Create feature branch | git branch creation |
| FR-PR.3 | Commit with DCO signoff | Auto-append `Signed-off-by` |
| FR-PR.4 | Create PR with context | PR title + detailed body |
| FR-PR.5 | Auto-sign CLAs | Detect CLA service, auto-sign |
| FR-PR.6 | Monitor CI | Check PR status, auto-close if CI fails |
| FR-PR.7 | Monitor reviews | Track maintainer feedback, auto-fix if needed |
| FR-PR.8 | Duplicate prevention | Title similarity matching (>90% = duplicate) |

### Configuration (FR-C)

| ID | Requirement | Implementation |
|----|----|---|
| FR-C.1 | Load from YAML | `config.yaml` with schema validation |
| FR-C.2 | Environment overrides | Env vars override YAML values |
| FR-C.3 | Profile presets | Named configs (security-focused, etc.) |
| FR-C.4 | Per-repo customization | Templates + webhooks enable custom rules |

### Safety & Limits (FR-S)

| ID | Requirement | Implementation |
|----|----|---|
| FR-S.1 | Daily PR limit | Configurable (default: 15 PRs/day) |
| FR-S.2 | Per-repo limit | Max 2 findings per repo |
| FR-S.3 | Rate limiting | Respect GitHub + LLM API limits |
| FR-S.4 | Quality gate | Min 0.6 score before submission |
| FR-S.5 | Dry-run mode | Preview without creating PRs |
| FR-S.6 | API quotas | Track daily usage, enforce limits |

---

## Non-Functional Requirements

### Performance (NFR-P)

| Requirement | Target |
|---|---|
| Single repo analysis | < 2 minutes (incl. LLM calls) |
| PR creation | < 30 seconds |
| Hunt mode (10 repos) | < 30 minutes (with delays) |
| Memory footprint | < 500 MB |
| Dashboard API latency | < 500 ms (p95) |

### Reliability (NFR-R)

| Requirement | Target |
|---|---|
| Uptime (dashboard) | 99.5% (self-hosted) |
| CI pass rate (tests) | 100% |
| PR success rate | > 80% (merge within 7 days) |
| Recovery time (crash) | < 5 minutes (auto-restart) |

### Security (NFR-S)

| Requirement | Implementation |
|---|---|
| API key handling | Never log, use env vars only |
| Code execution | Sandboxed (Docker or ast.parse) |
| LLM output validation | Treated as untrusted, validated before execute |
| HTTPS | Required for web dashboard |
| Auth | API key authentication on dashboard |
| Dependency audits | `pip-audit` in CI, auto-fix on new vulns |

### Scalability (NFR-Sc)

| Requirement | Implementation |
|---|---|
| Multi-instance deployment | Docker + shared SQLite → future PostgreSQL |
| Parallel repo processing | asyncio.gather + Semaphore(3) |
| Async-first | All I/O operations async |
| Token budgeting | 30k token limit per analysis |

---

## Success Metrics

### Operational Metrics

| Metric | Target | Measurement |
|---|---|---|
| **Repos analyzed per day** | 50+ | `run_log` table count |
| **PRs created per day** | 10-15 | `submitted_prs` table count |
| **PR success rate** | > 80% | (merged_prs / total_prs) |
| **Avg time-to-merge** | < 7 days | `time_to_close_hours` in DB |
| **False positive rate** | < 5% | Manual audit |

### Quality Metrics

| Metric | Target | Measurement |
|---|---|---|
| **Code review comments** | < 2 per PR | GitHub API |
| **Rejection rate** | < 20% | PR close reason analysis |
| **Quality score avg** | > 0.75/1.0 | Scorer output |
| **Test coverage** | > 85% | pytest-cov |

### User Engagement

| Metric | Target | Measurement |
|---|---|---|
| **Active instances** | 50+ | Telemetry opt-in |
| **GitHub stars** | 500+ | Public interest |
| **Plugin ecosystem** | 5+ plugins | Community contributions |

---

## Constraints & Assumptions

### Constraints

1. **GitHub API Rate Limits** — 5,000 requests/hour (authenticated)
2. **LLM API Costs** — Pay-as-you-go; configurable daily budget
3. **Code Size** — Skips files > 50 KB
4. **License** — AGPL-3.0 + Commons Clause (open source, non-commercial)
5. **Python Version** — 3.11+ only (type hints, async patterns)

### Assumptions

1. Users have valid GitHub + LLM API credentials
2. Target repos have standard structure (README, src dir, tests)
3. Maintainers read PRs within 7 days
4. AI-generated contributions are acceptable in target community
5. Network connectivity is stable (retries handle transient failures)

---

## Roadmap Alignment

### Recent Releases

- **v3.0.0** (Mar 2024) — EventBus, Formatter, MCP Client, CLI flags
- **v3.0.1** (Mar 2024) — Code generation quality fixes
- **v3.0.2** (Current) — Token efficiency, bug fixes

### Planned (v3.1.0+)

- Advanced context compression with semantic chunking
- Plugin marketplace
- Web-based configuration builder
- Distributed deployment (PostgreSQL backend)
- GPT-4o integration for code review feedback

---

## Dependencies & Integrations

### External Services

- **GitHub API** — Repo discovery, file access, PR creation
- **LLM APIs** — Gemini, OpenAI, Anthropic, Vertex AI
- **Slack/Discord/Telegram** — Notifications (optional)
- **CLA Services** — CLA-Assistant, EasyCLA (auto-sign)

### Internal Integrations

- **Event Bus** — All major operations emit typed events
- **Memory System** — Learns from PR outcomes
- **Plugin System** — Custom analyzers/generators via entry points
- **Web Dashboard** — REST API for status, config, webhooks

---

## Glossary

| Term | Definition |
|------|-----------|
| **Hunt** | Autonomous multi-round repo search + contribution cycle |
| **Contribution** | A proposed code change (generated fix for a finding) |
| **Finding** | An issue detected by an analyzer (security bug, missing docstring, etc.) |
| **Skill** | Reusable knowledge module loaded on-demand (e.g., Django security patterns) |
| **Middleware** | Cross-cutting concern handler (rate limiting, validation, retry, DCO, quality gate) |
| **Sub-Agent** | Specialized executor (Analyzer, Generator, Patrol, Compliance) |
| **Profile** | Named preset configuration (security-focused, docs-focused, etc.) |
| **PR Patrol** | Automated monitoring + fixing of open PRs based on feedback |
| **CLA** | Contributor License Agreement (auto-signed) |
| **DCO** | Developer Certificate of Origin (auto-appended) |

---

## Document Metadata

- **Created:** 2026-03-28
- **Last Updated:** 2026-03-28
- **Owner:** Technical Writer / Documentation Team
- **Status:** Complete
- **Related:** README.md, docs/ARCHITECTURE.md, docs/code-standards.md
