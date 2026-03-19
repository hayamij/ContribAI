"""Pull Request lifecycle manager.

Handles the full PR workflow: fork → branch → commit → PR.
Generates detailed PR descriptions with context and testing info.
"""

from __future__ import annotations

import logging

from contribai.core.exceptions import PRCreationError
from contribai.core.models import Contribution, ContributionType, PRResult, PRStatus, Repository
from contribai.github.client import GitHubClient

logger = logging.getLogger(__name__)


class PRManager:
    """Manage the full pull request lifecycle."""

    def __init__(self, github: GitHubClient):
        self._github = github
        self._user: dict | None = None

    async def _get_user(self) -> dict:
        """Get and cache the authenticated user."""
        if not self._user:
            self._user = await self._github.get_authenticated_user()
        return self._user

    async def create_pr(
        self,
        contribution: Contribution,
        target_repo: Repository,
        *,
        guidelines=None,
    ) -> PRResult:
        """Create a PR from a generated contribution.

        Full workflow:
        1. Fork the target repo (if not already forked)
        2. Create a feature branch on the fork
        3. Commit all file changes
        3b. Create a linked issue (if repo requires it)
        4. Create the pull request
        5. Check compliance and auto-fix if needed
        """
        user = await self._get_user()
        username = user["login"]

        try:
            # 1. Fork
            fork = await self._fork_if_needed(username, target_repo)
            fork_owner = fork.owner
            fork_name = fork.name

            # 2. Create branch
            branch = contribution.branch_name or f"contribai/fix-{contribution.finding.id}"
            await self._github.create_branch(fork_owner, fork_name, branch)

            # 3. Commit changes
            for change in contribution.changes + contribution.tests_added:
                # Get existing file SHA for updates
                sha = None
                if not change.is_new_file:
                    try:
                        await self._github.get_file_content(fork_owner, fork_name, change.path)
                        # We need the SHA to update - fetch via contents API

                        resp = await self._github._get(
                            f"/repos/{fork_owner}/{fork_name}/contents/{change.path}",
                            params={"ref": branch},
                        )
                        sha = resp.get("sha")
                    except Exception:
                        pass  # New file or couldn't get SHA

                await self._github.create_or_update_file(
                    fork_owner,
                    fork_name,
                    change.path,
                    change.new_content,
                    contribution.commit_message,
                    branch,
                    sha=sha,
                )

            # 3b. Create linked issue if repo likely requires it
            issue_number = None
            if guidelines and guidelines.has_guidelines:
                issue_number = await self._create_issue_for_finding(contribution, target_repo)

            # 4. Create PR
            if guidelines and guidelines.has_guidelines:
                from contribai.github.guidelines import adapt_pr_body

                pr_body = adapt_pr_body(contribution, guidelines)
            else:
                pr_body = self._generate_pr_body(contribution)

            # Inject issue link into body
            if issue_number:
                pr_body = pr_body.replace("Closes N/A", f"Closes #{issue_number}").replace(
                    "Closes #\n", f"Closes #{issue_number}\n"
                )
                # If no placeholder found, prepend
                if f"#{issue_number}" not in pr_body:
                    pr_body = f"Closes #{issue_number}\n\n{pr_body}"

            head = f"{fork_owner}:{branch}"

            pr_data = await self._github.create_pull_request(
                target_repo.owner,
                target_repo.name,
                title=contribution.title,
                body=pr_body,
                head=head,
                base=target_repo.default_branch,
            )

            result = PRResult(
                repo=target_repo,
                contribution=contribution,
                pr_number=pr_data["number"],
                pr_url=pr_data["html_url"],
                status=PRStatus.OPEN,
                branch_name=branch,
                fork_full_name=f"{fork_owner}/{fork_name}",
            )

            logger.info("✅ PR #%d created: %s", result.pr_number, result.pr_url)
            return result

        except Exception as e:
            raise PRCreationError(f"Failed to create PR: {e}") from e

    async def _fork_if_needed(self, username: str, repo: Repository) -> Repository:
        """Fork the repo if not already forked."""
        try:
            # Check if fork exists
            existing = await self._github.get_repo_details(username, repo.name)
            if existing.owner == username:
                logger.info("Fork already exists: %s/%s", username, repo.name)
                return existing
        except Exception:
            pass

        # Create fork
        return await self._github.fork_repository(repo.owner, repo.name)

    def _generate_pr_body(self, contribution: Contribution) -> str:
        """Generate a detailed PR description."""
        finding = contribution.finding

        # Type-specific emoji and label
        type_info = {
            ContributionType.SECURITY_FIX: ("🔒", "Security Fix"),
            ContributionType.CODE_QUALITY: ("✨", "Code Quality"),
            ContributionType.DOCS_IMPROVE: ("📝", "Documentation"),
            ContributionType.UI_UX_FIX: ("🎨", "UI/UX Improvement"),
            ContributionType.PERFORMANCE_OPT: ("⚡", "Performance"),
            ContributionType.FEATURE_ADD: ("🚀", "New Feature"),
            ContributionType.REFACTOR: ("♻️", "Refactoring"),
        }
        emoji, label = type_info.get(finding.type, ("🔧", "Fix"))

        # Files changed summary
        files_list = "\n".join(
            f"- `{c.path}` {'(new)' if c.is_new_file else '(modified)'}"
            for c in contribution.changes
        )

        body = f"""## {emoji} {label}

### Problem
{finding.description}

**Severity**: `{finding.severity.value}`
**File**: `{finding.file_path}`

### Solution
{finding.suggestion or contribution.description}

### Changes
{files_list}

### Testing
- [ ] Existing tests pass
- [ ] Manual review completed
- [ ] No new warnings/errors introduced

---

<details>
<summary>🤖 About this PR</summary>

This pull request was generated by [ContribAI](https://github.com/tang-vu/ContribAI), an AI agent
that helps improve open source projects. The change was:

1. **Discovered** by automated code analysis
2. **Generated** by AI with context-aware code generation
3. **Self-reviewed** by AI quality checks

If you have questions or feedback about this PR, please comment below.
We appreciate your time reviewing this contribution!

</details>
"""
        return body

    async def get_pr_status(self, owner: str, repo: str, pr_number: int) -> PRStatus:
        """Check the current status of a PR."""
        try:
            data = await self._github._get(f"/repos/{owner}/{repo}/pulls/{pr_number}")
            state = data.get("state", "open")
            merged = data.get("merged", False)

            if merged:
                return PRStatus.MERGED
            elif state == "closed":
                return PRStatus.CLOSED
            elif data.get("requested_reviewers"):
                return PRStatus.REVIEW_REQUESTED
            else:
                return PRStatus.OPEN
        except Exception:
            return PRStatus.PENDING

    # ── Auto Issue Creation ─────────────────────────────────────────────

    async def _create_issue_for_finding(
        self,
        contribution: Contribution,
        target_repo: Repository,
    ) -> int | None:
        """Create an issue describing the finding before creating a PR.

        Returns the issue number, or None if creation failed.
        """
        finding = contribution.finding

        # Map contribution type to issue label
        type_labels = {
            ContributionType.SECURITY_FIX: "bug",
            ContributionType.CODE_QUALITY: "bug",
            ContributionType.DOCS_IMPROVE: "documentation",
            ContributionType.UI_UX_FIX: "bug",
            ContributionType.PERFORMANCE_OPT: "perf",
            ContributionType.FEATURE_ADD: "enhancement",
            ContributionType.REFACTOR: "enhancement",
        }

        # Use conventional commit style title for issue
        type_map = {
            ContributionType.SECURITY_FIX: "fix",
            ContributionType.CODE_QUALITY: "fix",
            ContributionType.DOCS_IMPROVE: "docs",
            ContributionType.UI_UX_FIX: "fix",
            ContributionType.PERFORMANCE_OPT: "perf",
            ContributionType.FEATURE_ADD: "feat",
            ContributionType.REFACTOR: "refactor",
        }

        prefix = type_map.get(finding.type, "fix")

        # Extract scope from file path
        scope = ""
        if finding.file_path:
            parts = finding.file_path.split("/")
            if (len(parts) >= 2 and parts[0] in ("packages", "apps", "libs")) or (
                len(parts) >= 2 and parts[0] == "src"
            ):
                scope = parts[1]

        if scope:
            issue_title = f"{prefix}({scope}): {finding.title.lower()}"
        else:
            issue_title = f"{prefix}: {finding.title.lower()}"

        issue_body = (
            f"## Description\n\n"
            f"{finding.description}\n\n"
            f"**Severity**: `{finding.severity.value}`\n"
            f"**File**: `{finding.file_path}`\n\n"
            f"## Expected Behavior\n\n"
            f"The code should handle this case properly to avoid "
            f"unexpected errors or degraded quality.\n\n"
            f"---\n"
            f"*Found by [ContribAI](https://github.com/tang-vu/ContribAI) "
            f"automated code analysis.*"
        )

        try:
            label = type_labels.get(finding.type, "bug")
            try:
                data = await self._github.create_issue(
                    target_repo.owner,
                    target_repo.name,
                    title=issue_title,
                    body=issue_body,
                    labels=[label],
                )
            except Exception:
                # Labels might not exist, retry without labels
                data = await self._github.create_issue(
                    target_repo.owner,
                    target_repo.name,
                    title=issue_title,
                    body=issue_body,
                )

            issue_number = data["number"]
            logger.info(
                "📋 Created issue #%d on %s: %s",
                issue_number,
                target_repo.full_name,
                issue_title,
            )
            return issue_number

        except Exception as e:
            logger.warning("Failed to create issue: %s", e)
            return None

    # ── Post-PR Compliance ──────────────────────────────────────────────

    async def check_compliance_and_fix(
        self,
        pr_result: PRResult,
        contribution: Contribution,
        guidelines=None,
    ) -> bool:
        """Check bot comments for compliance issues and auto-fix.

        Waits briefly for bots to post, then reads comments and fixes
        any flagged issues (title format, missing issue link, etc.).

        Returns True if PR is compliant (or was auto-fixed).
        """
        import asyncio

        repo = pr_result.repo

        # Wait for bots to comment
        await asyncio.sleep(10)

        try:
            comments = await self._github.get_pr_comments(
                repo.owner, repo.name, pr_result.pr_number
            )
        except Exception as e:
            logger.warning("Could not fetch PR comments: %s", e)
            return True  # Don't block on this

        bot_issues = []
        for comment in comments:
            user = comment.get("user", {})
            body = comment.get("body", "")
            # Detect bot compliance comments
            is_bot = user.get("type") == "Bot" or user.get("login", "").endswith("[bot]")
            if is_bot and any(
                keyword in body.lower()
                for keyword in [
                    "doesn't follow conventional commit",
                    "no issue referenced",
                    "doesn't fully meet",
                    "pr title",
                    "needs:title",
                    "needs:issue",
                    "needs:compliance",
                ]
            ):
                bot_issues.append(body)

        if not bot_issues:
            logger.info("✅ PR #%d passed compliance checks", pr_result.pr_number)
            return True

        logger.info(
            "🔧 PR #%d has %d compliance issues, auto-fixing...",
            pr_result.pr_number,
            len(bot_issues),
        )

        # Detect specific issues and fix
        all_comments = " ".join(bot_issues).lower()
        needs_fix = False

        # Fix title format
        if "conventional commit" in all_comments or "needs:title" in all_comments:
            new_title = contribution.title
            # If title still has emoji format, convert to conventional
            if (
                any(
                    new_title.startswith(prefix)
                    for prefix in ["🔒", "✨", "📝", "🎨", "⚡", "🚀", "♻️", "🔧"]
                )
                and guidelines
                and guidelines.has_guidelines
            ):
                from contribai.github.guidelines import (
                    adapt_pr_title,
                    extract_scope_from_path,
                )

                scope = extract_scope_from_path(contribution.finding.file_path or "", guidelines)
                new_title = adapt_pr_title(
                    contribution.finding.title,
                    contribution.finding.type.value,
                    guidelines,
                    scope=scope,
                )

            try:
                await self._github.update_pull_request(
                    repo.owner, repo.name, pr_result.pr_number, title=new_title
                )
                logger.info("Fixed PR title → %s", new_title)
                needs_fix = True
            except Exception as e:
                logger.warning("Failed to fix title: %s", e)

        # Fix missing issue reference
        if "no issue referenced" in all_comments or "needs:issue" in all_comments:
            # Create issue if we didn't already
            issue_number = await self._create_issue_for_finding(contribution, pr_result.repo)
            if issue_number:
                try:
                    # Get current body and inject issue link
                    pr_data = await self._github._get(
                        f"/repos/{repo.owner}/{repo.name}/pulls/{pr_result.pr_number}"
                    )
                    current_body = pr_data.get("body", "")
                    new_body = current_body.replace("Closes N/A", f"Closes #{issue_number}")
                    if f"#{issue_number}" not in new_body:
                        new_body = f"Closes #{issue_number}\n\n{new_body}"

                    await self._github.update_pull_request(
                        repo.owner,
                        repo.name,
                        pr_result.pr_number,
                        body=new_body,
                    )
                    logger.info("Linked issue #%d to PR", issue_number)
                    needs_fix = True
                except Exception as e:
                    logger.warning("Failed to link issue: %s", e)

        if needs_fix:
            logger.info("🔄 PR #%d compliance auto-fixed", pr_result.pr_number)
        else:
            logger.warning("⚠️ PR #%d has unresolved compliance issues", pr_result.pr_number)

        return needs_fix
