mod file_lines;
mod function_lines;
mod limit;

use policy_core::RuleRegistry;

pub const FILE_MAX_LINES: &str = "size/file-max-lines";
pub const FUNCTION_MAX_LINES: &str = "size/function-max-lines";

pub fn registry() -> RuleRegistry {
    let mut registry = RuleRegistry::default();
    registry.register(Box::new(file_lines::FileMaxLinesFactory));
    registry.register(Box::new(function_lines::FunctionMaxLinesFactory));
    registry
}
