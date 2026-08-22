#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_lexer;
extern crate rustc_lint_defs;
extern crate rustc_middle;
extern crate rustc_parse;
extern crate rustc_session;
extern crate rustc_span;

use policy_semantic::protocol;

mod driver;

fn main() -> std::process::ExitCode {
    let Ok(args): Result<Vec<String>, _> = std::env::args_os()
        .map(std::ffi::OsString::into_string)
        .collect()
    else {
        eprintln!("hawk: command-line arguments must be valid UTF-8");
        return std::process::ExitCode::FAILURE;
    };
    if driver::is_protocol_version_query(&args) {
        driver::print_protocol_version()
    } else if driver::is_wrapper_invocation(&args) {
        driver::run_wrapper(args)
    } else {
        eprintln!("axiom: axiom-hir-driver is an internal compiler wrapper");
        std::process::ExitCode::FAILURE
    }
}
