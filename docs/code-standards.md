# Code Standards & Development Guidelines

**Version:** 3.0.2 | **Python:** 3.11+ | **Status:** Active

---

## Quick Reference

| Standard | Rule |
|----------|------|
| **Language** | Python 3.11+ (type hints required) |
| **Style** | ruff (autoformat + lint) |
| **Types** | Pydantic models for data, Python type hints for functions |
| **Async** | All I/O operations must be async |
| **Testing** | pytest + pytest-asyncio, > 85% coverage |
| **Database** | SQLite with aiosqlite (async) |
| **Errors** | Custom exceptions from `core.exceptions` |
| **Config** | Pydantic `BaseModel` with validation |
| **File Size** | Code files ≤ 200 LOC, split if larger |
| **Function Size** | ≤ 50 lines (excluding docstrings) |

---

## Python Conventions

### Type Hints (MANDATORY)

**All public APIs require type hints:**

```python
# Good ✓
async def analyze(
    repo: Repository,
    config: Config,
    skip_cache: bool = False
) -> AnalysisResult:
    """Analyze a repository for issues."""
    ...

# Bad ✗
async def analyze(repo, config, skip_cache=False):
    ...

# Type hints in class attributes
class Finding(BaseModel):
    type: str
    file: str
    line: int
    severity: Literal["critical", "high", "medium", "low"]
    description: str
```

### Async/Await (MANDATORY FOR I/O)

**All I/O operations are async, none are sync:**

```python
# Good ✓
async def fetch_repos(self) -> List[Repository]:
    async with self.http_client.get("/repos") as response:
        return await response.json()

# Bad ✗
def fetch_repos(self) -> List[Repository]:
    response = requests.get("/repos")  # Blocks entire app!
    return response.json()

# Async context managers
async with GitHubClient(token) as client:
    repos = await client.search_repos(language="python")
```

### Docstrings (Google-Style)

**Required for all public functions, classes, and methods:**

```python
async def generate_fix(
    self,
    finding: Finding,
    context: FileContext,
    max_tokens: int = 1000
) -> Contribution:
    """Generate a code fix for a finding.

    Uses LLM to generate a targeted fix, validates syntax,
    and scores quality before returning.

    Args:
        finding: Issue detected by analyzer.
        context: File content and surrounding context.
        max_tokens: Max tokens for LLM response.

    Returns:
        Contribution with generated code and explanation.

    Raises:
        GenerationError: If LLM fails or quality < threshold.
        ValidationError: If generated code has syntax errors.

    Example:
        >>> finding = Finding(type="missing_docstring", ...)
        >>> fix = await generator.generate_fix(finding, context)
        >>> print(fix.code_change)
    """
```

### Imports Organization

**Follow this import order (PEP 8):**

```python
# 1. Standard library (alphabetical)
import asyncio
import logging
from datetime import datetime
from typing import TYPE_CHECKING, List, Optional

# 2. Third-party (alphabetical)
import httpx
import pydantic
from pydantic import BaseModel, Field

# 3. Local imports (alphabetical)
from contribai.core.config import Config
from contribai.core.exceptions import AnalysisError
from contribai.core.models import Finding, Repository

# 4. Type-only imports (avoid circular dependencies)
if TYPE_CHECKING:
    from contribai.llm.provider import LLMProvider
```

### Constants & Enums

**Use constants for magic numbers/strings:**

```python
# Good ✓
DEFAULT_TIMEOUT_SECONDS = 30
MAX_FINDINGS_PER_REPO = 2
QUALITY_THRESHOLD = 0.6

class IssueType(str, Enum):
    SECURITY = "security"
    CODE_QUALITY = "code_quality"
    DOCUMENTATION = "documentation"

# Bad ✗
if timeout > 30:  # Magic number!
    ...

if issue_type == "security":  # Magic string!
    ...
```

### Logging (NOT Print)

**Use logger for all output, never print():**

```python
# Good ✓
import logging
logger = logging.getLogger(__name__)

logger.info(f"Analyzing repo: {repo.full_name}")
logger.debug(f"Found {len(findings)} issues")
logger.warning(f"API rate limit approaching: {remaining} requests left")
logger.error(f"Failed to create PR for {repo}: {error}", exc_info=True)

# Bad ✗
print(f"Analyzing repo: {repo.full_name}")  # Not logged!
print(f"Found {len(findings)} issues")
```

---

## Design Patterns

### Pattern 1: Provider/Strategy Pattern

**Use for extensible LLM providers, analyzers, generators:**

