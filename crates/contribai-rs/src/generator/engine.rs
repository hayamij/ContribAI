//! LLM-powered contribution generator.
//!
//! Port from Python `generator/engine.py`.
//! Takes findings from analysis and generates actual code changes,
//! tests, and commit messages that follow the target repo's conventions.

use chrono::Utc;
use regex::Regex;
use tracing::{info, warn};

use crate::core::config::ContributionConfig;
use crate::core::error::Result;
use crate::core::models::{
    Contribution, ContributionType, FileChange, Finding, RepoContext,
};
use crate::llm::provider::LlmProvider;

/// Generate code contributions from analysis findings.
pub struct ContributionGenerator<'a> {
    llm: &'a dyn LlmProvider,
    config: &'a ContributionConfig,
}

impl<'a> ContributionGenerator<'a> {
    pub fn new(llm: &'a dyn LlmProvider, config: &'a ContributionConfig) -> Self {
        Self { llm, config }
    }

    /// Generate a contribution for a single finding.
    ///
    /// Pipeline:
    /// 1. Build context-aware prompt
    /// 2. Get LLM to generate the fix
    /// 3. Parse structured output into FileChanges
    /// 4. Generate commit message
    /// 5. Self-review
    pub async fn generate(
        &self,
        finding: &Finding,
        context: &RepoContext,
    ) -> Result<Option<Contribution>> {
        // 1. Build prompts
        let system = self.build_system_prompt(context);
        let prompt = self.build_generation_prompt(finding, context);

        // 2. Generate with retry
        let mut changes = None;
        let mut last_error = String::new();

        for attempt in 0..2 {
            let actual_prompt = if attempt > 0 {
                format!(
                    "{}\n\n## IMPORTANT: Your previous attempt failed.\n\
                     Error: {}\n\
                     Please fix the issue and return ONLY valid JSON.",
                    prompt, last_error
                )
            } else {
                prompt.clone()
            };

            let response = self
                .llm
                .complete(&actual_prompt, Some(&system), Some(0.2), None)
                .await?;

            // 3. Parse changes
            match self.parse_changes(&response) {
                Some(c) if !c.is_empty() => {
                    // Validate
                    if self.validate_changes(&c) {
                        changes = Some(c);
                        break;
                    } else {
                        last_error = "Generated code failed syntax validation".into();
                    }
                }
                _ => {
                    last_error = "No valid changes could be parsed from JSON output".into();
                }
            }
        }

        let changes = match changes {
            Some(c) => c,
            None => {
                warn!(title = %finding.title, "No valid changes after retries");
                return Ok(None);
            }
        };

        // 4. Generate commit message
        let commit_msg = self.generate_commit_message(finding);

        // 5. Generate branch name
        let branch_name = Self::generate_branch_name(finding);

        // 6. Generate PR title
        let pr_title = Self::generate_pr_title(finding);

        let contribution = Contribution {
            finding: finding.clone(),
            contribution_type: finding.finding_type.clone(),
            title: pr_title,
            description: finding.description.clone(),
            changes,
            commit_message: commit_msg,
            tests_added: vec![],
            branch_name,
            generated_at: Utc::now(),
        };

        info!(
            title = %contribution.title,
            files = contribution.total_files_changed(),
            "Generated contribution"
        );

        Ok(Some(contribution))
    }

    /// Build system prompt with repo context and style guidance.
    fn build_system_prompt(&self, context: &RepoContext) -> String {
        let mut prompt = String::from(
            "You are a senior open-source contributor who writes production-ready \
             code. You understand that PRs are judged by maintainers who value \
             minimal, focused, and convention-matching changes.\n\n\
             RULES FOR GENERATING CHANGES:\n\
             1. Match existing code style EXACTLY (indentation, naming, patterns)\n\
             2. Make the SMALLEST change that correctly fixes the issue\n\
             3. Include proper error handling consistent with the codebase\n\
             4. Do NOT break existing functionality\n\
             5. Do NOT add unnecessary dependencies or imports\n\
             6. Do NOT refactor adjacent code — fix only the reported issue\n\
             7. Do NOT add comments explaining what the code does\n\
             8. Do NOT modify files unrelated to the finding\n\n\
             OUTPUT FORMAT:\n\
             Return ONLY raw JSON — no markdown fences, no ```json blocks.\n\
             The response must be valid, parseable JSON.\n\n\
             ACCEPTANCE CRITERIA:\n\
             - Would a busy maintainer merge this in under 30 seconds?\n\
             - Is the change obviously correct with no side effects?\n",
        );

        if let Some(style) = &context.coding_style {
            prompt.push_str(&format!(
                "\nCODEBASE STYLE:\n{}\n\
                 You MUST match these conventions exactly.\n",
                style
            ));
        }

        prompt.push_str(&format!(
            "\nREPOSITORY: {}\nLanguage: {}\n",
            context.repo.full_name,
            context.repo.language.as_deref().unwrap_or("unknown")
        ));

        prompt
    }

