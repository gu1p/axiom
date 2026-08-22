# Axiom

[![CI](https://github.com/gu1p/axiom/actions/workflows/ci.yml/badge.svg)](https://github.com/gu1p/axiom/actions/workflows/ci.yml)
[![Release](https://github.com/gu1p/axiom/actions/workflows/release.yml/badge.svg)](https://github.com/gu1p/axiom/actions/workflows/release.yml)

Axiom is a small executable-policy platform for Rust workspaces. Its first release turns source
size limits into facts, policies, and compiler-style diagnostics while keeping the analysis and
rule layers ready for architecture, dependency, complexity, and semantic checks.

## Usage

Install the latest release for macOS or Linux with:

```console
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/gu1p/axiom/main/install.sh | sh
```

The installer detects the operating system and CPU architecture, verifies the release checksum,
and installs `axiom` into `${CARGO_HOME:-$HOME/.cargo}/bin`. It adds that directory to the current
user's shell profile when necessary; restart the shell afterward.

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

This repository enforces its own configuration: functions may contain at most 50 physical lines and
Rust files at most 200 physical lines.

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

Each release includes archives and SHA-256 checksums for:

- Linux x86_64 and ARM64, linked statically with musl.
- macOS Intel and Apple Silicon, linked only against Apple system libraries. macOS does not support
  fully static user executables.

Download the archive for your target from the
[latest GitHub Release](https://github.com/gu1p/axiom/releases/latest), extract it, and place `axiom`
on `PATH`.

## Configuration

Rules are registered by stable IDs and configured independently:

```toml
version = 1

[sources]
include = ["**/*.rs"]
exclude = []

[rules."size/function-max-lines"]
level = "deny"
limit = 50

[rules."size/file-max-lines"]
level = "deny"
limit = 200
```

Levels are `allow`, `warn`, and `deny`. Omitted rules are disabled. Source globs are evaluated
against forward-slash workspace-relative paths; normal ignore files, `.git`, Cargo's target
directory, and directory symlinks are excluded from discovery.

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
- `policy-rules` converts configured policies into diagnostics.
- `cargo-policy` owns CLI commands and output renderers.

Future backends add facts through `FactProvider`; future policy families register `RuleFactory`
implementations. Neither requires another command or analysis platform.

## Development

Before finishing a change, run the complete formatting, Clippy, test, and self-policy suite:

```console
make check
```

Do not add or weaken exclusions to bypass a violation. Fix the underlying source-size issue.
