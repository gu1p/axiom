use std::collections::BTreeMap;

use toml::Table;

use crate::Rule;

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
            rules.push(factory.create(config)?);
        }
        Ok(rules)
    }
}