    /// Build the generation prompt based on finding.
    fn build_generation_prompt(&self, finding: &Finding, context: &RepoContext) -> String {
        let current_content = context
            .relevant_files
            .get(&finding.file_path)
            .map(|s| s.as_str())
            .unwrap_or("");

        format!(
            "Generate a code fix for this issue:\n\n\
             **Title**: {}\n\
             **Severity**: {}\n\
             **File**: {}\n\
             **Description**: {}\n\
             {}\n\n\
             Current file content:\n```\n{}\n```\n\n\
             Respond with a JSON array of file changes:\n\
             ```json\n\
             [{{\n\
               \"path\": \"file/path.py\",\n\
               \"new_content\": \"...full fixed content...\",\n\
               \"is_new_file\": false\n\
             }}]\n\
             ```",
            finding.title,
            finding.severity,
            finding.file_path,
            finding.description,
            finding
                .suggestion
                .as_deref()
                .map(|s| format!("**Suggestion**: {}", s))
                .unwrap_or_default(),
            current_content
        )
    }

    /// Parse LLM response into FileChange objects.
    fn parse_changes(&self, response: &str) -> Option<Vec<FileChange>> {
        // Extract JSON array
        let start = response.find('[')?;
        let end = response.rfind(']')?;
        let json_str = &response[start..=end];

        let items: Vec<serde_json::Value> = serde_json::from_str(json_str).ok()?;

        let changes: Vec<FileChange> = items
            .into_iter()
            .filter_map(|item| {
                let path = item["path"].as_str()?.to_string();
                let new_content = item["new_content"].as_str()?.to_string();
                let is_new_file = item["is_new_file"].as_bool().unwrap_or(false);

                Some(FileChange {
                    path,
                    original_content: None,
                    new_content,
                    is_new_file,
                    is_deleted: false,
                })
            })
            .collect();

        if changes.is_empty() {
            None
        } else {
            Some(changes)
        }
    }

    /// Validate generated changes for basic sanity.
    fn validate_changes(&self, changes: &[FileChange]) -> bool {
        for change in changes {
            let content = &change.new_content;

            // Check for empty content
            if content.trim().is_empty() {
                return false;
            }

            // Check for balanced brackets/braces/parens
            let opens: usize = content.matches('{').count()
                + content.matches('(').count()
                + content.matches('[').count();
            let closes: usize = content.matches('}').count()
                + content.matches(')').count()
                + content.matches(']').count();

            // Allow some imbalance (strings can contain brackets)
            if (opens as i64 - closes as i64).unsigned_abs() > 5 {
                return false;
            }
        }
        true
    }

    /// Generate a conventional commit message.
    fn generate_commit_message(&self, finding: &Finding) -> String {
        let prefix = match finding.finding_type {
            ContributionType::SecurityFix => "fix",
            ContributionType::CodeQuality => "fix",
            ContributionType::DocsImprove => "docs",
            ContributionType::PerformanceOpt => "perf",
            ContributionType::FeatureAdd => "feat",
            ContributionType::Refactor => "refactor",
            ContributionType::UiUxFix => "fix",
        };

        // Extract scope from file path
        let scope = finding
            .file_path
            .split('/')
            .nth(1)
            .filter(|_| {
                finding.file_path.starts_with("src/")
                    || finding.file_path.starts_with("packages/")
                    || finding.file_path.starts_with("apps/")
            })
            .unwrap_or("");

        let title = finding.title.to_lowercase();
        let title = if title.len() > 50 { &title[..50] } else { &title };

        if scope.is_empty() {
            format!("{}: {}", prefix, title)
        } else {
            format!("{}({}): {}", prefix, scope, title)
        }
    }

