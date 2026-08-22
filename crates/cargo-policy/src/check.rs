use policy_cargo::Workspace;
use policy_core::{AnalysisError, Engine, FactProvider, Level, PolicyConfig};
use policy_syntax::SyntaxFactProvider;

use crate::{
    args::{CheckOptions, OutputFormat},
    report,
};

pub fn run(options: &CheckOptions) -> u8 {
    let workspace = match Workspace::discover(options.manifest_path.as_deref()) {
        Ok(workspace) => workspace,
        Err(error) => {
            return operational(options, &[AnalysisError::new(error.to_string())], None);
        }
    };
    let config_path = options
        .config
        .clone()
        .unwrap_or_else(|| workspace.policy_path());
    let config = match PolicyConfig::load(&config_path) {
        Ok(config) => config,
        Err(error) => {
            return operational(options, &[AnalysisError::new(error.to_string())], None);
        }
    };
    let rules = match policy_rules::registry().build(&config.rules) {
        Ok(rules) => rules,
        Err(error) => return operational(options, &[AnalysisError::new(error)], None),
    };
    let input = match workspace.load(&config.sources) {
        Ok(input) => input,
        Err(errors) => return operational(options, &errors, None),
    };
    let collect_hir = rules
        .iter()
        .any(|rule| rule.level() != Level::Allow && policy_rules::is_hir_rule(rule.metadata().id));
    let collect_private_dead_code = rules.iter().any(|rule| {
        rule.level() != Level::Allow && rule.metadata().id == policy_rules::PRIVATE_DEAD_CODE
    });
    let mut providers: Vec<Box<dyn FactProvider>> = vec![Box::new(SyntaxFactProvider)];
    if collect_hir || collect_private_dead_code {
        providers.push(Box::new(crate::semantic::SemanticFactProvider::new(
            config.semantic.clone(),
            collect_hir,
            collect_private_dead_code,
        )));
    }
    let engine = Engine::new(providers, rules);
    let diagnostics = match engine.run(&input) {
        Ok(diagnostics) => diagnostics,
        Err(errors) => return operational(options, &errors, Some(&input)),
    };
    report::policies(options, &input, &diagnostics);
    u8::from(diagnostics.iter().any(|item| item.level == Level::Deny))
}

fn operational(
    options: &CheckOptions,
    errors: &[AnalysisError],
    input: Option<&policy_core::AnalysisInput>,
) -> u8 {
    match options.format {
        OutputFormat::Human => report::human::operational(errors, input, options.color),
        OutputFormat::Json => report::json::operational(errors),
    }
    2
}
