# Project Roadmap

**Current Version:** 3.0.6 | **Release Date:** 2026-03-28 | **Status:** Active Development

---

## Executive Summary

ContribAI is a mature autonomous AI contribution system with a robust foundation (v1.0-v2.x) and recent architectural improvements (v3.0+). The roadmap focuses on production maturity, enterprise scalability, and ecosystem expansion.

---

## Release Timeline

### v0.x Series (2026-03-17 to 2026-03-20) — Foundation Building

| Version | Date | Milestone | Status |
|---------|------|-----------|--------|
| **v0.1** | 2026-03-17 | Core pipeline (discovery → analysis → generation → PR) | ✓ Complete |
| **v0.4** | 2026-03-18 | Web dashboard + REST API | ✓ Complete |
| **v0.5** | 2026-03-18 | Scheduler + cron automation | ✓ Complete |
| **v0.7** | 2026-03-19 | Multi-LLM support (Gemini, OpenAI, Anthropic, Ollama) | ✓ Complete |
| **v0.11** | 2026-03-20 | Hunt Mode (autonomous multi-round hunting) | ✓ Complete |

**Key Achievements:**
- ✓ Autonomous PR creation at scale
- ✓ Multi-strategy code analysis (7 analyzers)
- ✓ Multi-provider LLM routing
- ✓ Web dashboard for monitoring
- ✓ Hunt mode for large-scale discovery

---

### v1.x Series (2026-03-20) — Quality & Safety

| Version | Date | Milestone | Status |
|---------|------|-----------|--------|
| **v1.0** | 2026-03-20 | Official release; quality improvements | ✓ Complete |
| **v1.5** | 2026-03-20 | CLA/DCO handling; compliance automation | ✓ Complete |
| **v1.8** | 2026-03-20 | Cross-file pattern detection + bulk fixes | ✓ Complete |

**Key Achievements:**
- ✓ AI policy detection (respects contributor bans)
- ✓ CLA auto-signing (CLA-Assistant, EasyCLA)
- ✓ DCO auto-signoff on commits
- ✓ Deep validation (LLM validates findings)
- ✓ Cross-file fix capability

---

### v2.x Series (2026-03-22 to 2026-03-26) — Learning & Resilience

| Version | Date | Milestone | Status |
|---------|------|-----------|--------|
| **v2.0** | 2026-03-22 | Safety gates (quality scorer, duplicate prevention) | ✓ Complete |
| **v2.2** | 2026-03-23 | PR Patrol (auto-monitor + auto-fix feedback) | ✓ Complete |
| **v2.4** | 2026-03-25 | Outcome memory (learns from PR results) | ✓ Complete |
| **v2.6** | 2026-03-26 | MCP server (14 tools for Claude Desktop) | ✓ Complete |
| **v2.7** | 2026-03-26 | Event bus (15 typed events) + working memory | ✓ Complete |
| **v2.8** | 2026-03-26 | Context compression + progressive skills | ✓ Complete |

**Key Achievements:**
- ✓ Quality scoring (7-check gate, 0.6 min threshold)
- ✓ Duplicate PR prevention (title similarity matching)
- ✓ PR monitoring + auto-fix on review feedback
- ✓ Learning from PR outcomes (repo preferences)
- ✓ Event bus for audit trail + integrations
- ✓ Working memory with 72h TTL
- ✓ MCP server for Claude Desktop integration

---

### v3.x Series (2026-03-26 to Present) — Production Hardening & Ecosystem

| Version | Date | Milestone | Status |
|---------|------|-----------|--------|
| **v3.0.0** | 2026-03-26 | EventBus system, Formatter, MCP Client, CLI flags | ✓ Complete |
| **v3.0.2** | 2026-03-28 | Token efficiency, bug fixes, doc generation | ✓ Complete |
| **v3.0.4** | 2026-03-28 | Security hardening (constant-time API keys, webhook validation) | ✓ Complete |
| **v3.0.5** | 2026-03-28 | Critical bug fixes (webhook bypass, retry re-entry, context compressor) | ✓ Complete |
| **v3.0.6** | 2026-03-28 | SKIP_DIRECTORIES filter, auto-close linked issues, HALL_OF_FAME | ✓ Complete |

