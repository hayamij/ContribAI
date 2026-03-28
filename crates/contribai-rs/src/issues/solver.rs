//! Issue-driven contribution engine.
//!
//! Port from Python `issues/solver.py`.
//! Reads open GitHub issues, classifies them, and generates
//! targeted contributions that solve specific issues.

use regex::Regex;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::core::error::Result;
use crate::core::models::{ContributionType, FileNode, Finding, Issue, RepoContext, Repository, Severity};
use crate::github::client::GitHubClient;
use crate::llm::provider::LlmProvider;

/// Classification categories for GitHub issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueCategory {
    Bug,
    Feature,
    Docs,
    Security,
    Performance,
    UiUx,
    GoodFirstIssue,
    Unsolvable,
}

/// Map label → category.
fn label_to_category(label: &str) -> Option<IssueCategory> {
    match label.to_lowercase().trim() {
        "bug" | "fix" | "defect" => Some(IssueCategory::Bug),
        "feature" | "enhancement" | "feature-request" => Some(IssueCategory::Feature),
        "documentation" | "docs" => Some(IssueCategory::Docs),
        "security" | "vulnerability" => Some(IssueCategory::Security),
        "performance" => Some(IssueCategory::Performance),
        "ui" | "ux" | "accessibility" => Some(IssueCategory::UiUx),
        "good first issue" | "good-first-issue" | "beginner" | "help wanted" => {
            Some(IssueCategory::GoodFirstIssue)
        }
        _ => None,
    }
}

/// Map category → contribution type.
fn category_to_contrib(cat: IssueCategory) -> ContributionType {
    match cat {
        IssueCategory::Bug => ContributionType::CodeQuality,
        IssueCategory::Feature => ContributionType::FeatureAdd,
        IssueCategory::Docs => ContributionType::DocsImprove,
        IssueCategory::Security => ContributionType::SecurityFix,
        IssueCategory::Performance => ContributionType::PerformanceOpt,
        IssueCategory::UiUx => ContributionType::UiUxFix,
        IssueCategory::GoodFirstIssue => ContributionType::CodeQuality,
        IssueCategory::Unsolvable => ContributionType::CodeQuality,
    }
}

const SOLVABLE_LABELS: &[&str] = &[
    "good first issue",
    "good-first-issue",
    "help wanted",
    "help-wanted",
    "beginner",
    "easy",
    "low-hanging-fruit",
    "bug",
    "documentation",
    "docs",
    "enhancement",
    "feature",
];

/// Analyzes and solves GitHub issues using LLM.
pub struct IssueSolver<'a> {
    llm: &'a dyn LlmProvider,
    github: &'a GitHubClient,
}

impl<'a> IssueSolver<'a> {
    pub fn new(llm: &'a dyn LlmProvider, github: &'a GitHubClient) -> Self {
        Self { llm, github }
    }

    /// Classify an issue based on labels and title keywords.
    pub fn classify_issue(&self, issue: &Issue) -> IssueCategory {
        // Check labels first
        for label in &issue.labels {
            if let Some(cat) = label_to_category(label) {
                return cat;
            }
        }

        // Keyword matching on title
        let title = issue.title.to_lowercase();
        let keyword_map: &[(&[&str], IssueCategory)] = &[
            (&["bug", "fix", "error", "crash", "broken", "fail"], IssueCategory::Bug),
            (&["add", "feature", "implement", "support", "new"], IssueCategory::Feature),
            (&["doc", "readme", "typo", "documentation", "example"], IssueCategory::Docs),
            (&["security", "vulnerability", "cve", "xss", "injection"], IssueCategory::Security),
            (&["slow", "performance", "optimize", "speed", "memory"], IssueCategory::Performance),
            (&["ui", "ux", "responsive", "accessibility", "design"], IssueCategory::UiUx),
        ];

        for (keywords, category) in keyword_map {
            if keywords.iter().any(|kw| title.contains(kw)) {
                return *category;
            }
        }

        IssueCategory::Bug // default
    }