```python
class LLMProvider(ABC):
    """Abstract base for LLM implementations."""

    @abstractmethod
    async def complete(self, prompt: str, **kwargs) -> str:
        """Generate completion."""

class GeminiProvider(LLMProvider):
    async def complete(self, prompt: str, **kwargs) -> str:
        # Gemini-specific implementation
        ...

class OpenAIProvider(LLMProvider):
    async def complete(self, prompt: str, **kwargs) -> str:
        # OpenAI-specific implementation
        ...

# Factory function
def create_llm_provider(config: LLMConfig) -> LLMProvider:
    """Create LLM provider based on config."""
    if config.provider == "gemini":
        return GeminiProvider(api_key=config.api_key)
    elif config.provider == "openai":
        return OpenAIProvider(api_key=config.api_key)
    else:
        raise ValueError(f"Unknown provider: {config.provider}")
```

### Pattern 2: Middleware Chain

**Use for cross-cutting concerns (validation, retry, rate limit, DCO):**

```python
class Middleware(ABC):
    @abstractmethod
    async def process(
        self,
        repo: Repository,
        next_handler: Callable
    ) -> PipelineResult:
        """Process repo, then call next middleware."""

class RateLimitMiddleware(Middleware):
    async def process(self, repo: Repository, next_handler: Callable):
        # Check rate limit
        if not self.can_process(repo):
            raise RateLimitError("Daily PR limit exceeded")
        # Continue to next middleware
        return await next_handler(repo)

# Apply chain
pipeline.add_middleware(RateLimitMiddleware())
pipeline.add_middleware(ValidationMiddleware())
pipeline.add_middleware(RetryMiddleware())
result = await pipeline.process(repo)
```

### Pattern 3: Event Bus (Observer Pattern)

**Emit events for major actions; subscribe for side effects:**

```python
# Define typed event
@dataclass
class PRCreated(BaseModel):
    repo: Repository
    pr_number: int
    url: str
    timestamp: datetime

# Emit event
event_bus.emit(PRCreated(
    repo=repo,
    pr_number=123,
    url="https://github.com/...",
    timestamp=datetime.now()
))

# Subscribe to event
@event_bus.on(PRCreated)
async def notify_on_pr(event: PRCreated):
    await notifier.send(f"PR created: {event.url}")

@event_bus.on(PRCreated)
async def log_pr(event: PRCreated):
    logger.info(f"PR #{event.pr_number} for {event.repo.full_name}")
```

### Pattern 4: Pydantic Models for All Data

**Never use dicts for structured data; use Pydantic models:**

```python
# Good ✓
class Finding(BaseModel):
    type: str
    file: str
    line: int
    description: str
    severity: Literal["critical", "high", "medium", "low"]

    model_config = ConfigDict(
        validate_assignment=True,
        frozen=False  # Allow modification
    )

# Bad ✗
finding = {
    "type": "missing_docstring",
    "file": "app.py",
    "line": 42,
    "description": "Missing docstring",
    "severity": "low"
}  # No validation, no IDE support
```

### Pattern 5: Dependency Injection

**Pass dependencies as constructor parameters, not globals:**

```python
# Good ✓
class CodeAnalyzer:
    def __init__(
        self,
        llm_provider: LLMProvider,
        github_client: GitHubClient,
        config: Config
    ):
        self.llm = llm_provider
        self.github = github_client
        self.config = config

# Usage: inject at instantiation
analyzer = CodeAnalyzer(
    llm_provider=llm,
    github_client=github,
    config=config
)

# Bad ✗
class CodeAnalyzer:
    def __init__(self):
        # Creates its own dependencies (hard to test!)
        self.llm = create_llm_provider()  # Global state
        self.github = GitHubClient()
```

---

## Error Handling

### Exception Hierarchy

**Use custom exceptions from `contribai.core.exceptions`:**

```python
# Base exceptions
class ContribAIError(Exception):
    """Base exception for all ContribAI errors."""

class AnalysisError(ContribAIError):
    """Analysis phase failed."""

class GenerationError(ContribAIError):
    """Code generation failed."""

class GitHubError(ContribAIError):
    """GitHub API operation failed."""

class LLMError(ContribAIError):
    """LLM provider call failed."""

class ConfigError(ContribAIError):
    """Configuration error."""

# Usage
try:
    await analyzer.analyze(repo)
except AnalysisError as e:
    logger.error(f"Analysis failed: {e}", exc_info=True)
    # Handle gracefully
except GitHubError as e:
    logger.warning(f"GitHub API error: {e}")
    # Retry or skip repo
```

### Try/Except Best Practices

