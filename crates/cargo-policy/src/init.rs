use std::{fs::OpenOptions, io::Write};

use policy_cargo::Workspace;

use crate::args::InitOptions;

const DEFAULT_CONFIG: &str = r#"version = 1

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
    if let Err(error) = file.write_all(DEFAULT_CONFIG.as_bytes()) {
        eprintln!("error: could not write {path}: {error}");
        return 2;
    }
    println!("created {path}");
    0
}