    /// Generate a natural-looking branch name.
    pub fn generate_branch_name(finding: &Finding) -> String {
        let prefix = match finding.finding_type {
            ContributionType::SecurityFix => "fix/security",
            ContributionType::CodeQuality => "fix",
            ContributionType::DocsImprove => "docs",
            ContributionType::PerformanceOpt => "perf",
            ContributionType::FeatureAdd => "feat",
            ContributionType::Refactor => "refactor",
            ContributionType::UiUxFix => "fix/ui",
        };

        let re = Regex::new(r"[^a-z0-9]+").unwrap();
        let lower = finding.title.to_lowercase();
        let slug = re.replace_all(&lower, "-");
        let slug = slug.trim_matches('-');
        let slug = if slug.len() > 50 { &slug[..50] } else { slug };

        format!("{}/{}", prefix, slug)
    }

    /// Generate a PR title using conventional commit format.
    pub fn generate_pr_title(finding: &Finding) -> String {
        let prefix = match finding.finding_type {
            ContributionType::SecurityFix => "fix",
            ContributionType::CodeQuality => "fix",
            ContributionType::DocsImprove => "docs",
            ContributionType::PerformanceOpt => "perf",
            ContributionType::FeatureAdd => "feat",
            ContributionType::Refactor => "refactor",
            ContributionType::UiUxFix => "fix",
        };

        format!("{}: {}", prefix, finding.title.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{ContributionType, Severity};

    fn test_finding() -> Finding {
        Finding {
            id: "test".into(),
            finding_type: ContributionType::SecurityFix,
            severity: Severity::High,
            title: "SQL injection in user query".into(),
            description: "User input not sanitized".into(),
            file_path: "src/db/queries.py".into(),
            line_start: Some(42),
            line_end: Some(45),
            suggestion: Some("Use parameterized queries".into()),
            confidence: 0.9,
            priority_signals: vec![],
        }
    }

    #[test]
    fn test_generate_branch_name() {
        let f = test_finding();
        let branch = ContributionGenerator::generate_branch_name(&f);
        assert!(branch.starts_with("fix/security/"));
        assert!(branch.contains("sql-injection"));
    }

    #[test]
    fn test_generate_pr_title() {
        let f = test_finding();
        let title = ContributionGenerator::generate_pr_title(&f);
        assert!(title.starts_with("fix: "));
        assert!(title.contains("sql injection"));
    }

    #[test]
    fn test_generate_commit_message() {
        let config = ContributionConfig::default();
        let gen = ContributionGenerator {
            llm: &MockLlm,
            config: &config,
        };
        let f = test_finding();
        let msg = gen.generate_commit_message(&f);
        assert!(msg.starts_with("fix(db): "));
    }

    #[test]
    fn test_parse_changes_valid() {
        let config = ContributionConfig::default();
        let gen = ContributionGenerator {
            llm: &MockLlm,
            config: &config,
        };

        let response = r#"[{"path": "src/main.py", "new_content": "print('fixed')", "is_new_file": false}]"#;
        let changes = gen.parse_changes(response);
        assert!(changes.is_some());
        assert_eq!(changes.unwrap().len(), 1);
    }

    #[test]
    fn test_parse_changes_invalid() {
        let config = ContributionConfig::default();
        let gen = ContributionGenerator {
            llm: &MockLlm,
            config: &config,
        };

        let response = "This is not valid JSON at all";
        let changes = gen.parse_changes(response);
        assert!(changes.is_none());
    }

    #[test]
    fn test_validate_changes() {
        let config = ContributionConfig::default();
        let gen = ContributionGenerator {
            llm: &MockLlm,
            config: &config,
        };

        let good = vec![FileChange {
            path: "test.py".into(),
            original_content: None,
            new_content: "def foo():\n    return 42\n".into(),
            is_new_file: false,
            is_deleted: false,
        }];
        assert!(gen.validate_changes(&good));

        let empty = vec![FileChange {
            path: "test.py".into(),
            original_content: None,
            new_content: "   \n  ".into(),
            is_new_file: false,
            is_deleted: false,
        }];
        assert!(!gen.validate_changes(&empty));
    }

    /// Mock LLM for unit tests.
    struct MockLlm;

    #[async_trait::async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
            _temperature: Option<f64>,
            _max_tokens: Option<u32>,
        ) -> Result<String> {
            Ok("mock response".into())
        }

        async fn chat(
            &self,
            _messages: &[crate::llm::provider::ChatMessage],
            _system: Option<&str>,
            _temperature: Option<f64>,
            _max_tokens: Option<u32>,
        ) -> Result<String> {
            Ok("mock response".into())
        }
    }
}
