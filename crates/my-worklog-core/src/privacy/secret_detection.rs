use regex::Regex;

#[derive(Debug)]
pub struct SecretPatterns {
    patterns: Vec<(Regex, &'static str)>,
}

impl SecretPatterns {
    pub fn compile() -> Result<Self, regex::Error> {
        let specs = [
            (
                r"(?i)(OPENAI_API_KEY|ANTHROPIC_API_KEY|API_KEY|TOKEN|PASSWORD|SECRET)=([^\s]+)",
                "$1=[REDACTED]",
            ),
            (
                r"(?i)Authorization:\s*Bearer\s+[^\s]+",
                "Authorization: Bearer [REDACTED]",
            ),
            (r"postgres(ql)?://[^\s]+", "postgres://[REDACTED]"),
            (r"mysql://[^\s]+", "mysql://[REDACTED]"),
            (r"mongodb(\+srv)?://[^\s]+", "mongodb://[REDACTED]"),
            (
                r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----",
                "[REDACTED_PRIVATE_KEY]",
            ),
            (
                r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+",
                "[REDACTED_JWT]",
            ),
            (r"(?i)(Cookie|Set-Cookie):\s*[^\n]+", "$1: [REDACTED]"),
        ];
        let patterns = specs
            .into_iter()
            .map(|(pattern, replacement)| Regex::new(pattern).map(|regex| (regex, replacement)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { patterns })
    }

    pub fn redact(&self, input: &str) -> String {
        self.patterns
            .iter()
            .fold(input.to_owned(), |text, (regex, replacement)| {
                regex.replace_all(&text, *replacement).into_owned()
            })
    }
}
