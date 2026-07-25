mod providers;

use reqwest::StatusCode;
use thiserror::Error;

const SHARE_RESPONSE_MAX_TOKENS: u16 = 1_600;
const SUMMARY_SYSTEM_INSTRUCTION: &str =
    "You turn developer worklogs into accurate, human-readable status updates.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProvider {
    OpenAi,
    DeepSeek,
}

impl LlmProvider {
    pub const fn api_key_env(self) -> &'static str {
        match self {
            Self::OpenAi => "OPENAI_API_KEY",
            Self::DeepSeek => "DEEPSEEK_API_KEY",
        }
    }

    pub const fn default_model(self) -> &'static str {
        match self {
            Self::OpenAi => "gpt-5.6",
            Self::DeepSeek => "deepseek-v4-pro",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SummaryRequest {
    pub provider: LlmProvider,
    pub model: String,
    pub api_key: String,
    pub report: String,
    pub audience: SummaryAudience,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryAudience {
    Boss,
    Client,
    Standup,
}

impl SummaryAudience {
    const fn instruction(self) -> &'static str {
        match self {
            Self::Boss => {
                "Write this as a manager-ready report with this exact structure:\nTitle: one clear line summarizing the period.\nDetailed explanation: 1-3 concise paragraphs explaining what was done, why it matters, and validation/blockers when present.\nDecisions: bullet list of decisions made, or 'None captured' if absent.\nFeatures: bullet list of features or improvements delivered, or 'None captured' if absent.\nBugs fixed: bullet list of bugs/debugging/fixes, or 'None captured' if absent.\nMetrics: bullet list with token count and total time spent when present in the source context, or 'Not available' if absent."
            }
            Self::Client => {
                "Write this as a concise client-facing update. Use 2 short paragraphs. Emphasize value delivered and next steps. Do not expose internal tooling noise."
            }
            Self::Standup => {
                "Write this as a standup update. Use 3 bullets: Yesterday/Today/Blockers when possible."
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum SummaryError {
    #[error("missing API key: set {0}")]
    MissingApiKey(&'static str),
    #[error("LLM request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("LLM provider returned {status}: {body}")]
    ProviderStatus { status: StatusCode, body: String },
    #[error("LLM response body was not valid JSON: {body}")]
    InvalidJson { body: String },
    #[error("LLM response did not include text output")]
    MissingOutput,
    #[error("LLM response was still incomplete after continuation attempts")]
    IncompleteOutput,
}

pub fn summarize(request: &SummaryRequest) -> Result<String, SummaryError> {
    providers::summarize(request)
}

pub fn build_summary_input(report: &str, audience: SummaryAudience) -> String {
    format!(
        "## Task\nTurn the my-worklog source context into a shareable work update.\n\n## Audience\n{}\n\n## Rules\n- Use only facts present in the source context.\n- Do not invent facts, validation, blockers, metrics, dates, or outcomes.\n- Do not include raw JSON or internal event payloads.\n- Mention metrics only when they appear in the source context.\n- Complete every required response-format section and do not stop mid-sentence.\n\n## Response format\n{}\n\n## Source context\n{}",
        audience_name(audience),
        audience.instruction(),
        report.trim()
    )
}

const fn audience_name(audience: SummaryAudience) -> &'static str {
    match audience {
        SummaryAudience::Boss => "manager",
        SummaryAudience::Client => "client",
        SummaryAudience::Standup => "standup",
    }
}

pub(crate) fn summary_output(content: String) -> Result<String, SummaryError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(SummaryError::MissingOutput);
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{
        LlmProvider, SHARE_RESPONSE_MAX_TOKENS, SummaryAudience, build_summary_input,
        summary_output,
    };

    #[test]
    fn openai_defaults_to_best_summary_model() {
        assert_eq!(LlmProvider::OpenAi.default_model(), "gpt-5.6");
        assert_eq!(LlmProvider::OpenAi.api_key_env(), "OPENAI_API_KEY");
    }

    #[test]
    fn deepseek_uses_openai_compatible_chat_model() {
        assert_eq!(LlmProvider::DeepSeek.default_model(), "deepseek-v4-pro");
        assert_eq!(LlmProvider::DeepSeek.api_key_env(), "DEEPSEEK_API_KEY");
    }

    #[test]
    fn share_response_budget_supports_complete_report_output() {
        let budget = SHARE_RESPONSE_MAX_TOKENS;
        assert!((1_500..=2_000).contains(&budget));
    }

    #[test]
    fn prompt_asks_for_manager_paragraphs_without_raw_invention() {
        let input = build_summary_input("- User: fixed report rendering", SummaryAudience::Boss);

        assert!(input.contains("Title:"));
        assert!(input.contains("Detailed explanation:"));
        assert!(input.contains("Do not invent facts"));
        assert!(input.contains("fixed report rendering"));
    }

    #[test]
    fn boss_prompt_requires_report_sections_for_decisions_features_bugs_and_metrics() {
        let input = build_summary_input(
            "## Metrics\n- Total time: 2h 05m 00s\n- Tokens: 12,300 total (8,000 input, 4,300 output)\n\n- User: fixed importer bug",
            SummaryAudience::Boss,
        );

        assert!(input.contains("Title:"));
        assert!(input.contains("Detailed explanation:"));
        assert!(input.contains("Decisions:"));
        assert!(input.contains("Features:"));
        assert!(input.contains("Bugs fixed:"));
        assert!(input.contains("Metrics:"));
        assert!(input.contains("token count and total time spent"));
    }

    #[test]
    fn prompt_has_clear_sections_and_metric_rules() {
        let input = build_summary_input(
            "## Metrics\n- Tokens: 1,234 total (900 input, 334 output)",
            SummaryAudience::Client,
        );

        assert!(input.contains("## Task"));
        assert!(input.contains("## Response format"));
        assert!(input.contains("## Source context"));
        assert!(input.contains("Mention metrics only when they appear in the source context."));
        assert!(input.contains("Complete every required response-format section"));
        assert!(input.contains("Do not include raw JSON"));
    }

    #[test]
    fn summary_output_trims_provider_whitespace() {
        let output = summary_output("\n\nFinished the report fixes.  \n".to_owned())
            .expect("summary output");

        assert_eq!(output, "Finished the report fixes.");
    }
}
