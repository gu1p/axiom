# Built-in lint profile

`[tools.clippy] profile = "axiom"` applies the following lint policy without requiring every
workspace to copy it into `Cargo.toml`. Names are shown in Cargo's workspace-lint syntax; Axiom
passes the equivalent ordered flags to Clippy and rustdoc.

The configured `warnings` policy is applied after the warn-level selections. With the default
`warnings = "deny"`, every lint shown as `warn` blocks `axiom check`. Explicit `allow` entries
remain allowed.

## Rust

```toml
[workspace.lints.rust]
unsafe_code = "deny"

future_incompatible = { level = "warn", priority = -1 }
nonstandard_style = { level = "warn", priority = -1 }
rust_2018_idioms = { level = "warn", priority = -1 }
trivial_numeric_casts = "warn"
unused_import_braces = "warn"
unused_lifetimes = "warn"

trivial_casts = "allow"
unused_qualifications = "allow"
```

## Rustdoc

```toml
[workspace.lints.rustdoc]
all = "warn"
```

## Clippy

```toml
[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
cargo = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }

allow_attributes = "warn"
as_ptr_cast_mut = "warn"
branches_sharing_code = "warn"
clear_with_drain = "warn"
clone_on_ref_ptr = "warn"
cognitive_complexity = "warn"
coerce_container_to_any = "warn"
dbg_macro = "warn"
debug_assert_with_mut_call = "warn"
default_union_representation = "warn"
derive_partial_eq_without_eq = "warn"
disallowed_script_idents = "warn"
doc_include_without_cfg = "warn"
empty_enum_variants_with_brackets = "warn"
equatable_if_let = "warn"
exit = "warn"
fallible_impl_from = "warn"
float_cmp_const = "warn"
fn_to_numeric_cast_any = "warn"
get_unwrap = "warn"
imprecise_flops = "warn"
infinite_loop = "warn"
iter_on_empty_collections = "warn"
iter_on_single_items = "warn"
iter_over_hash_type = "warn"
large_include_file = "warn"
large_stack_frames = "warn"
literal_string_with_formatting_args = "warn"
lossy_float_literal = "warn"
map_err_ignore = "warn"
mem_forget = "warn"
missing_assert_message = "warn"
mutex_integer = "warn"
needless_pass_by_ref_mut = "warn"
needless_type_cast = "warn"
non_zero_suggestions = "warn"
nonstandard_macro_braces = "warn"
or_fun_call = "warn"
path_buf_push_overwrite = "warn"
pathbuf_init_then_push = "warn"
precedence_bits = "warn"
print_stderr = "warn"
print_stdout = "warn"
pub_without_shorthand = "warn"
rc_mutex = "warn"
redundant_type_annotations = "warn"
ref_patterns = "warn"
rest_pat_in_fully_bound_structs = "warn"
return_and_then = "warn"
set_contains_or_insert = "warn"
single_option_map = "warn"
std_instead_of_core = "warn"
str_to_string = "warn"
string_add = "warn"
string_lit_as_bytes = "warn"
string_lit_chars_any = "warn"
suspicious_xor_used_as_pow = "warn"
todo = "warn"
too_long_first_doc_paragraph = "warn"
trailing_empty_array = "warn"
trait_duplication_in_bounds = "warn"
tuple_array_conversions = "warn"
undocumented_unsafe_blocks = "warn"
unimplemented = "warn"
uninhabited_references = "warn"
unnecessary_safety_comment = "warn"
unnecessary_safety_doc = "warn"
unnecessary_self_imports = "warn"
unnecessary_struct_initialization = "warn"
unused_peekable = "warn"
unused_rounding = "warn"
unused_trait_names = "warn"
unwrap_used = "warn"
use_self = "warn"
useless_let_if_seq = "warn"
verbose_file_reads = "warn"

cast_lossless = "allow"
cast_possible_truncation = "allow"
cast_possible_wrap = "allow"
cast_precision_loss = "allow"
cast_sign_loss = "allow"
comparison_chain = "allow"
default_trait_access = "allow"
float_cmp = "allow"
inline_always = "allow"
items_after_statements = "allow"
many_single_char_names = "allow"
missing_panics_doc = "allow"
must_use_candidate = "allow"
redundant_closure_for_method_calls = "allow"
return_self_not_must_use = "allow"
should_panic_without_expect = "allow"
similar_names = "allow"
struct_excessive_bools = "allow"
struct_field_names = "allow"
too_many_lines = "allow"
trivially_copy_pass_by_ref = "allow"
unreadable_literal = "allow"
used_underscore_binding = "allow"

assigning_clones = "allow"
manual_range_contains = "allow"
map_unwrap_or = "allow"
multiple_crate_versions = "allow"
wildcard_imports = "allow"

let_underscore_must_use = "allow"
let_underscore_untyped = "allow"
self_named_module_files = "allow"
significant_drop_tightening = "allow"
```

`cognitive_complexity` is the current name of Clippy's former `cyclomatic_complexity` lint;
current Clippy does not provide two independent metrics. Its limit defaults to `25`; this
repository sets it to `12` in the root `clippy.toml`. A workspace can choose its own limit in
`clippy.toml` or `.clippy.toml`:

```toml
cognitive-complexity-threshold = 12
```

An item-level `clippy::cognitive_complexity` attribute can override the limit locally, but a
workspace-wide configuration is preferable when the intended policy is general.

Use `profile = "workspace"` when a repository intentionally owns a different complete profile in
its Cargo manifests. This switches off Axiom's selections rather than merging two competing
profiles.

Per-lint entries under `[tools.clippy.lints]` are applied after this profile and the global
`warnings` policy. Use the exact lint code printed by Axiom:

```toml
[tools.clippy.lints]
# Possible values: "deny" (error), "warn" (warning), "allow" (disabled).
"clippy::unwrap_used" = "deny"
# Possible values: "deny" (error), "warn" (warning), "allow" (disabled).
"clippy::needless_return" = "allow"
# Possible values: "deny" (error), "warn" (warning), "allow" (disabled).
"rustdoc::broken_intra_doc_links" = "warn"
```

The supported values are `deny` (error), `warn` (non-blocking warning), and `allow` (disabled).
