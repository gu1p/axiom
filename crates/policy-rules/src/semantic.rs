use policy_core::{
    CodebaseFacts, Diagnostic, Level, Rule, RuleClass, RuleFactory, RuleMetadata,
    SemanticFindingFact, SemanticFindingKind,
};
use serde::Deserialize;
use toml::Table;

pub struct SemanticRuleFactory {
    pub id: &'static str,
    pub kind: SemanticFindingKind,
    pub description: &'static str,
    pub help: &'static str,
}

impl RuleFactory for SemanticRuleFactory {
    fn id(&self) -> &'static str {
        self.id
    }

    fn create(&self, table: &Table) -> Result<Box<dyn Rule>, String> {
        let config: SemanticRuleConfig = toml::Value::Table(table.clone())
            .try_into()
            .map_err(|error| format!("invalid configuration for `{}`: {error}", self.id))?;
        Ok(Box::new(SemanticRule {
            id: self.id,
            kind: self.kind,
            description: self.description,
            help: self.help,
            level: config.level,
        }))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SemanticRuleConfig {
    level: Level,
}

struct SemanticRule {
    id: &'static str,
    kind: SemanticFindingKind,
    description: &'static str,
    help: &'static str,
    level: Level,
}

impl Rule for SemanticRule {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            id: self.id,
            class: RuleClass::Smell,
            description: self.description,
        }
    }

    fn level(&self) -> Level {
        self.level
    }

    fn evaluate(&self, facts: &CodebaseFacts, diagnostics: &mut Vec<Diagnostic>) {
        diagnostics.extend(
            facts
                .semantic_findings
                .iter()
                .filter(|finding| finding.kind == self.kind)
                .map(|finding| self.diagnostic(finding)),
        );
    }
}

impl SemanticRule {
    fn diagnostic(&self, finding: &SemanticFindingFact) -> Diagnostic {
        Diagnostic {
            rule_id: self.id.to_owned(),
            class: RuleClass::Smell,
            level: self.level,
            message: message(finding),
            help: self.help.to_owned(),
            path: finding.path.clone(),
            span: finding.span,
            observed: None,
            limit: None,
            unit: None,
        }
    }
}

fn message(finding: &SemanticFindingFact) -> String {
    if finding.kind == SemanticFindingKind::PrivateDeadCode {
        return finding.item.clone();
    }
    let kind = finding.item_kind.as_deref().unwrap_or("item");
    match finding.kind {
        SemanticFindingKind::PrivateDeadCode => unreachable!(),
        SemanticFindingKind::DeadPublic => {
            format!("public {kind} `{}` is unreachable", finding.item)
        }
        SemanticFindingKind::TestOnly => {
            format!("{kind} `{}` is reachable only from tests", finding.item)
        }
        SemanticFindingKind::UnnecessaryPublic => {
            format!(
                "{kind} `{}` does not require public visibility",
                finding.item
            )
        }
        SemanticFindingKind::UnnecessaryRestrictedVisibility => format!(
            "restricted visibility on {kind} `{}` is unnecessary",
            finding.item
        ),
        SemanticFindingKind::UnnecessaryCrateVisibility => format!(
            "crate visibility on {kind} `{}` is broader than required",
            finding.item
        ),
    }
}
