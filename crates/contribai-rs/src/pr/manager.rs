//! Pull Request lifecycle manager.
//!
//! Port from Python `pr/manager.py`.
//! Handles: fork → branch → commit → PR → compliance.

use regex::Regex;
use tracing::info;

use crate::core::error::Result;
use crate::core::models::{
    Contribution, ContributionType, PrResult, PrStatus, Repository,
};
use crate::github::client::GitHubClient;

/// Manage the full pull request lifecycle.
pub struct PrManager<'a> {
    github: &'a GitHubClient,
    user: Option<serde_json::Value>,
}

impl<'a> PrManager<'a> {
    pub fn new(github: &'a GitHubClient) -> Self {
        Self {
            github,
            user: None,
        }
    }

    /// Get and cache the authenticated user.
    async fn get_user(&mut self) -> Result<&serde_json::Value> {
        if self.user.is_none() {
            let user = self.github.get_authenticated_user().await?;
            self.user = Some(user);
        }
        Ok(self.user.as_ref().unwrap())
    }

    /// Build DCO Signed-off-by string.
    fn build_signoff(user: &serde_json::Value) -> Option<String> {
        let name = user["name"]
            .as_str()
            .or(user["login"].as_str())
            .unwrap_or("");
        let email = user["email"].as_str().map(String::from).unwrap_or_else(|| {
            let uid = user["id"].as_i64().unwrap_or(0);
            let login = user["login"].as_str().unwrap_or("");
            format!("{}+{}@users.noreply.github.com", uid, login)
        });

        if name.is_empty() {
            None
        } else {
            Some(format!("{} <{}>", name, email))
        }
    }

