"""Main code analysis orchestrator.

Runs multiple analyzers (security, code quality, docs, UI/UX) in parallel
using LLM-powered analysis. Each analyzer examines the repo through
a different lens and returns findings.
"""

from __future__ import annotations

import asyncio
import contextlib
import logging
import time
import uuid
from fnmatch import fnmatch

from contribai.core.config import AnalysisConfig
from contribai.core.models import (
    AnalysisResult,
    ContributionType,
    FileNode,
    Finding,
    RepoContext,
    Repository,
    Severity,
)
from contribai.github.client import GitHubClient
from contribai.llm.provider import LLMProvider

logger = logging.getLogger(__name__)

# File extensions we can meaningfully analyze
ANALYZABLE_EXTENSIONS = {
    ".py",
    ".js",
    ".ts",
    ".jsx",
    ".tsx",
    ".java",
    ".go",
    ".rs",
    ".rb",
    ".php",
    ".c",
    ".cpp",
    ".h",
    ".hpp",
    ".cs",
    ".swift",
    ".kt",
    ".html",
    ".css",
    ".scss",
    ".vue",
    ".svelte",
    ".md",
    ".rst",
    ".txt",
    ".yaml",
    ".yml",
    ".json",
    ".toml",
}


class CodeAnalyzer:
    """Orchestrates multiple code analyzers using LLM."""

    def __init__(
        self,
        llm: LLMProvider,
        github: GitHubClient,
        config: AnalysisConfig,
    ):
        self._llm = llm
        self._github = github
        self._config = config

    async def analyze(self, repo: Repository) -> AnalysisResult:
        """Run full analysis on a repository.

        1. Fetch file tree
        2. Select files to analyze
        3. Run enabled analyzers in parallel
        4. Aggregate and deduplicate findings
        """
        start = time.monotonic()

        # Fetch file tree
        file_tree = await self._github.get_file_tree(repo.owner, repo.name)
        analyzable = self._select_files(file_tree)

        logger.info(
            "Analyzing %s: %d/%d files selected",
            repo.full_name,
            len(analyzable),
            len(file_tree),
        )

        # Build repo context
        context = await self._build_context(repo, file_tree, analyzable)

        # Run enabled analyzers
        all_findings: list[Finding] = []
        analyzer_tasks = []

        for analyzer_name in self._config.enabled_analyzers:
            analyzer_tasks.append(self._run_analyzer(analyzer_name, context))

        results = await asyncio.gather(*analyzer_tasks, return_exceptions=True)
        for result in results:
            if isinstance(result, Exception):
                logger.error("Analyzer failed: %s", result)
            elif isinstance(result, list):
                all_findings.extend(result)

        # Deduplicate
        findings = self._deduplicate(all_findings)

        # Filter by severity threshold
        findings = self._filter_severity(findings)

        duration = time.monotonic() - start
        return AnalysisResult(
            repo=repo,
            findings=findings,
            analyzed_files=len(analyzable),
            skipped_files=len(file_tree) - len(analyzable),
            analysis_duration_sec=round(duration, 2),
        )

    def _select_files(self, tree: list[FileNode]) -> list[FileNode]:
        """Select files suitable for analysis."""
        selected: list[FileNode] = []
        for node in tree:
            if node.type != "blob":
                continue

            # Check extension
            ext = "." + node.path.rsplit(".", 1)[-1] if "." in node.path else ""
            if ext.lower() not in ANALYZABLE_EXTENSIONS:
                continue

            # Check skip patterns
            if any(fnmatch(node.path, pat) for pat in self._config.skip_patterns):
                continue

            # Check file size
            if node.size > self._config.max_file_size_kb * 1024:
                continue

            selected.append(node)

        return selected

    async def _build_context(
        self,
        repo: Repository,
        tree: list[FileNode],
        analyzable: list[FileNode],
    ) -> RepoContext:
        """Build repository context for LLM analysis."""
        # Fetch key files
        readme = None
        contributing = None
        relevant_files: dict[str, str] = {}

        with contextlib.suppress(Exception):
            readme = await self._github.get_file_content(repo.owner, repo.name, "README.md")

        with contextlib.suppress(Exception):
            contributing = await self._github.get_contributing_guide(repo.owner, repo.name)

        # Fetch a sample of source files (up to 15 most important)
        priority_files = self._prioritize_files(analyzable)[:15]
        for node in priority_files:
            try:
                content = await self._github.get_file_content(repo.owner, repo.name, node.path)
                relevant_files[node.path] = content
            except Exception as e:
                logger.debug("Failed to fetch %s: %s", node.path, e)

        return RepoContext(
            repo=repo,
            file_tree=tree,
            readme_content=readme,
            contributing_guide=contributing,
            relevant_files=relevant_files,
        )

    def _prioritize_files(self, files: list[FileNode]) -> list[FileNode]:
        """Prioritize files for analysis (entry points, configs, core modules first)."""
        priority_patterns = [
            "main.py",
            "app.py",
            "index.ts",
            "index.js",
            "server.py",
            "setup.py",
            "pyproject.toml",
            "package.json",
            "config",
            "settings",
            "auth",
            "security",
        ]

        def file_priority(node: FileNode) -> int:
            name = node.path.lower()
            for i, pattern in enumerate(priority_patterns):
                if pattern in name:
                    return i
            return len(priority_patterns) + 1

        return sorted(files, key=file_priority)

    async def _run_analyzer(self, name: str, context: RepoContext) -> list[Finding]:
        """Run a single LLM-powered analyzer."""
        prompts = {
            "security": self._security_prompt,
            "code_quality": self._code_quality_prompt,
            "docs": self._docs_prompt,
            "ui_ux": self._ui_ux_prompt,
            "performance": self._performance_prompt,
            "refactor": self._refactor_prompt,
            "testing": self._testing_prompt,
        }

        prompt_fn = prompts.get(name)
        if not prompt_fn:
            logger.warning("Unknown analyzer: %s", name)
            return []

        prompt = prompt_fn(context)
        system = (
            "You are an expert code analyst. Analyze the given repository and return findings "
            "in a structured format. For each finding, provide:\n"
            "- title: short descriptive title\n"
            "- severity: low|medium|high|critical\n"
            "- file_path: path to the affected file\n"
            "- line_start: approximate line number (or 0 if unknown)\n"
            "- description: detailed explanation of the issue\n"
            "- suggestion: how to fix it\n\n"
            "Return findings as a YAML list. If no issues found, return 'findings: []'.\n"
            "Be specific and actionable. Avoid false positives."
        )

        try:
            response = await self._llm.complete(prompt, system=system, temperature=0.2)
            return self._parse_findings(response, name, context)
        except Exception as e:
            logger.error("Analyzer %s failed: %s", name, e)
            return []

    def _security_prompt(self, ctx: RepoContext) -> str:
        files_text = self._format_files(ctx)
        return (
            f"Analyze this {ctx.repo.language} repository for SECURITY vulnerabilities:\n\n"
            f"Repository: {ctx.repo.full_name}\n\n"
            f"{files_text}\n\n"
            "Look for:\n"
            "1. Hardcoded secrets/credentials/API keys\n"
            "2. SQL injection vulnerabilities\n"
            "3. XSS (Cross-Site Scripting) risks\n"
            "4. Insecure deserialization\n"
            "5. Path traversal vulnerabilities\n"
            "6. Missing input validation\n"
            "7. Insecure cryptography\n"
            "8. Exposed sensitive data in logs\n"
            "9. Missing authentication/authorization checks\n"
            "10. CSRF vulnerabilities\n"
        )

    def _code_quality_prompt(self, ctx: RepoContext) -> str:
        files_text = self._format_files(ctx)
        return (
            f"Analyze this {ctx.repo.language} repository for CODE QUALITY issues:\n\n"
            f"Repository: {ctx.repo.full_name}\n\n"
            f"{files_text}\n\n"
            "Look for:\n"
            "1. Missing error handling (bare excepts, unhandled errors)\n"
            "2. Code duplication\n"
            "3. Overly complex functions (high cyclomatic complexity)\n"
            "4. Missing type hints/annotations\n"
            "5. Dead/unreachable code\n"
            "6. Performance anti-patterns (N+1 queries, unnecessary loops)\n"
            "7. Missing resource cleanup (file handles, connections)\n"
            "8. Inconsistent naming conventions\n"
            "9. Magic numbers/strings\n"
            "10. Missing or inadequate logging\n"
        )

    def _docs_prompt(self, ctx: RepoContext) -> str:
        readme = ctx.readme_content or "No README found"
        files_text = self._format_files(ctx)
        return (
            f"Analyze this {ctx.repo.language} repository for DOCUMENTATION gaps:\n\n"
            f"Repository: {ctx.repo.full_name}\n\n"
            f"README:\n{readme[:2000]}\n\n"
            f"{files_text}\n\n"
            "Look for:\n"
            "1. Missing or incomplete README sections (install, usage, API docs)\n"
            "2. Undocumented public functions/classes/modules\n"
            "3. Outdated or incorrect code examples\n"
            "4. Missing docstrings\n"
            "5. Missing CHANGELOG entries\n"
            "6. Missing or incomplete API documentation\n"
            "7. Broken links in documentation\n"
            "8. Missing contributing guidelines\n"
        )

    def _ui_ux_prompt(self, ctx: RepoContext) -> str:
        files_text = self._format_files(ctx)
        return (
            f"Analyze this repository for UI/UX issues:\n\n"
            f"Repository: {ctx.repo.full_name} ({ctx.repo.language})\n\n"
            f"{files_text}\n\n"
            "Look for:\n"
            "1. Accessibility (a11y) issues (missing ARIA labels, alt text)\n"
            "2. Missing loading/skeleton states\n"
            "3. Missing error boundaries/states\n"
            "4. Responsiveness issues\n"
            "5. Color contrast problems\n"
            "6. Missing keyboard navigation\n"
            "7. Missing form validation feedback\n"
            "8. Poor empty states\n"
            "NOTE: Only analyze if the repo contains frontend code (HTML/CSS/JS/React/Vue/etc). "
            "If no frontend code found, return 'findings: []'.\n"
        )

    def _performance_prompt(self, ctx: RepoContext) -> str:
        files_text = self._format_files(ctx)
        return (
            f"Analyze this {ctx.repo.language} repository for PERFORMANCE issues:\n\n"
            f"Repository: {ctx.repo.full_name}\n\n"
            f"{files_text}\n\n"
            "Look for:\n"
            "1. N+1 query patterns (database calls inside loops)\n"
            "2. Missing caching for expensive operations\n"
            "3. Unnecessary synchronous I/O blocking the event loop\n"
            "4. Inefficient data structures (list for lookups instead of set/dict)\n"
            "5. Unnecessary re-renders in React/Vue components\n"
            "6. Missing pagination for large data fetches\n"
            "7. Unoptimized regex or string operations in hot paths\n"
            "8. Memory leaks (unclosed resources, growing caches without eviction)\n"
            "9. Redundant API calls or duplicate network requests\n"
            "10. Missing lazy loading for heavy imports or large assets\n\n"
            "Focus on issues that have measurable performance impact. "
            "Avoid micro-optimizations that don't matter in practice.\n"
        )

    def _refactor_prompt(self, ctx: RepoContext) -> str:
        files_text = self._format_files(ctx)
        return (
            f"Analyze this {ctx.repo.language} repository for REFACTORING opportunities:\n\n"
            f"Repository: {ctx.repo.full_name}\n\n"
            f"{files_text}\n\n"
            "Look for:\n"
            "1. Functions/methods that are too long (>50 lines) and should be split\n"
            "2. DRY violations (duplicated logic across multiple files)\n"
            "3. God classes/modules that do too many things\n"
            "4. Deeply nested conditionals (>3 levels) that need extraction\n"
            "5. Inappropriate use of inheritance vs composition\n"
            "6. Magic numbers/strings that should be named constants\n"
            "7. Complex boolean expressions that need helper methods\n"
            "8. Mixed abstraction levels within a single function\n"
            "9. Feature envy (methods that use other class's data more than their own)\n"
            "10. Dead code or unused imports/variables\n\n"
            "Focus on refactorings that improve readability and maintainability. "
            "Each finding should be a single, self-contained refactoring.\n"
        )

    def _testing_prompt(self, ctx: RepoContext) -> str:
        files_text = self._format_files(ctx)
        return (
            f"Analyze this {ctx.repo.language} repository for TESTING gaps:\n\n"
            f"Repository: {ctx.repo.full_name}\n\n"
            f"{files_text}\n\n"
            "Look for:\n"
            "1. Public functions/methods with NO unit tests\n"
            "2. Critical business logic without test coverage\n"
            "3. Edge cases not covered by existing tests\n"
            "4. Error handling paths without tests\n"
            "5. Missing integration tests for API endpoints\n"
            "6. Untested configuration validation\n"
            "7. Missing tests for data transformations/serialization\n"
            "8. Race conditions or concurrency that needs testing\n\n"
            "For each finding, suggest a specific test that should be written. "
            "Focus on the most impactful missing tests — those covering critical "
            "code paths or frequently modified code.\n"
            "NOTE: If the repo has no test directory/framework at all, suggest "
            "setting up a test framework as one finding and specific tests as others.\n"
        )

    def _format_files(self, ctx: RepoContext) -> str:
        """Format relevant files for the prompt."""
        parts = []
        for path, content in ctx.relevant_files.items():
            truncated = content[:3000] if len(content) > 3000 else content
            parts.append(f"### {path}\n```\n{truncated}\n```")
        return "\n\n".join(parts) if parts else "No source files available."

    def _parse_findings(self, response: str, analyzer_name: str, ctx: RepoContext) -> list[Finding]:
        """Parse LLM response into Finding objects."""
        import yaml

        findings: list[Finding] = []

        type_map = {
            "security": ContributionType.SECURITY_FIX,
            "code_quality": ContributionType.CODE_QUALITY,
            "docs": ContributionType.DOCS_IMPROVE,
            "ui_ux": ContributionType.UI_UX_FIX,
            "performance": ContributionType.PERFORMANCE_OPT,
            "refactor": ContributionType.REFACTOR,
            "testing": ContributionType.CODE_QUALITY,
        }
        contrib_type = type_map.get(analyzer_name, ContributionType.CODE_QUALITY)

        try:
            # Try to extract YAML from the response
            yaml_text = response
            if "```yaml" in response:
                yaml_text = response.split("```yaml")[1].split("```")[0]
            elif "```" in response:
                yaml_text = response.split("```")[1].split("```")[0]

            parsed = yaml.safe_load(yaml_text)
            if not parsed:
                return []

            items = parsed if isinstance(parsed, list) else parsed.get("findings", [])

            for item in items:
                if not isinstance(item, dict):
                    continue

                severity_str = str(item.get("severity", "medium")).lower()
                try:
                    severity = Severity(severity_str)
                except ValueError:
                    severity = Severity.MEDIUM

                findings.append(
                    Finding(
                        id=str(uuid.uuid4())[:8],
                        type=contrib_type,
                        severity=severity,
                        title=str(item.get("title", "Untitled finding")),
                        description=str(item.get("description", "")),
                        file_path=str(item.get("file_path", "")),
                        line_start=item.get("line_start"),
                        line_end=item.get("line_end"),
                        suggestion=item.get("suggestion"),
                    )
                )
        except Exception as e:
            logger.warning("Failed to parse %s findings: %s", analyzer_name, e)

        logger.info("Analyzer %s found %d issues", analyzer_name, len(findings))
        return findings

    def _deduplicate(self, findings: list[Finding]) -> list[Finding]:
        """Remove duplicate findings."""
        seen: set[str] = set()
        unique: list[Finding] = []
        for f in findings:
            key = f"{f.file_path}:{f.title}:{f.severity}"
            if key not in seen:
                seen.add(key)
                unique.append(f)
        return unique

    def _filter_severity(self, findings: list[Finding]) -> list[Finding]:
        """Filter findings by minimum severity threshold."""
        order = [Severity.LOW, Severity.MEDIUM, Severity.HIGH, Severity.CRITICAL]
        try:
            threshold = Severity(self._config.severity_threshold)
        except ValueError:
            threshold = Severity.MEDIUM
        min_idx = order.index(threshold)
        return [f for f in findings if order.index(f.severity) >= min_idx]
