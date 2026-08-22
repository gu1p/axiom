use std::collections::BTreeMap;

use toml::Table;

use crate::{CodebaseFacts, Diagnostic, Level, Rule, RuleMetadata, RuleScope};

pub trait RuleFactory: Send + Sync {
    fn id(&self) -> &'static str;
    /// Builds one configured rule.
    ///
    /// # Errors
    ///
    /// Returns a validation error when the rule-specific table is invalid.
    fn create(&self, config: &Table) -> Result<Box<dyn Rule>, String>;
}

#[derive(Default)]
pub struct RuleRegistry {
    factories: BTreeMap<&'static str, Box<dyn RuleFactory>>,
}

impl RuleRegistry {
    pub fn register(&mut self, factory: Box<dyn RuleFactory>) {
        self.factories.insert(factory.id(), factory);
    }

    /// Builds all configured rules in stable rule-ID order.
    ///
    /// # Errors
    ///
    /// Returns an error for unknown rule IDs or invalid rule options.
    pub fn build(
        &self,
        configured: &BTreeMap<String, Table>,
    ) -> Result<Vec<Box<dyn Rule>>, String> {
        let mut rules = Vec::new();
        for (id, config) in configured {
            let factory = self.factories.get(id.as_str()).ok_or_else(|| {
                let known = self
                    .factories
                    .keys()
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("unknown rule `{id}`; known rules: {known}")
            })?;
            let (scope, rule_config) = split_scope(id, config)?;
            let inner = factory.create(&rule_config)?;
            rules.push(Box::new(ScopedRule { inner, scope }) as Box<dyn Rule>);
        }
        Ok(rules)
    }
}

fn split_scope(id: &str, config: &Table) -> Result<(RuleScope, Table), String> {
    let mut rule_config = config.clone();
    let Some(value) = rule_config.remove("scope") else {
        return Ok((RuleScope::All, rule_config));
    };
    let scope = value
        .try_into()
        .map_err(|error| format!("invalid configuration for `{id}`: invalid scope: {error}"))?;
    Ok((scope, rule_config))
}

struct ScopedRule {
    inner: Box<dyn Rule>,
    scope: RuleScope,
}

impl Rule for ScopedRule {
    fn metadata(&self) -> RuleMetadata {
        self.inner.metadata()
    }

    fn level(&self) -> Level {
        self.inner.level()
    }

    fn evaluate(&self, facts: &CodebaseFacts, diagnostics: &mut Vec<Diagnostic>) {
        let mut candidates = Vec::new();
        self.inner.evaluate(facts, &mut candidates);
        candidates.retain(|diagnostic| {
            facts.matches_scope(&diagnostic.path, diagnostic.span.byte_start, self.scope)
        });
        diagnostics.extend(candidates);
    }
}
