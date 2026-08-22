mod file_lines;
mod function_lines;
mod limit;
mod separate_test_files;

use policy_core::RuleRegistry;

pub const FILE_MAX_LINES: &str = "size/file-max-lines";
pub const FUNCTION_MAX_LINES: &str = "size/function-max-lines";
pub const SEPARATE_TEST_FILES: &str = "testing/separate-test-files";

pub fn registry() -> RuleRegistry {
    let mut registry = RuleRegistry::default();
    registry.register(Box::new(file_lines::FileMaxLinesFactory));
    registry.register(Box::new(function_lines::FunctionMaxLinesFactory));
    registry.register(Box::new(separate_test_files::SeparateTestFilesFactory));
    registry
}
