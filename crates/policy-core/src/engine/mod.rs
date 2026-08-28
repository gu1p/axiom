mod registry;

pub use registry::{RuleFactory, RuleRegistry};

use crate::{
    AnalysisError, AnalysisInput, CodebaseFacts, Diagnostic, Level, RuleMetadata, RuleScope,
};

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

    /// Evaluates this rule for one configured source scope.
    ///
    /// Aggregate rules can override this hook to apply the scope before calculating totals.
    fn evaluate_scoped(
        &self,
        facts: &CodebaseFacts,
        scope: RuleScope,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut candidates = Vec::new();
        self.evaluate(facts, &mut candidates);
        candidates.retain(|diagnostic| {
            facts.matches_scope(&diagnostic.path, diagnostic.span.byte_start, scope)
        });
        diagnostics.extend(candidates);
    }
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

    /// Adds this engine's facts to an existing fact store.
    ///
    /// # Errors
    ///
    /// Returns every provider error without discarding facts collected by other engines.
    pub fn collect_into(
        &self,
        input: &AnalysisInput,
        facts: &mut CodebaseFacts,
    ) -> Result<(), Vec<AnalysisError>> {
        let mut errors = Vec::new();
        for provider in &self.providers {
            if let Err(mut provider_errors) = provider.collect(input, facts) {
                errors.append(&mut provider_errors);
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(())
    }

    /// Evaluates this engine's enabled rules against previously collected facts.
    pub fn evaluate(&self, facts: &CodebaseFacts) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for rule in &self.rules {
            if rule.level() != Level::Allow {
                rule.evaluate(facts, &mut diagnostics);
            }
        }
        diagnostics.sort_by(|left, right| {
            (&left.path, left.span.byte_start, &left.rule_id).cmp(&(
                &right.path,
                right.span.byte_start,
                &right.rule_id,
            ))
        });
        diagnostics
    }

    #[must_use]
    pub fn has_rules(&self) -> bool {
        !self.rules.is_empty()
    }
}
