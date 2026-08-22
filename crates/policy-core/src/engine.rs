use crate::{AnalysisError, AnalysisInput, CodebaseFacts, Diagnostic, Level, RuleMetadata};

pub trait FactProvider: Send + Sync {
    /// Adds facts derived from the analysis input to the shared fact store.
    ///
    /// # Errors
    ///
    /// Returns every operational error that prevents complete fact collection.
    fn collect(
        &self,
        input: &AnalysisInput,
        facts: &mut CodebaseFacts,
    ) -> Result<(), Vec<AnalysisError>>;
}

pub trait Rule: Send + Sync {
    fn metadata(&self) -> RuleMetadata;
    fn level(&self) -> Level;
    fn evaluate(&self, facts: &CodebaseFacts, diagnostics: &mut Vec<Diagnostic>);
}

#[derive(Default)]
pub struct Engine {
    providers: Vec<Box<dyn FactProvider>>,
    rules: Vec<Box<dyn Rule>>,
}

impl Engine {
    pub fn new(providers: Vec<Box<dyn FactProvider>>, rules: Vec<Box<dyn Rule>>) -> Self {
        Self { providers, rules }
    }

    /// Collects all facts and evaluates every enabled rule.
    ///
    /// # Errors
    ///
    /// Returns provider errors without evaluating policies against partial facts.
    pub fn run(&self, input: &AnalysisInput) -> Result<Vec<Diagnostic>, Vec<AnalysisError>> {
        let mut facts = CodebaseFacts::default();
        let mut errors = Vec::new();
        for provider in &self.providers {
            if let Err(mut provider_errors) = provider.collect(input, &mut facts) {
                errors.append(&mut provider_errors);
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        let mut diagnostics = Vec::new();
        for rule in &self.rules {
            if rule.level() != Level::Allow {
                rule.evaluate(&facts, &mut diagnostics);
            }
        }
        diagnostics.sort_by(|left, right| {
            (&left.path, left.span.byte_start, &left.rule_id).cmp(&(
                &right.path,
                right.span.byte_start,
                &right.rule_id,
            ))
        });
        Ok(diagnostics)
    }
}
