use policy_core::{
    AnalysisError, AnalysisInput, CodebaseFacts, Diagnostic, Engine, FactProvider, Level,
    PolicyConfig, Rule,
};
use policy_syntax::SyntaxFactProvider;

use super::selection::{Family, Selection};

pub(super) struct Policies {
    size: Engine,
    testing: Engine,
    dead_code: Engine,
    visibility: Engine,
    collect_hir: bool,
    collect_private_dead_code: bool,
}

impl Policies {
    pub(super) fn prepare(
        config: &PolicyConfig,
        selection: Selection,
        ignore_warnings: bool,
    ) -> Result<Self, AnalysisError> {
        let rules = policy_rules::registry()
            .build(&config.rules)
            .map_err(AnalysisError::new)?;
        let mut families: [Vec<Box<dyn Rule>>; 4] = Default::default();
        let mut collect_hir = false;
        let mut collect_private_dead_code = false;
        for rule in rules {
            let id = rule.metadata().id;
            let Some(family) = Selection::policy_family(id) else {
                continue;
            };
            if !selection.includes(family)
                || rule.level() == Level::Allow
                || (ignore_warnings && rule.level() == Level::Warn)
            {
                continue;
            }
            collect_hir |= policy_rules::is_hir_rule(id);
            collect_private_dead_code |= id == policy_rules::PRIVATE_DEAD_CODE;
            families[policy_index(family)].push(rule);
        }
        let [size, testing, dead_code, visibility] = families;
        Ok(Self {
            size: Engine::new(Vec::new(), size),
            testing: Engine::new(Vec::new(), testing),
            dead_code: Engine::new(Vec::new(), dead_code),
            visibility: Engine::new(Vec::new(), visibility),
            collect_hir,
            collect_private_dead_code,
        })
    }

    pub(super) fn is_empty(&self) -> bool {
        !self.has_syntax() && !self.has_semantic()
    }

    pub(super) fn has_syntax(&self) -> bool {
        self.has_syntax_family() || self.has_semantic()
    }

    pub(super) fn has_syntax_family(&self) -> bool {
        self.has_family(Family::Size) || self.has_family(Family::Testing)
    }

    pub(super) const fn has_semantic(&self) -> bool {
        self.collect_hir || self.collect_private_dead_code
    }

    pub(super) fn collect_syntax(
        input: &AnalysisInput,
        facts: &mut CodebaseFacts,
    ) -> Result<(), Vec<AnalysisError>> {
        let engine = Engine::new(
            vec![Box::new(SyntaxFactProvider) as Box<dyn FactProvider>],
            Vec::new(),
        );
        engine.collect_into(input, facts)
    }

    pub(super) fn collect_semantic(
        &self,
        config: &PolicyConfig,
        input: &AnalysisInput,
        facts: &mut CodebaseFacts,
    ) -> Result<(), Vec<AnalysisError>> {
        let provider = crate::semantic::SemanticFactProvider::new(
            config.semantic.clone(),
            self.collect_hir,
            self.collect_private_dead_code,
        );
        let engine = Engine::new(vec![Box::new(provider)], Vec::new());
        engine.collect_into(input, facts)
    }

    pub(super) fn collect_semantic_fail_fast(
        &self,
        config: &PolicyConfig,
        input: &AnalysisInput,
        facts: &mut CodebaseFacts,
    ) -> Result<Option<Vec<Diagnostic>>, Vec<AnalysisError>> {
        let provider = crate::semantic::SemanticFactProvider::new(
            config.semantic.clone(),
            self.collect_hir,
            self.collect_private_dead_code,
        );
        let mut first = None;
        let stopped = provider.collect_fail_fast_private(input, facts, &mut |current_facts| {
            let mut diagnostics = self.dead_code.evaluate(current_facts);
            diagnostics.truncate(1);
            let found = !diagnostics.is_empty();
            if found {
                first = Some(diagnostics);
            }
            found
        })?;
        debug_assert_eq!(
            stopped,
            first.is_some(),
            "semantic collection only stops after storing its first diagnostic"
        );
        Ok(first)
    }

    pub(super) fn evaluate(&self, family: Family, facts: &CodebaseFacts) -> Vec<Diagnostic> {
        match family {
            Family::Size => self.size.evaluate(facts),
            Family::Testing => self.testing.evaluate(facts),
            Family::DeadCode => self.dead_code.evaluate(facts),
            Family::Visibility => self.visibility.evaluate(facts),
            Family::Clippy | Family::Rustdoc => Vec::new(),
        }
    }

    pub(super) fn evaluate_all(&self, facts: &CodebaseFacts) -> Vec<Diagnostic> {
        let mut diagnostics = [
            self.evaluate(Family::Size, facts),
            self.evaluate(Family::Testing, facts),
            self.evaluate(Family::DeadCode, facts),
            self.evaluate(Family::Visibility, facts),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        diagnostics.sort_by(|left, right| {
            (&left.path, left.span.byte_start, &left.rule_id).cmp(&(
                &right.path,
                right.span.byte_start,
                &right.rule_id,
            ))
        });
        diagnostics
    }

    pub(super) fn has_family(&self, family: Family) -> bool {
        match family {
            Family::Size => self.size.has_rules(),
            Family::Testing => self.testing.has_rules(),
            Family::DeadCode => self.dead_code.has_rules(),
            Family::Visibility => self.visibility.has_rules(),
            Family::Clippy | Family::Rustdoc => false,
        }
    }
}

const fn policy_index(family: Family) -> usize {
    match family {
        Family::Size => 0,
        Family::Testing => 1,
        Family::DeadCode => 2,
        Family::Visibility => 3,
        Family::Clippy | Family::Rustdoc => unreachable!(),
    }
}
