use std::env;
use std::io::{self, IsTerminal, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Result;
use clap::{Args, ValueEnum};
use my_worklog_core::WorklogDb;
use my_worklog_core::report::{daily, weekly};
use my_worklog_summarizer::llm::{
    LlmProvider, SummaryAudience, SummaryError, SummaryRequest, build_summary_input, summarize,
};

use crate::commands::Context;

#[derive(Debug, Args)]
pub struct ShareArgs {
    #[arg(value_enum, help = "Report period to summarize")]
    pub period: SharePeriod,
    #[arg(long, value_enum, default_value_t = ShareProvider::OpenAi, help = "LLM provider to call")]
    pub provider: ShareProvider,
    #[arg(long, help = "Override the provider default model")]
    pub model: Option<String>,
    #[arg(long, value_enum, default_value_t = ShareAudience::Boss, help = "Audience/style for the generated summary")]
    pub audience: ShareAudience,
    #[arg(
        long,
        help = "Print the exact prompt that would be sent and do not call the LLM"
    )]
    pub print_prompt: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SharePeriod {
    Today,
    Yesterday,
    Week,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShareProvider {
    OpenAi,
    DeepSeek,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShareAudience {
    Boss,
    Client,
    Standup,
}

pub fn run(context: &Context, args: &ShareArgs) -> Result<()> {
    let db = WorklogDb::open_existing(context.paths.database())?;
    let report = match args.period {
        SharePeriod::Today => daily::today_share_context(db.connection())?,
        SharePeriod::Yesterday => daily::yesterday_share_context(db.connection())?,
        SharePeriod::Week => weekly::week_share_context(db.connection())?,
    };
    let audience = args.audience.into();
    if args.print_prompt {
        println!("{}", build_summary_input(&report, audience));
        return Ok(());
    }
    let provider: LlmProvider = args.provider.into();
    let api_key = env::var(provider.api_key_env())
        .map_err(|_| SummaryError::MissingApiKey(provider.api_key_env()))?;
    let request = SummaryRequest {
        provider,
        model: args
            .model
            .clone()
            .unwrap_or_else(|| provider.default_model().to_owned()),
        api_key,
        report,
        audience,
    };
    let loading = LoadingIndicator::start(loading_label(args.provider, &request.model));
    let summary = summarize(&request);
    drop(loading);
    println!("{}", summary?);
    Ok(())
}

fn loading_label(provider: ShareProvider, model: &str) -> String {
    format!(
        "Waiting for {} ({model}) to generate the report...",
        provider.label()
    )
}

struct LoadingIndicator {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    active: bool,
}

impl LoadingIndicator {
    fn start(message: String) -> Self {
        if !io::stderr().is_terminal() {
            eprintln!("{message}");
            return Self {
                stop: Arc::new(AtomicBool::new(true)),
                handle: None,
                active: false,
            };
        }
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let handle = thread::spawn(move || {
            let frames = ["|", "/", "-", "\\"];
            let mut index = 0usize;
            while !thread_stop.load(Ordering::Relaxed) {
                eprint!("\r{} {message}", frames[index % frames.len()]);
                let _ = io::stderr().flush();
                index += 1;
                thread::sleep(Duration::from_millis(120));
            }
        });
        Self {
            stop,
            handle: Some(handle),
            active: true,
        }
    }
}

impl Drop for LoadingIndicator {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        if self.active {
            eprint!("\r\x1b[2K");
            let _ = io::stderr().flush();
        }
    }
}

impl From<ShareProvider> for LlmProvider {
    fn from(value: ShareProvider) -> Self {
        match value {
            ShareProvider::OpenAi => Self::OpenAi,
            ShareProvider::DeepSeek => Self::DeepSeek,
        }
    }
}

impl ShareProvider {
    const fn label(self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI",
            Self::DeepSeek => "DeepSeek",
        }
    }
}

impl From<ShareAudience> for SummaryAudience {
    fn from(value: ShareAudience) -> Self {
        match value {
            ShareAudience::Boss => Self::Boss,
            ShareAudience::Client => Self::Client,
            ShareAudience::Standup => Self::Standup,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ShareProvider, loading_label};

    #[test]
    fn loading_label_names_provider_and_model() {
        let label = loading_label(ShareProvider::DeepSeek, "deepseek-v4-pro");

        assert!(label.contains("DeepSeek"));
        assert!(label.contains("deepseek-v4-pro"));
        assert!(label.contains("generate the report"));
    }
}
