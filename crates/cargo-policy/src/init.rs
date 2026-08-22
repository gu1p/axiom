use std::{fs::OpenOptions, io::Write};

use policy_cargo::Workspace;

use crate::args::InitOptions;

const BASE_CONFIG: &str = r#"version = 1

[sources]
include = ["**/*.rs"]
exclude = []

[rules."size/function-max-lines"]
level = "deny"
limit = 50

[rules."size/file-max-lines"]
level = "deny"
limit = 200

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
level = "warn"

[rules."visibility/unnecessary-restricted"]
level = "warn"

[rules."visibility/unnecessary-crate"]
level = "warn"
"#;

const LIBRARY_SEMANTIC_EXAMPLE: &str = r#"
# Semantic analysis needs an explicit product boundary in a library-only workspace.
# Uncomment and update this declaration and the warning rules below.
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
# level = "warn"
# [rules."visibility/unnecessary-restricted"]
# level = "warn"
# [rules."visibility/unnecessary-crate"]
# level = "warn"
"#;

pub fn run(options: &InitOptions) -> u8 {
    let workspace = match Workspace::discover(options.manifest_path.as_deref()) {
        Ok(workspace) => workspace,
        Err(error) => {
            eprintln!("error: {error}");
            return 2;
        }
    };
    let path = workspace.policy_path();
    let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("error: could not create {path}: {error}");
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
        .and_then(|()| file.write_all(semantic.as_bytes()))
    {
        eprintln!("error: could not write {path}: {error}");
        return 2;
    }
    println!("created {path}");
    0
}