```python
# Good ✓
async def process_repo(repo: Repository) -> Optional[PipelineResult]:
    """Process a single repo, handling errors gracefully."""
    try:
        findings = await analyzer.analyze(repo)
        contributions = await generator.generate_fixes(findings)
        prs = await pr_manager.create_prs(repo, contributions)
        return PipelineResult(repo=repo, prs=prs)

    except AnalysisError as e:
        logger.error(f"Analysis failed for {repo.full_name}: {e}")
        return None

    except Exception as e:
        logger.error(f"Unexpected error processing {repo}: {e}", exc_info=True)
        return None

# Bad ✗
try:
    findings = await analyzer.analyze(repo)
    # ... more code
except:  # Catches everything, hard to debug!
    pass
```

---

## Testing Strategy

### Test Structure

```
tests/
├── unit/                   # Isolated module tests
│   ├── test_analyzer.py
│   ├── test_generator.py
│   ├── test_github_client.py
│   └── ...
├── integration/            # End-to-end tests
│   └── test_pipeline.py
└── conftest.py            # Shared fixtures
```

### Unit Test Example

```python
import pytest
from unittest.mock import AsyncMock, MagicMock
from contribai.analysis.analyzer import CodeAnalyzer
from contribai.core.models import Repository, Finding

@pytest.mark.asyncio
async def test_analyzer_detects_missing_docstrings():
    """Test that analyzer detects missing docstrings."""
    # Arrange
    repo = Repository(
        owner="test",
        name="repo",
        url="https://github.com/test/repo"
    )

    mock_llm = AsyncMock()
    mock_llm.complete.return_value = "Missing docstring on function `foo()`"

    analyzer = CodeAnalyzer(llm_provider=mock_llm)

    # Act
    findings = await analyzer.analyze(repo)

    # Assert
    assert len(findings) > 0
    assert any(f.type == "missing_docstring" for f in findings)
    mock_llm.complete.assert_called()

@pytest.mark.asyncio
async def test_analyzer_handles_github_errors():
    """Test error handling when GitHub API fails."""
    # Arrange
    mock_github = AsyncMock()
    mock_github.get_repo_tree.side_effect = GitHubError("404 Not Found")

    analyzer = CodeAnalyzer(github_client=mock_github)

    # Act & Assert
    with pytest.raises(GitHubError):
        await analyzer.analyze(repo)
```

### Test Fixtures (conftest.py)

```python
import pytest
from contribai.core.models import Repository, Config

@pytest.fixture
async def sample_repo():
    """Fixture providing a sample repository."""
    return Repository(
        owner="test",
        name="repo",
        url="https://github.com/test/repo",
        stars=100,
        language="python"
    )

@pytest.fixture
async def config():
    """Fixture providing test configuration."""
    return Config.from_yaml("config.example.yaml")

@pytest.fixture
async def github_client(mocker):
    """Mock GitHub client."""
    return mocker.AsyncMock()
```

### Coverage Requirements

- **Minimum:** > 85% (enforced in CI)
- **Target:** > 90% for critical paths (analysis, generation, PR)
- **Exclusions:** Logging, CLI arg parsing, trivial getters
- **Command:** `pytest tests/ -v --cov=contribai --cov-report=html`

---

## Code Quality Tools

### Ruff (Linting & Formatting)

**Run before every commit:**

```bash
# Check code style
ruff check contribai/ tests/

# Auto-fix issues
ruff format contribai/ tests/

# Check specific rule
ruff check --select E501 contribai/  # Line length
```

**Config in `pyproject.toml`:**

```toml
[tool.ruff]
line-length = 100
target-version = "py311"

[tool.ruff.lint]
select = ["E", "F", "W", "I", "N", "UP", "B"]  # Error, Undefined, Warning, Import, Naming, Upgrade, Bugbear
ignore = ["E501"]  # Line length (handled by formatter)
```

### Type Checking (Implicit via Pyright)

**Type hints are validated in CI. Run locally:**

```bash
# Check types (if pyright installed)
pyright contribai/
```

### Pre-Commit Checks

**Add to `.git/hooks/pre-commit` (auto-installed):**

```bash
#!/bin/bash
ruff check contribai/ tests/
pytest tests/ -x  # Stop on first failure
```

---

## File Organization

### Module Layout

```
contribai/module/
├── __init__.py              # Public API re-exports
├── main_component.py        # Primary class/functions
├── sub_component.py         # Supporting classes
├── constants.py             # Constants & Enums (optional)
├── exceptions.py            # Module-specific exceptions (optional)
└── utils.py                 # Helper functions (optional)
```

### Public API (__init__.py)

**Only export what's meant for public use:**

```python
# contribai/analysis/__init__.py
from .analyzer import CodeAnalyzer
from .strategies import SecurityStrategy, CodeQualityStrategy

__all__ = [
    "CodeAnalyzer",
    "SecurityStrategy",
    "CodeQualityStrategy",
]
```

### File Size Limits