    /// Estimate issue complexity (1-5).
    fn estimate_complexity(&self, issue: &Issue) -> u32 {
        let mut score: u32 = 2;

        // Good first issues are simple
        if issue.labels.iter().any(|l| {
            let low = l.to_lowercase();
            low.contains("first") || low.contains("beginner")
        }) {
            return 1;
        }

        let body_len = issue.body.as_ref().map(|b| b.len()).unwrap_or(0);
        if body_len > 2000 {
            score += 1;
        }
        if body_len > 5000 {
            score += 1;
        }

        // Multiple file references = complex
        if let Some(body) = &issue.body {
            let re = Regex::new(r"[\w/]+\.\w{1,4}").unwrap_or_else(|_| Regex::new(".").unwrap());
            if re.find_iter(body).count() > 3 {
                score += 1;
            }
        }

        score.min(5)
    }

    /// Filter issues to only those solvable by the agent.
    pub fn filter_solvable(&self, issues: &[Issue], max_complexity: u32) -> Vec<Issue> {
        issues
            .iter()
            .filter(|issue| {
                let cat = self.classify_issue(issue);
                if cat == IssueCategory::Unsolvable {
                    return false;
                }
                self.estimate_complexity(issue) <= max_complexity
            })
            .cloned()
            .collect()
    }

    /// Convert a GitHub issue into a Finding for the generator.
    pub async fn solve_issue(
        &self,
        issue: &Issue,
        repo: &Repository,
        context: &RepoContext,
    ) -> Option<Finding> {
        let category = self.classify_issue(issue);
        let contrib_type = category_to_contrib(category);

        let file_tree_str: String = context
            .file_tree
            .iter()
            .filter(|f| f.node_type == "blob")
            .take(50)
            .map(|f| format!("  {}", f.path))
            .collect::<Vec<_>>()
            .join("\n");

        let mut relevant_code = String::new();
        for (path, content) in context.relevant_files.iter().take(3) {
            let snippet: String = content.chars().take(2000).collect();
            relevant_code.push_str(&format!("\n### {}\n```\n{}\n```\n", path, snippet));
        }

        let body = issue.body.as_deref().unwrap_or("No description provided.");

        let prompt = format!(
            "Analyze this GitHub issue and determine:\n\
             1. Which file(s) need changes\n\
             2. What changes are needed\n\
             3. The severity of the issue\n\n\
             ## Repository: {} ({})\n\n\
             ## Issue #{}: {}\n{}\n\n\
             ## Labels: {}\n\n\
             ## File Tree:\n{}\n\n\
             {}\n\n\
             Respond in this exact format:\n\
             FILE_PATH: <main file to change>\n\
             SEVERITY: <low|medium|high|critical>\n\
             TITLE: <short descriptive title>\n\
             DESCRIPTION: <what needs to be changed and why>\n\
             SUGGESTION: <specific implementation suggestion>",
            repo.full_name,
            repo.language.as_deref().unwrap_or("unknown"),
            issue.number,
            issue.title,
            body,
            if issue.labels.is_empty() { "none".to_string() } else { issue.labels.join(", ") },
            file_tree_str,
            relevant_code
        );

        let response = match self
            .llm
            .complete(
                &prompt,
                Some("You are a senior developer analyzing GitHub issues. Identify the root cause and suggest a specific fix."),
                Some(0.2),
                None,
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!(issue = issue.number, error = %e, "Failed to analyze issue");
                return None;
            }
        };

        // Parse structured response
        let parsed = Self::parse_structured_response(&response);

        let severity = match parsed.get("SEVERITY").map(|s| s.to_lowercase()).as_deref() {
            Some("low") => Severity::Low,
            Some("high") => Severity::High,
            Some("critical") => Severity::Critical,
            _ => Severity::Medium,
        };

