use std::ffi::OsString;

use super::normalized_args;

#[test]
fn strips_cargo_subcommand_argument() {
    let args = ["cargo-policy", "policy", "check"].map(OsString::from);
    assert_eq!(normalized_args(args), ["cargo-policy", "check"]);
}

#[test]
fn preserves_direct_arguments() {
    let args = ["axiom", "check"].map(OsString::from);
    assert_eq!(normalized_args(args.clone()), args);
}
