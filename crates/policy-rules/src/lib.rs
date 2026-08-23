//! Built-in executable-policy rule registry.

mod file_lines;
mod function_lines;
mod limit;
mod semantic;
mod separate_test_files;

use policy_core::{RuleRegistry, SemanticFindingKind};

const FILE_MAX_LINES: &str = "size/file-max-lines";
const FUNCTION_MAX_LINES: &str = "size/function-max-lines";
const SEPARATE_TEST_FILES: &str = "testing/separate-test-files";
pub const PRIVATE_DEAD_CODE: &str = "dead-code/private";
const DEAD_PUBLIC: &str = "dead-code/public";
const TEST_ONLY: &str = "dead-code/test-only";
const UNNECESSARY_PUBLIC: &str = "visibility/unnecessary-public";
const UNNECESSARY_RESTRICTED: &str = "visibility/unnecessary-restricted";
const UNNECESSARY_CRATE: &str = "visibility/unnecessary-crate";

pub fn registry() -> RuleRegistry {
    let mut registry = RuleRegistry::default();
    registry.register(Box::new(file_lines::FileMaxLinesFactory));
    registry.register(Box::new(function_lines::FunctionMaxLinesFactory));
    registry.register(Box::new(separate_test_files::SeparateTestFilesFactory));
    register_semantic_rules(&mut registry);
    registry
}

fn is_semantic_rule(id: &str) -> bool {
    matches!(
        id,
        PRIVATE_DEAD_CODE
            | DEAD_PUBLIC
            | TEST_ONLY
            | UNNECESSARY_PUBLIC
            | UNNECESSARY_RESTRICTED
            | UNNECESSARY_CRATE
    )
}

pub fn is_hir_rule(id: &str) -> bool {
    is_semantic_rule(id) && id != PRIVATE_DEAD_CODE
}

fn register_semantic_rules(registry: &mut RuleRegistry) {
    for (id, kind, description, help) in [
        (
            PRIVATE_DEAD_CODE,
            SemanticFindingKind::PrivateDeadCode,
            "Private code must be used by a compiled target",
            "remove the item, use it from production code, or add a justified Rust dead_code allowance",
        ),
        (
            DEAD_PUBLIC,
            SemanticFindingKind::DeadPublic,
            "Public workspace APIs must be reachable",
            "remove the dead public API or declare the external product boundary in [semantic]",
        ),
        (
            TEST_ONLY,
            SemanticFindingKind::TestOnly,
            "Production declarations should not be used exclusively by tests",
            "move test support out of production code or add a real production consumer",
        ),
        (
            UNNECESSARY_PUBLIC,
            SemanticFindingKind::UnnecessaryPublic,
            "Public visibility should not exceed workspace needs",
            "reduce the declaration to pub(crate)",
        ),
        (
            UNNECESSARY_RESTRICTED,
            SemanticFindingKind::UnnecessaryRestrictedVisibility,
            "Restricted visibility should not exceed module needs",
            "make the declaration private",
        ),
        (
            UNNECESSARY_CRATE,
            SemanticFindingKind::UnnecessaryCrateVisibility,
            "Crate visibility should be reduced when a parent scope is sufficient",
            "reduce the declaration to pub(super)",
        ),
    ] {
        registry.register(Box::new(semantic::SemanticRuleFactory {
            id,
            kind,
            description,
            help,
        }));
    }
}
