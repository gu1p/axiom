mod directory;
mod file_lines;
mod function_lines;
mod limit;

use policy_core::RuleRegistry;

const DIRECTORY_MAX_FILES: &str = "size/directory-max-files";
const DIRECTORY_MAX_LINES: &str = "size/directory-max-lines";
const FILE_MAX_LINES: &str = "size/file-max-lines";
const FUNCTION_MAX_LINES: &str = "size/function-max-lines";

pub(super) fn register(registry: &mut RuleRegistry) {
    registry.register(Box::new(directory::DirectoryMaxFilesFactory));
    registry.register(Box::new(directory::DirectoryMaxLinesFactory));
    registry.register(Box::new(file_lines::FileMaxLinesFactory));
    registry.register(Box::new(function_lines::FunctionMaxLinesFactory));
}