**Key Achievements (v3.0.x):**
- ✓ Enhanced event bus with JSONL logging
- ✓ LLM output formatter for consistent parsing
- ✓ MCP client for internal communication
- ✓ Improved CLI flags and help system
- ✓ Code generation quality enhancements
- ✓ Fuzzy matching for PR similarity detection
- ✓ Robust JSON extraction from LLM outputs
- ✓ Comprehensive documentation suite

---

## Feature Status Matrix

### Core Pipeline (v3.0.6)

| Feature | Status | Details |
|---------|--------|---------|
| Repository discovery | ✓ Complete | GitHub Search API with filters |
| Multi-strategy analysis | ✓ Complete | 7 analyzers, 17 skills, framework detection |
| LLM-powered generation | ✓ Complete | Multi-provider routing, self-review, quality scoring |
| Autonomous PR creation | ✓ Complete | Fork, branch, commit, PR, CLA/DCO handling |
| Hunt mode (multi-round) | ✓ Complete | Configurable rounds, inter-repo delays, deduplication |
| Cross-file fixes | ✓ Complete | Bulk fix capability for pattern repetition |
| Issue-driven solving | ✓ Complete | Fetch + solve open GitHub issues |

### Safety & Compliance (v3.0.6)

| Feature | Status | Details |
|---------|--------|---------|
| Rate limiting | ✓ Complete | Daily PR limit (configurable) + API rate respect |
| Quality gate | ✓ Complete | 7-check scorer, 0.6 min score threshold |
| Duplicate prevention | ✓ Complete | Title similarity matching (>90% = duplicate) |
| AI policy detection | ✓ Complete | Parse CONTRIBUTING.md for AI bans |
| CLA auto-signing | ✓ Complete | CLA-Assistant, EasyCLA integration |
| DCO signoff | ✓ Complete | Auto-append to all commits |
| Deep validation | ✓ Complete | LLM validates findings vs. full file context |
| Dry-run mode | ✓ Complete | Preview without PR creation |

### Platform Features (v3.0.6)

| Feature | Status | Details |
|---------|--------|---------|
| Web dashboard | ✓ Complete | FastAPI REST API + static UI at `:8787` |
| Scheduler | ✓ Complete | APScheduler cron automation |
| Webhooks | ✓ Complete | GitHub webhook receiver for auto-triggering |
| Profiles | ✓ Complete | Named presets (security-focused, docs-focused, etc.) |
| Templates | ✓ Complete | 5 built-in YAML contribution templates |
| Plugins | ✓ Complete | Entry-point plugin system for extensions |
| Notifications | ✓ Complete | Slack, Discord, Telegram integrations |
| API quotas | ✓ Complete | Daily usage tracking + limits |

### Architecture & Internals (v3.0.6)

| Feature | Status | Details |
|---------|--------|---------|
| Event bus | ✓ Complete | 15 typed events, JSONL logging |
| Sub-agent registry | ✓ Complete | 4 built-in agents (Analyzer, Generator, Patrol, Compliance) |
| Context compression | ✓ Complete | LLM-driven summarization + token budgeting |
| Working memory | ✓ Complete | Per-repo cache with 72h TTL |
| Outcome learning | ✓ Complete | PR outcome tracking + repo preference learning |
| MCP server | ✓ Complete | 14 tools for Claude Desktop integration |
| Error handling | ✓ Complete | Custom exception hierarchy + graceful recovery |
| Async-first design | ✓ Complete | All I/O operations async (asyncio) |

---

## Completed Milestones

### Milestone 1: MVP (v0.1 - v0.11) ✓
**Goal:** Autonomously contribute to open source at scale
- ✓ Pipeline discovery → analysis → generation → PR
- ✓ 7 multi-strategy analyzers
- ✓ Multi-LLM support
- ✓ Hunt mode (autonomous discovery)
- ✓ Web dashboard

### Milestone 2: Safety & Learning (v1.0 - v2.8) ✓
**Goal:** Safe, learning system that improves over time
- ✓ Quality scoring + duplicate prevention
- ✓ CLA/DCO compliance
- ✓ PR patrol + auto-fix
- ✓ Outcome memory + learning
- ✓ Event bus + working memory