        Some(Finding {
            id: format!("issue-{}", issue.number),
            finding_type: contrib_type,
            severity,
            title: parsed.get("TITLE").cloned().unwrap_or_else(|| issue.title.clone()),
            description: parsed
                .get("DESCRIPTION")
                .cloned()
                .unwrap_or_else(|| body.to_string()),
            file_path: parsed.get("FILE_PATH").cloned().unwrap_or_else(|| "unknown".into()),
            suggestion: parsed.get("SUGGESTION").cloned(),
            confidence: 0.85,
            line_start: None,
            line_end: None,
            priority_signals: vec![],
        })
    }

    /// Deep multi-file issue solving.
    pub async fn solve_issue_deep(
        &self,
        issue: &Issue,
        repo: &Repository,
        context: &RepoContext,
    ) -> Vec<Finding> {
        let category = self.classify_issue(issue);
        let contrib_type = category_to_contrib(category);

        let body = issue.body.as_deref().unwrap_or("No description provided.");

        let file_tree_str = Self::build_file_tree_summary(&context.file_tree);

        let mut relevant_code = String::new();
        for (path, content) in context.relevant_files.iter().take(10) {
            let snippet: String = content.chars().take(3000).collect();
            relevant_code.push_str(&format!("\n### {}\n```\n{}\n```\n", path, snippet));
        }

        let prompt = format!(
            "You are solving a GitHub issue. Analyze the issue carefully and create\n\
             a detailed plan for which file(s) to create or modify.\n\n\
             ## Repository: {} ({})\n\n\
             ## Issue #{}: {}\n{}\n\n\
             ## File Tree:\n{}\n\n\
             ## Relevant Code:\n{}\n\n\
             Respond with one or more blocks in this exact format:\n\n\
             ---FILE---\n\
             PATH: <path to file>\n\
             SEVERITY: <low|medium|high|critical>\n\
             TITLE: <what this change does>\n\
             DESCRIPTION: <detailed description>\n\
             SUGGESTION: <specific implementation details>\n\
             ---END---",
            repo.full_name,
            repo.language.as_deref().unwrap_or("unknown"),
            issue.number,
            issue.title,
            body,
            file_tree_str,
            relevant_code,
        );

        match self
            .llm
            .complete(
                &prompt,
                Some("You are an expert open-source developer solving GitHub issues."),
                Some(0.2),
                None,
            )
            .await
        {
            Ok(response) => {
                let findings = Self::parse_multi_file_response(&response, issue, contrib_type);
                if findings.is_empty() {
                    // fallback to single-file
                    if let Some(f) = self.solve_issue(issue, repo, context).await {
                        return vec![f];
                    }
                }
                info!(
                    issue = issue.number,
                    files = findings.len(),
                    "🧠 Deep solve complete"
                );
                findings
            }
            Err(e) => {
                warn!(issue = issue.number, error = %e, "Deep solve failed");
                if let Some(f) = self.solve_issue(issue, repo, context).await {
                    vec![f]
                } else {
                    vec![]
                }
            }
        }
    }

    fn build_file_tree_summary(tree: &[FileNode]) -> String {
        let mut dirs: HashMap<String, Vec<String>> = HashMap::new();
        for f in tree.iter().filter(|f| f.node_type == "blob").take(200) {
            let (dir, file) = match f.path.rsplit_once('/') {
                Some((d, f)) => (d.to_string(), f.to_string()),
                None => (".".to_string(), f.path.clone()),
            };
            dirs.entry(dir).or_default().push(file);
        }

        let mut keys: Vec<_> = dirs.keys().cloned().collect();
        keys.sort();
        keys.iter()
            .take(30)
            .map(|dir| {
                let files = &dirs[dir];
                let files_str = if files.len() > 8 {
                    format!(
                        "{} (+{} more)",
                        files[..8].join(", "),
                        files.len() - 8
                    )
                } else {
                    files.join(", ")
                };
                format!("  {}/  [{}]", dir, files_str)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn parse_structured_response(response: &str) -> HashMap<String, String> {
        let mut parsed = HashMap::new();
        for line in response.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_uppercase();
                if ["FILE_PATH", "PATH", "SEVERITY", "TITLE", "DESCRIPTION", "SUGGESTION", "ACTION"]
                    .contains(&key.as_str())
                {
                    parsed.insert(key, value.trim().to_string());
                }
            }
        }
        parsed
    }

    fn parse_multi_file_response(
        response: &str,
        issue: &Issue,
        default_type: ContributionType,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();
        let blocks: Vec<&str> = response.split("---FILE---").collect();

        for block in blocks {
            let block = block.trim();
            if block.is_empty() || !block.contains("---END---") {
                continue;
            }

            let block = block.split("---END---").next().unwrap_or("").trim();
            let parsed = Self::parse_structured_response(block);

            let file_path = match parsed.get("PATH") {
                Some(p) if p != "unknown" && !p.is_empty() => p.clone(),
                _ => continue,
            };

            let severity = match parsed.get("SEVERITY").map(|s| s.to_lowercase()).as_deref() {
                Some("low") => Severity::Low,
                Some("high") => Severity::High,
                Some("critical") => Severity::Critical,
                _ => Severity::Medium,
            };

            findings.push(Finding {
                id: format!("issue-{}-{}", issue.number, findings.len()),
                finding_type: default_type.clone(),
                severity,
                title: parsed
                    .get("TITLE")
                    .cloned()
                    .unwrap_or_else(|| issue.title.clone()),
                description: parsed
                    .get("DESCRIPTION")
                    .cloned()
                    .unwrap_or_else(|| issue.body.clone().unwrap_or_default()),
                file_path,
                suggestion: parsed.get("SUGGESTION").cloned(),
                confidence: 0.80,
                line_start: None,
            line_end: None,
            priority_signals: vec![],
            });
        }

        findings.into_iter().take(5).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_issue(title: &str, labels: &[&str]) -> Issue {
        Issue {
            number: 1,
            title: title.to_string(),
            body: Some("test body".into()),
            labels: labels.iter().map(|s| s.to_string()).collect(),
            state: "open".into(),
            created_at: None,
            html_url: String::new(),
        }
    }

    #[test]
    fn test_classify_by_label() {
        let solver = IssueSolver { llm: &MockLlm, github: &unsafe_mock_github() };
        assert_eq!(
            solver.classify_issue(&make_issue("anything", &["bug"])),
            IssueCategory::Bug
        );
        assert_eq!(
            solver.classify_issue(&make_issue("anything", &["documentation"])),
            IssueCategory::Docs
        );
        assert_eq!(
            solver.classify_issue(&make_issue("anything", &["good first issue"])),
            IssueCategory::GoodFirstIssue
        );
    }

    #[test]
    fn test_classify_by_title() {
        let solver = IssueSolver { llm: &MockLlm, github: &unsafe_mock_github() };
        assert_eq!(
            solver.classify_issue(&make_issue("fix crash on startup", &[])),
            IssueCategory::Bug
        );
        assert_eq!(
            solver.classify_issue(&make_issue("add support for JSON", &[])),
            IssueCategory::Feature
        );
        assert_eq!(
            solver.classify_issue(&make_issue("update readme docs", &[])),
            IssueCategory::Docs
        );
    }

    #[test]
    fn test_complexity_good_first() {
        let solver = IssueSolver { llm: &MockLlm, github: &unsafe_mock_github() };
        assert_eq!(
            solver.estimate_complexity(&make_issue("easy fix", &["good first issue"])),
            1
        );
    }

    #[test]
    fn test_filter_solvable() {
        let solver = IssueSolver { llm: &MockLlm, github: &unsafe_mock_github() };
        let issues = vec![
            make_issue("fix bug", &["bug"]),
            make_issue("x".repeat(6000).as_str(), &[]),
        ];
        // Second issue has long body (but it's in title here, body is "test body")
        let filtered = solver.filter_solvable(&issues, 3);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_parse_structured_response() {
        let response = "FILE_PATH: src/main.py\nSEVERITY: high\nTITLE: Fix null check\nDESCRIPTION: Missing null check\nSUGGESTION: Add if not None";
        let parsed = IssueSolver::parse_structured_response(response);
        assert_eq!(parsed.get("FILE_PATH").unwrap(), "src/main.py");
        assert_eq!(parsed.get("SEVERITY").unwrap(), "high");
    }

    #[test]
    fn test_parse_multi_file_response() {
        let response = "---FILE---\nPATH: src/a.py\nSEVERITY: high\nTITLE: Fix A\nDESCRIPTION: desc\nSUGGESTION: fix\n---END---\n---FILE---\nPATH: src/b.py\nSEVERITY: low\nTITLE: Fix B\nDESCRIPTION: desc2\nSUGGESTION: fix2\n---END---";
        let issue = make_issue("test", &[]);
        let findings =
            IssueSolver::parse_multi_file_response(response, &issue, ContributionType::CodeQuality);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].file_path, "src/a.py");
        assert_eq!(findings[1].file_path, "src/b.py");
    }

    // Minimal mock LLM for tests
    struct MockLlm;
    #[async_trait::async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(
            &self, _: &str, _: Option<&str>, _: Option<f64>, _: Option<u32>,
        ) -> Result<String> {
            Ok("mock".into())
        }
        async fn chat(
            &self,
            _: &[crate::llm::provider::ChatMessage],
            _: Option<&str>,
            _: Option<f64>,
            _: Option<u32>,
        ) -> Result<String> {
            Ok("mock".into())
        }
    }

    // Safety: only used in tests, never actually called
    fn unsafe_mock_github() -> GitHubClient {
        GitHubClient::new("test-token", 100).unwrap()
    }
}