    /// Create a PR from a generated contribution.
    ///
    /// Full workflow: fork → branch → commit → PR
    pub async fn create_pr(
        &mut self,
        contribution: &Contribution,
        target_repo: &Repository,
    ) -> Result<PrResult> {
        let user = self.get_user().await?;
        let username = user["login"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let signoff = Self::build_signoff(user);

        // 1. Fork
        let fork = self.fork_if_needed(&username, target_repo).await?;

        // 2. Create branch
        let branch = if contribution.branch_name.is_empty() {
            Self::human_branch_name(contribution)
        } else {
            contribution.branch_name.clone()
        };

        self.github
            .create_branch(&fork.owner, &fork.name, &branch, None)
            .await?;

        // 3. Commit all file changes
        for change in contribution.changes.iter().chain(contribution.tests_added.iter()) {
            let sha = if !change.is_new_file {
                self.github
                    .get_file_sha(&fork.owner, &fork.name, &change.path, Some(&branch))
                    .await
                    .ok()
            } else {
                None
            };

            self.github
                .create_or_update_file(
                    &fork.owner,
                    &fork.name,
                    &change.path,
                    &change.new_content,
                    &contribution.commit_message,
                    &branch,
                    sha.as_deref(),
                    signoff.as_deref(),
                )
                .await?;
        }

        // 4. Create PR
        let pr_body = self.generate_pr_body(contribution);
        let head = format!("{}:{}", fork.owner, branch);

        let pr_data = self
            .github
            .create_pull_request(
                &target_repo.owner,
                &target_repo.name,
                &contribution.title,
                &pr_body,
                &head,
                Some(target_repo.default_branch.as_str()),
            )
            .await?;

        let pr_number = pr_data["number"].as_i64().unwrap_or(0);
        let pr_url = pr_data["html_url"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let result = PrResult {
            repo: target_repo.clone(),
            contribution: contribution.clone(),
            pr_number,
            pr_url: pr_url.clone(),
            status: PrStatus::Open,
            created_at: chrono::Utc::now(),
            branch_name: branch,
            fork_full_name: fork.full_name,
        };

        info!(pr = pr_number, url = %pr_url, "✅ PR created");
        Ok(result)
    }

    /// Fork if not already forked.
    async fn fork_if_needed(
        &self,
        username: &str,
        repo: &Repository,
    ) -> Result<Repository> {
        // Check if fork exists
        match self.github.get_repo_details(username, &repo.name).await {
            Ok(existing) if existing.owner == username => {
                info!(fork = %existing.full_name, "Fork already exists");
                return Ok(existing);
            }
            _ => {}
        }

        // Create fork
        self.github
            .fork_repository(&repo.owner, &repo.name)
            .await
    }

    /// Generate a natural-looking branch name.
    fn human_branch_name(contribution: &Contribution) -> String {
        let prefix = match contribution.contribution_type {
            ContributionType::SecurityFix => "fix/security",
            ContributionType::CodeQuality => "fix",
            ContributionType::DocsImprove => "docs",
            ContributionType::UiUxFix => "fix/ui",
            ContributionType::PerformanceOpt => "perf",
            ContributionType::FeatureAdd => "feat",
            ContributionType::Refactor => "refactor",
        };

        let re = Regex::new(r"[^a-z0-9]+").unwrap();
        let lower = contribution.finding.title.to_lowercase();
        let slug = re.replace_all(&lower, "-");
        let slug = slug.trim_matches('-');
        let slug = if slug.len() > 50 { &slug[..50] } else { slug };

        format!("{}/{}", prefix, slug)
    }

    /// Generate PR description.
    fn generate_pr_body(&self, contribution: &Contribution) -> String {
        let finding = &contribution.finding;

        let files_list: String = contribution
            .changes
            .iter()
            .map(|c| {
                let tag = if c.is_new_file { "(new)" } else { "(modified)" };
                format!("- `{}` {}", c.path, tag)
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "## Problem\n\n\
             {}\n\n\
             **Severity**: `{}`\n\
             **File**: `{}`\n\n\
             ## Solution\n\n\
             {}\n\n\
             ## Changes\n\n\
             {}\n\n\
             ## Testing\n\n\
             - [ ] Existing tests pass\n\
             - [ ] Manual review completed\n\
             - [ ] No new warnings/errors introduced\n",
            finding.description,
            finding.severity,
            finding.file_path,
            finding
                .suggestion
                .as_deref()
                .unwrap_or(&contribution.description),
            files_list
        )
    }

    /// Check PR status.
    pub async fn get_pr_status(
        &self,
        owner: &str,
        repo: &str,
        pr_number: i64,
    ) -> Result<PrStatus> {
        let data = self.github.get_pr_details(owner, repo, pr_number).await?;

        let state = data["state"].as_str().unwrap_or("open");
        let merged = data["merged"].as_bool().unwrap_or(false);

        Ok(if merged {
            PrStatus::Merged
        } else if state == "closed" {
            PrStatus::Closed
        } else if data["requested_reviewers"].as_array().map_or(false, |r| !r.is_empty()) {
            PrStatus::ReviewRequested
        } else {
            PrStatus::Open
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{FileChange, Finding, Severity};
    use chrono::Utc;

    fn test_contribution() -> Contribution {
        Contribution {
            finding: Finding {
                id: "1".into(),
                finding_type: ContributionType::SecurityFix,
                severity: Severity::High,
                title: "SQL injection vulnerability".into(),
                description: "Unsafe query construction".into(),
                file_path: "src/db.py".into(),
                line_start: Some(10),
                line_end: Some(15),
                suggestion: Some("Use parameterized queries".into()),
                confidence: 0.9,
                priority_signals: vec![],
            },
            contribution_type: ContributionType::SecurityFix,
            title: "fix: sql injection".into(),
            description: "Fixed unsafe query".into(),
            changes: vec![FileChange {
                path: "src/db.py".into(),
                original_content: None,
                new_content: "fixed code".into(),
                is_new_file: false,
                is_deleted: false,
            }],
            commit_message: "fix: sanitize sql queries".into(),
            tests_added: vec![],
            branch_name: String::new(),
            generated_at: Utc::now(),
        }
    }

    #[test]
    fn test_human_branch_name() {
        let c = test_contribution();
        let branch = PrManager::human_branch_name(&c);
        assert!(branch.starts_with("fix/security/"));
        assert!(branch.contains("sql-injection"));
    }

    #[test]
    fn test_build_signoff() {
        let user = serde_json::json!({
            "login": "testuser",
            "name": "Test User",
            "email": "test@example.com",
            "id": 12345
        });
        let signoff = PrManager::build_signoff(&user);
        assert_eq!(signoff, Some("Test User <test@example.com>".to_string()));
    }

    #[test]
    fn test_build_signoff_noreply() {
        let user = serde_json::json!({
            "login": "testuser",
            "name": "Test User",
            "id": 12345
        });
        let signoff = PrManager::build_signoff(&user);
        assert!(signoff.unwrap().contains("noreply.github.com"));
    }
}