### Milestone 3: Production & Ecosystem (v3.0+) — IN PROGRESS
**Goal:** Enterprise-ready with plugin ecosystem
- ✓ MCP server (Claude Desktop integration)
- ✓ Enhanced code generation quality
- ✓ Comprehensive documentation
- ⚙ Advanced context compression
- ⚙ Plugin marketplace / registry
- ⚙ Distributed deployment (PostgreSQL backend)
- ⚙ Advanced monitoring & observability

---

## Planned Features (v3.1+)

### v3.1.0 — Enterprise Scalability (Q2 2025)

**Goals:**
- Multi-instance deployment support
- Distributed rate limiting via Redis
- PostgreSQL backend for production
- Observability suite (Prometheus metrics, OpenTelemetry)

**Planned Features:**
- [ ] PostgreSQL migration layer (drop-in replacement for SQLite)
- [ ] Redis-based distributed rate limiting
- [ ] Prometheus metrics export
- [ ] OpenTelemetry distributed tracing
- [ ] Kubernetes deployment manifests
- [ ] Helm charts for easy cluster deployment
- [ ] High-availability setup guide
- [ ] Multi-region deployment patterns

**Expected Impact:** Enable enterprise deployments handling 1000+ repos/day

---

### v3.2.0 — Advanced Context & Reasoning (Q3 2025)

**Goals:**
- Semantic code understanding
- Multi-step reasoning for complex fixes
- Code review feedback integration

**Planned Features:**
- [ ] Semantic code chunking (not just truncation)
- [ ] Code2Vec embeddings for similarity matching
- [ ] Multi-turn LLM conversations for reasoning
- [ ] Integrate Coderabbit/DeepCode feedback into generation
- [ ] Abstract syntax tree (AST) analysis for structural patterns
- [ ] Type-aware code generation (leverage type stubs, type hints)

**Expected Impact:** 15% improvement in PR success rate

---

### v3.3.0 — Plugin Marketplace (Q4 2025)

**Goals:**
- Community extension ecosystem
- Pre-built plugins for popular frameworks
- Plugin discovery & versioning

**Planned Features:**
- [ ] Central plugin registry (GitHub-based)
- [ ] Plugin package format standardization
- [ ] Plugin versioning & dependency resolution
- [ ] Pre-built plugins: Django patterns, React hooks, async/await optimization
- [ ] Plugin rating/review system
- [ ] Plugin security scanning (supply chain)
- [ ] Plugin sandbox execution

**Expected Impact:** Community-driven analyzer/generator extensions

---

### v3.4.0 — GPT-4o Integration & Advanced Models (Q1 2026)

**Goals:**
- Support latest model releases
- Advanced code understanding
- Multimodal analysis (images, diagrams)

**Planned Features:**
- [ ] OpenAI GPT-4o integration
- [ ] Claude 3.5+ native support (beyond API)
- [ ] Multimodal analysis (UI screenshots, architecture diagrams)
- [ ] Vision-based UI/UX issue detection
- [ ] Code review feedback parsing with vision
- [ ] Advanced model-specific optimizations

**Expected Impact:** Better code understanding, 20% improvement in fix quality

---

### v4.0.0 — Full Agent Autonomy (2026 H2)

**Goals:**
- True agent-to-agent collaboration
- Self-improving capabilities
- Custom tool creation

**Planned Features:**
- [ ] Agent-to-agent communication protocol
- [ ] Tool creation framework (agents define custom tools)
- [ ] Self-evaluation + automatic improvement loops
- [ ] Feedback integration from maintainers (via comments)
- [ ] Reputation system (track agent performance per repo)
- [ ] Multi-agent coordination (spec → design → implement → test)

**Expected Impact:** Autonomous end-to-end solution delivery (from issue to merged PR)

---

## Technical Debt & Improvements

### High Priority (v3.0.x Hotfixes)

| Item | Effort | Impact | Status |
|------|--------|--------|--------|
| Reduce token consumption in context compression | Medium | Reduce LLM API cost by 20% | ⚙ In Progress |
| Improve LLM JSON extraction robustness | Small | Reduce parsing errors | ⚙ In Progress |
| Add comprehensive error recovery | Medium | Improve reliability | ⚙ In Progress |
| Enhance GitHub rate limit handling | Small | Smoother operation at scale | ⚙ In Progress |

