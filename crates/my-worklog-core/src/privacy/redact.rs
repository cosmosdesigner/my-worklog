use crate::privacy::secret_detection::SecretPatterns;

#[derive(Debug)]
pub struct Redactor {
    secrets: SecretPatterns,
    home: Option<String>,
}

impl Redactor {
    pub fn new(home: Option<String>) -> Result<Self, regex::Error> {
        Ok(Self {
            secrets: SecretPatterns::compile()?,
            home,
        })
    }

    pub fn redact(&self, input: &str) -> String {
        let redacted = self.secrets.redact(input);
        match &self.home {
            Some(home) => redacted.replace(home, "~"),
            None => redacted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Redactor;

    #[test]
    fn redacts_api_key_when_present() {
        let redactor = Redactor::new(None).expect("redactor compiles");
        let redacted = redactor.redact("OPENAI_API_KEY=sk-secret");
        assert_eq!(redacted, "OPENAI_API_KEY=[REDACTED]");
    }

    #[test]
    fn redacts_bearer_token_when_present() {
        let redactor = Redactor::new(None).expect("redactor compiles");
        let redacted = redactor.redact("Authorization: Bearer abc123");
        assert_eq!(redacted, "Authorization: Bearer [REDACTED]");
    }
}