| Type | Max LOC | Action |
|------|---------|--------|
| Python module | 200 | Split into components |
| Function | 50 | Extract sub-functions |
| Class | 300 | Split by responsibility |
| Test file | 500 | Create separate test files |
| Markdown doc | 800 | Create sub-docs + index |

---

## Configuration Management

### Pydantic Config Models

**All config must use Pydantic with validation:**

```python
from pydantic import BaseModel, Field, ConfigDict
from typing import Literal

class GitHubConfig(BaseModel):
    token: str = Field(..., description="GitHub personal access token")
    max_prs_per_day: int = Field(default=15, ge=1, le=100)
    rate_limit_margin: int = Field(default=100, ge=0)

class LLMConfig(BaseModel):
    provider: Literal["gemini", "openai", "anthropic", "ollama", "vertex"]
    model: str
    api_key: str
    temperature: float = Field(default=0.5, ge=0.0, le=2.0)
    max_tokens: int = Field(default=2000, ge=100, le=10000)

class Config(BaseModel):
    github: GitHubConfig
    llm: LLMConfig
    discovery: DiscoveryConfig
    analysis: AnalysisConfig

    model_config = ConfigDict(
        validate_assignment=True,
        json_schema_extra={
            "example": {
                "github": {"token": "ghp_...", "max_prs_per_day": 15},
                "llm": {"provider": "gemini", "model": "gemini-2.5-flash"},
            }
        }
    )
```

### Loading Config

```python
# Load from YAML
config = Config.from_yaml("config.yaml")

# Load from environment
config = Config.from_env()

# Load with overrides
config = Config.from_yaml("config.yaml")
config.github.max_prs_per_day = 20  # Override
```

---

## Documentation Standards

### Docstring Format (Google-style)

```python
def process_repo(
    self,
    repo: Repository,
    dry_run: bool = False,
    timeout: int = 300
) -> PipelineResult:
    """Process a repository through the full pipeline.

    Analyzes repo for issues, generates fixes, and creates PRs.
    Respects all safety limits and middleware constraints.

    Args:
        repo: Repository to process.
        dry_run: If True, preview changes without creating PRs.
        timeout: Max seconds to wait for LLM responses.

    Returns:
        PipelineResult with all PRs created and outcomes.

    Raises:
        ConfigError: If config is invalid.
        GitHubError: If GitHub API calls fail.
        LLMError: If LLM provider is unavailable.

    Example:
        >>> repo = Repository(owner="test", name="repo")
        >>> result = await pipeline.process_repo(repo)
        >>> print(f"Created {len(result.prs)} PRs")
        Created 2 PRs
    """
```

### README in Every Module

**Create a brief README in each major module:**

```markdown
# Analysis Module

Detects code issues in repositories using multiple strategies.

## Entry Points

- `CodeAnalyzer.analyze()` — Run all analyzers
- `SkillLoader.load()` — Load language-specific skills

## Key Classes

- `CodeAnalyzer` — Multi-strategy analysis orchestrator
- `SecurityStrategy` — Detects security issues
- `CodeQualityStrategy` — Detects code quality issues

## Example

```python
analyzer = CodeAnalyzer(llm_provider=llm)
findings = await analyzer.analyze(repo)
```
```

---

## Performance Guidelines

### Async Concurrency

- **Max concurrent repos:** 3 (via `Semaphore`)
- **Max concurrent API calls:** 5 per provider
- **Timeout defaults:** 30s (GitHub), 60s (LLM)

### Token Budgeting

- **Per-analysis budget:** 30,000 tokens
- **Soft limit:** Compress context at 25,000
- **Hard limit:** Stop processing at 30,000

### Database

- **Batch inserts:** 100+ records per transaction
- **Indexes:** On `repo_id`, `pr_number`, `timestamp`
- **Cleanup:** Archive old records (> 90 days) monthly

---

## Security Standards

### Secrets

- **Never log** API keys, tokens, or credentials
- **Use env vars** `CONTRIBAI_GITHUB_TOKEN`, `CONTRIBAI_LLM_API_KEY`
- **Validate inputs** from external sources
- **Sanitize LLM output** before code execution

### Dependencies

- **Audit:** `pip-audit` in CI
- **Auto-update:** Dependabot in GitHub
- **Lock versions:** `pip freeze > requirements.lock`

### Access Control

- **GitHub:** Use least-privilege token (only `repo`, `workflow`)
- **Web API:** Require API key (SHA256 hash, no plaintext)
- **Webhooks:** Validate HMAC signature

---

## Document Metadata

- **Created:** 2026-03-28
- **Last Updated:** 2026-03-28
- **Owner:** Technical Lead / Code Reviewer
- **References:** README.md, CONTRIBUTING.md, .github/workflows/