### Medium Priority (v3.1.0)

| Item | Effort | Impact | Status |
|------|--------|--------|--------|
| Refactor analysis pipeline (composition over inheritance) | Large | Better maintainability | Planned |
| Add structured logging (JSON format) | Medium | Better observability | Planned |
| Extract tool protocol into separate package | Medium | Enable third-party tools | Planned |
| Optimize database queries (add indexes) | Small | Improve memory access | Planned |

### Low Priority (v3.2.0+)

| Item | Effort | Impact | Status |
|------|--------|--------|--------|
| Migrate from Rich CLI to interactive TUI | Large | Better UX | Planned |
| Add telemetry (opt-in) | Medium | Usage insights | Planned |
| Implement time-series analytics | Large | Trend analysis | Backlog |

---

## Success Metrics & KPIs

### Current State (v3.0.6)

| Metric | Current | Target (v3.1) | Target (v4.0) |
|--------|---------|---|---|
| **Repos analyzed/day** | 50+ | 200+ | 1000+ |
| **PRs created/day** | 10-15 | 50+ | 200+ |
| **PR merge rate** | ~26% (9/34) | 50% | 75% |
| **Avg time-to-merge** | 7 days | 5 days | 3 days |
| **False positive rate** | ~15% | <5% | <1% |
| **Code quality score avg** | 0.75/1.0 | 0.80/1.0 | 0.85/1.0 |
| **Test coverage** | 53% | 70% | 85% |

### User Engagement

| Metric | Current | Target (v3.1) | Target (v4.0) |
|--------|---------|---|---|
| **GitHub stars** | ~180 | 500+ | 2000+ |
| **Active instances** | 1 | 10+ | 100+ |
| **Plugin ecosystem** | 0 | 5+ | 50+ |
| **Community contributions** | Low | Medium | High |

---

## Known Limitations & Future Improvements

### Current Limitations (v3.0.6)

1. **Single-instance only** — No distributed deployment yet
   - **Fix planned:** v3.1.0 (PostgreSQL + Redis)

2. **Token budget not semantic** — Simple truncation-based compression
   - **Fix planned:** v3.2.0 (semantic chunking + embeddings)

3. **No multimodal analysis** — Can't analyze images/diagrams
   - **Fix planned:** v3.4.0 (GPT-4o, vision models)

4. **Limited to GitHub** — No GitLab, Gitea, Gitee support
   - **Fix planned:** v4.0.0 (pluggable VCS)

5. **No agent-to-agent communication** — Agents are independent
   - **Fix planned:** v4.0.0 (inter-agent protocol)

---

## Dependency & Risk Assessment

### Key Dependencies

| Dependency | Risk Level | Mitigation |
|-----------|-----------|-----------|
| **Google Gemini API** | Medium | Have OpenAI/Anthropic fallbacks |
| **GitHub API** | Low | Proper rate limiting, retry logic |
| **Python 3.11+** | Low | Community support strong |
| **asyncio ecosystem** | Low | Mature, well-tested |

### Risk Mitigation Strategies

- **API provider lock-in:** Multi-provider support baked in from v0.7
- **GitHub breaking changes:** Monitor API deprecations, test in CI
- **Security vulnerabilities:** `pip-audit` in CI, auto-dependency updates
- **License compliance:** AGPL-3.0 + Commons Clause enforced

---

## Contributing to Roadmap

### How to Propose Features

1. Open a GitHub Discussion with `[RFC]` prefix
2. Describe use case + expected impact
3. Link to any research/design docs
4. Community votes via reactions
5. Team evaluates for inclusion in next release

### Feature Acceptance Criteria

- **Clear use case** — Solves real problem for 5+ users
- **Low breaking changes** — Backward compatible or clear migration
- **Testable** — Can write unit + integration tests
- **Documented** — Includes user guide + API docs
- **Community interest** — 10+ upvotes on discussion

---

## Document Metadata

- **Created:** 2026-03-28
- **Last Updated:** 2026-03-28
- **Owner:** Product Manager / Tech Lead
- **References:** README.md, CHANGELOG.md, docs/project-overview-pdr.md
- **Next Review:** 2026-06-30 (end of Q2)
