# Axiom

[![CI](https://github.com/gu1p/axiom/actions/workflows/ci.yml/badge.svg)](https://github.com/gu1p/axiom/actions/workflows/ci.yml)
[![Release](https://github.com/gu1p/axiom/actions/workflows/release.yml/badge.svg)](https://github.com/gu1p/axiom/actions/workflows/release.yml)

Axiom is an executable-policy platform for Rust workspaces. It combines source facts, rustc/HIR
semantics, workspace reachability, and small declarative policies behind one compiler-like command.

## Usage

Install the latest release for macOS or Linux with:

```console
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/gu1p/axiom/main/install.sh | sh
```

The installer detects the operating system and CPU architecture, verifies the release checksum,
and installs `axiom` plus its private semantic compiler driver into
`${CARGO_HOME:-$HOME/.cargo}/bin`. It adds that directory to the current user's shell profile when
necessary; restart the shell afterward. Only `axiom` is a public command.

To build and install from source instead:

```console
make build
make install
```

`make install` installs into `${CARGO_HOME:-$HOME/.cargo}/bin` without administrator privileges. It
also adds that directory to `PATH` in the current user's zsh, bash, fish, or POSIX shell profile.
Restart the shell afterward. Managed environments can override both locations:

```console
make install INSTALL_ROOT="$HOME/.local" PROFILE="$HOME/.profile"
```

Once installed:

```console
axiom init
axiom check
axiom --version
```

The `cargo policy init` and `cargo policy check` aliases remain available for Cargo-oriented use.

This repository enforces its own size, test-placement, dead-code, and visibility policies.

Useful options:

```console
axiom check --manifest-path path/to/Cargo.toml
axiom check --config path/to/policy.toml
axiom check --format json
axiom check --color never
```

Exit code `0` means the check completed without deny-level findings, `1` means policies were
violated, and `2` means configuration, discovery, I/O, UTF-8, or parsing prevented a complete check.

## Releases

Every successful push to `main` automatically creates the next `v0.1.x` patch tag and publishes a
GitHub Release. Release builds inject the tag into the executable, so `axiom --version` reports the
exact released version.

Each release includes `axiom`, its matching `axiom-hir-driver`, and SHA-256 checksums for:

- Linux x86_64 and ARM64, linked statically with musl.
- macOS Intel and Apple Silicon, linked only against Apple system libraries. macOS does not support
  fully static user executables.

Download the archive for your target from the
[latest GitHub Release](https://github.com/gu1p/axiom/releases/latest), extract it, and place both
executables in the same directory on `PATH`.

## Configuration

Rules are registered by stable IDs and configured independently:

```toml
version = 1

[sources]
include = ["**/*.rs"]
exclude = []
test = ["**/tests.rs", "**/*_test.rs", "**/*_tests.rs", "**/tests/**/*.rs"]

[rules."size/function-max-lines"]
level = "deny"
limit = 50
scope = "production"

[rules."size/file-max-lines"]
level = "deny"
limit = 200
scope = "production"

[rules."testing/separate-test-files"]
level = "deny"

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
```

Levels are `allow`, `warn`, and `deny`. Omitted rules are disabled. Every rule also accepts a
`scope` of `all` (the default), `production`, or `test`. A production-scoped rule ignores findings
inside dedicated test files, test-attributed functions, and inline `#[cfg(test)]` items. This lets
production functions keep a strict line budget without imposing the same budget on test setup:

```toml
[rules."size/function-max-lines"]
level = "deny"
limit = 50
scope = "production"
```

Source globs are evaluated against forward-slash workspace-relative paths; normal ignore files,
`.git`, Cargo's target directory, and directory symlinks are excluded from discovery.

`testing/separate-test-files` rejects inline `#[cfg(test)]` implementations and test-attributed
functions in production files. External test module declarations remain valid, so private unit
tests can be placed in separate files:

```rust
#[cfg(test)]
#[path = "tests/order.rs"]
mod tests;
```

By default, files under a `tests/` directory and files named `tests.rs`, `*_test.rs`, or
`*_tests.rs` are classified as test sources. Override those conventions for every scoped rule with
the `[sources].test` glob list, for example `test = ["**/specs/**/*.rs"]`.

## Semantic analysis

The semantic rules analyze the complete Cargo workspace rather than one crate at a time:

- `dead-code/private` reports rustc's ordinary private `dead_code` findings and respects
  `#[allow(dead_code)]`.
- `dead-code/public` finds public declarations unreachable from the configured products.
- `dead-code/test-only` finds production declarations reachable only from tests and other
  non-production targets.
- The three `visibility/*` rules find declarations that can safely become `pub(crate)`, private,
  or `pub(super)`.

Binary workspaces need no semantic configuration: every workspace binary is a production root. A
library-only workspace must declare the closed-world product explicitly:

```toml
[[semantic.production]]
package = "billing"
lib = "billing"
reason = "internal library consumed only by this workspace"
```

Exactly one of `bin` or `lib` is required per production entry. `[semantic]` also accepts
`preserve-uniform-field-visibility`, `exclude-crates`, and nested `feature-profile`, `doctest`,
`override`, and `exclude` entries. Exceptions require a reason and use Axiom rule IDs:

```toml
[[semantic.override]]
rule = "dead-code/public"
crate = "billing"
item = "legacy_entry"
level = "expect"
reason = "kept until the legacy consumer is migrated"
```

Semantic analysis uses the compiler-coupled Rust 1.98.0 driver included with Axiom. On first use,
Axiom installs the matching minimal toolchain through rustup if it is missing. Set
`AXIOM_OFFLINE=1` to forbid network access; Axiom then reports the exact provisioning command.
Building Axiom itself also requires the `rustc-dev` component, which its `rust-toolchain.toml`
selects automatically.

The public Linux Axiom executable remains statically linked and works on musl. Semantic rules
currently require glibc because the private driver dynamically loads Rust compiler libraries; on a
musl-only host, disable semantic rules or run Axiom in a glibc environment.

## Physical-line definition

- Empty files contain zero lines.
- LF and CRLF each terminate one line; a final newline does not create another line.
- File rules include every physical line in the file.
- Function rules cover attached attributes and documentation through the closing brace or
  declaration semicolon, including signatures, blank lines, and comments.
- Free, associated, trait, extern, nested, and inactive-`cfg` functions are checked.
- Macro invocations are not expanded.

JSON output is a deterministic document with `schema_version = 1`, an outcome, diagnostics, and a
summary. Span byte offsets are zero-based; line and Unicode-scalar columns are one-based.

## Architecture

The workspace deliberately separates responsibilities:

- `policy-core` owns facts, source spans, provider/rule contracts, the registry, and engine.
- `policy-cargo` discovers a workspace and its source inputs.
- `policy-syntax` contributes lossless syntax facts.
- `policy-semantic` owns the absorbed workspace graph, HIR collector, and private compiler driver.
- `policy-rules` converts configured policies into diagnostics.
- `cargo-policy` owns CLI commands and output renderers.

Backends add facts through `FactProvider`; policy families register `RuleFactory` implementations.
Neither requires another public command or analysis platform.

## Development

Before finishing a change, run the complete formatting, Clippy, test, and self-policy suite:

```console
make check
```

Do not add or weaken exclusions to bypass a violation. Fix the underlying source-size issue.
