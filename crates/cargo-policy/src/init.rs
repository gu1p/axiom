use std::fs::OpenOptions;
use std::io::{self, Write as _};

use policy_cargo::Workspace;

use crate::args::InitOptions;

const BASE_CONFIG: &str = r#"version = 1

[sources]
include = ["**/*.rs"]
exclude = []
test = ["**/tests.rs", "**/*_test.rs", "**/*_tests.rs", "**/tests/**/*.rs"]

[tools.clippy]
enabled = true
profile = "axiom"
check-docs = true
targets = "all"
features = "default"
warnings = "deny"
"#;

const BASE_RULES: &str = r#"

[rules."size/function-max-lines"]
level = "deny"
limit = 50
scope = "production"

[rules."size/file-max-lines"]
level = "deny"
limit = 200
scope = "production"

[rules."size/directory-max-files"]
level = "deny"
limit = 5
scope = "production"

[rules."size/directory-max-lines"]
level = "deny"
limit = 1000
scope = "production"

[rules."testing/separate-test-files"]
level = "deny"
"#;

const ACTIVE_SEMANTIC_RULES: &str = r#"
[rules."dead-code/private"]
level = "warn"

[rules."dead-code/public"]
level = "warn"

[rules."dead-code/test-only"]
level = "warn"

[rules."visibility/unnecessary-public"]
level = "deny"

[rules."visibility/unnecessary-restricted"]
level = "warn"

[rules."visibility/unnecessary-crate"]
level = "warn"
"#;

const LIBRARY_SEMANTIC_EXAMPLE: &str = r#"
# Semantic analysis needs an explicit product boundary in a library-only workspace.
# Uncomment and update this declaration and the semantic rules below.
#
# [[semantic.production]]
# package = "my-package"
# lib = "my_library"
# reason = "internal library audited as a closed-world product"
#
# [rules."dead-code/private"]
# level = "warn"
# [rules."dead-code/public"]
# level = "warn"
# [rules."dead-code/test-only"]
# level = "warn"
# [rules."visibility/unnecessary-public"]
# level = "deny"
# [rules."visibility/unnecessary-restricted"]
# level = "warn"
# [rules."visibility/unnecessary-crate"]
# level = "warn"
"#;

pub fn run(options: &InitOptions) -> u8 {
    let workspace = match Workspace::discover(options.manifest_path.as_deref()) {
        Ok(workspace) => workspace,
        Err(error) => {
            let _ = writeln!(io::stderr(), "error: {error}");
            return 2;
        }
    };
    let path = workspace.policy_path();
    let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(file) => file,
        Err(error) => {
            let _ = writeln!(io::stderr(), "error: could not create {path}: {error}");
            return 2;
        }
    };
    let semantic = if workspace.has_binary_targets() {
        ACTIVE_SEMANTIC_RULES
    } else {
        LIBRARY_SEMANTIC_EXAMPLE
    };
    if let Err(error) = file
        .write_all(BASE_CONFIG.as_bytes())
        .and_then(|()| file.write_all(BASE_RULES.as_bytes()))
        .and_then(|()| file.write_all(semantic.as_bytes()))
    {
        let _ = writeln!(io::stderr(), "error: could not write {path}: {error}");
        return 2;
    }
    let _ = writeln!(io::stdout(), "created {path}");
    0
}
