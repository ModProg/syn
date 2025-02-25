#![allow(clippy::manual_assert)]

mod progress;

use self::progress::Progress;
use anyhow::Result;
use flate2::read::GzDecoder;
use std::fs;
use std::path::Path;
use tar::Archive;
use walkdir::DirEntry;

const REVISION: &str = "afaf3e07aaa7ca9873bdb439caec53faffa4230c";

#[rustfmt::skip]
static EXCLUDE_FILES: &[&str] = &[
    // Compile-fail expr parameter in const generic position: f::<1 + 2>()
    "tests/ui/const-generics/early/closing-args-token.rs",
    "tests/ui/const-generics/early/const-expression-parameter.rs",

    // Compile-fail variadics in not the last position of a function parameter list
    "tests/ui/parser/variadic-ffi-syntactic-pass.rs",

    // Need at least one trait in impl Trait, no such type as impl 'static
    "tests/ui/type-alias-impl-trait/generic_type_does_not_live_long_enough.rs",

    // Lifetime bound inside for<>: `T: ~const ?for<'a: 'b> Trait<'a>`
    "tests/ui/rfc-2632-const-trait-impl/tilde-const-syntax.rs",

    // Const impl that is not a trait impl: `impl ~const T {}`
    "tests/ui/rfc-2632-const-trait-impl/syntax.rs",

    // Deprecated anonymous parameter syntax in traits
    "src/tools/rustfmt/tests/source/trait.rs",
    "src/tools/rustfmt/tests/target/trait.rs",
    "tests/ui/issues/issue-13105.rs",
    "tests/ui/issues/issue-13775.rs",
    "tests/ui/issues/issue-34074.rs",
    "tests/ui/proc-macro/trait-fn-args-2015.rs",

    // Deprecated where-clause location
    "src/tools/rustfmt/tests/source/issue_4257.rs",
    "src/tools/rustfmt/tests/source/issue_4911.rs",
    "src/tools/rustfmt/tests/target/issue_4257.rs",
    "src/tools/rustfmt/tests/target/issue_4911.rs",
    "tests/pretty/gat-bounds.rs",
    "tests/rustdoc/generic-associated-types/gats.rs",

    // Old type ascription expression syntax
    "src/tools/rustfmt/tests/source/type-ascription.rs",
    "src/tools/rustfmt/tests/target/type-ascription.rs",

    // Invalid unparenthesized range pattern inside slice pattern: `[1..]`
    "tests/ui/consts/miri_unleashed/const_refers_to_static_cross_crate.rs",

    // Various extensions to Rust syntax made up by rust-analyzer
    "src/tools/rust-analyzer/crates/parser/test_data/parser/inline/ok/0012_type_item_where_clause.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/inline/ok/0040_crate_keyword_vis.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/inline/ok/0058_range_pat.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/inline/ok/0123_param_list_vararg.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/inline/ok/0131_existential_type.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/inline/ok/0156_fn_def_param.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/inline/ok/0179_use_tree_abs_star.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/inline/ok/0188_const_param_default_path.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/ok/0015_use_tree.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/ok/0029_range_forms.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/ok/0051_parameter_attrs.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/ok/0055_dot_dot_dot.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/ok/0068_item_modifiers.rs",
    "src/tools/rust-analyzer/crates/syntax/test_data/parser/validation/0031_block_inner_attrs.rs",
    "src/tools/rust-analyzer/crates/syntax/test_data/parser/validation/0038_endless_inclusive_range.rs",
    "src/tools/rust-analyzer/crates/syntax/test_data/parser/validation/0045_ambiguous_trait_object.rs",
    "src/tools/rust-analyzer/crates/syntax/test_data/parser/validation/0046_mutable_const_item.rs",

    // Placeholder syntax for "throw expressions"
    "compiler/rustc_errors/src/translation.rs",
    "src/tools/clippy/tests/ui/needless_return.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/inline/ok/0204_yeet_expr.rs",
    "tests/pretty/yeet-expr.rs",
    "tests/ui/try-trait/yeet-for-option.rs",
    "tests/ui/try-trait/yeet-for-result.rs",

    // Edition 2015 code using identifiers that are now keywords
    // TODO: some of these we should probably parse
    "src/tools/rust-analyzer/crates/parser/test_data/parser/inline/ok/0159_try_macro_fallback.rs",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/inline/ok/0160_try_macro_rules.rs",
    "src/tools/rustfmt/tests/source/configs/indent_style/block_call.rs",
    "src/tools/rustfmt/tests/source/configs/use_try_shorthand/false.rs",
    "src/tools/rustfmt/tests/source/configs/use_try_shorthand/true.rs",
    "src/tools/rustfmt/tests/source/try-conversion.rs",
    "src/tools/rustfmt/tests/target/configs/indent_style/block_call.rs",
    "src/tools/rustfmt/tests/target/configs/use_try_shorthand/false.rs",
    "src/tools/rustfmt/tests/target/issue-1681.rs",
    "tests/ui/dyn-keyword/dyn-2015-no-warnings-without-lints.rs",
    "tests/ui/editions/edition-keywords-2015-2015.rs",
    "tests/ui/editions/edition-keywords-2015-2018.rs",
    "tests/ui/lint/lint_pre_expansion_extern_module_aux.rs",
    "tests/ui/macros/macro-comma-support-rpass.rs",
    "tests/ui/macros/try-macro.rs",
    "tests/ui/parser/extern-crate-async.rs",
    "tests/ui/try-block/try-is-identifier-edition2015.rs",

    // Excessive nesting
    "tests/ui/issues/issue-74564-if-expr-stack-overflow.rs",

    // Testing tools on invalid syntax
    "src/tools/rustfmt/tests/coverage/target/comments.rs",
    "src/tools/rustfmt/tests/parser/issue-4126/invalid.rs",
    "src/tools/rustfmt/tests/parser/issue_4418.rs",
    "src/tools/rustfmt/tests/parser/unclosed-delims/issue_4466.rs",
    "src/tools/rustfmt/tests/source/configs/disable_all_formatting/true.rs",
    "src/tools/rustfmt/tests/source/configs/spaces_around_ranges/false.rs",
    "src/tools/rustfmt/tests/source/configs/spaces_around_ranges/true.rs",
    "src/tools/rustfmt/tests/source/type.rs",
    "src/tools/rustfmt/tests/target/configs/spaces_around_ranges/false.rs",
    "src/tools/rustfmt/tests/target/configs/spaces_around_ranges/true.rs",
    "src/tools/rustfmt/tests/target/type.rs",
    "tests/run-make/translation/test.rs",
    "tests/ui/generics/issue-94432-garbage-ice.rs",

    // Generated file containing a top-level expression, used with `include!`
    "compiler/rustc_codegen_gcc/src/intrinsic/archs.rs",

    // Clippy lint lists represented as expressions
    "src/tools/clippy/clippy_lints/src/lib.deprecated.rs",

    // Not actually test cases
    "tests/ui/lint/expansion-time-include.rs",
    "tests/ui/macros/auxiliary/macro-comma-support.rs",
    "tests/ui/macros/auxiliary/macro-include-items-expr.rs",
    "tests/ui/macros/include-single-expr-helper.rs",
    "tests/ui/macros/include-single-expr-helper-1.rs",
    "tests/ui/parser/issues/auxiliary/issue-21146-inc.rs",
];

#[rustfmt::skip]
static EXCLUDE_DIRS: &[&str] = &[
    // Inputs that intentionally do not parse
    "src/tools/rust-analyzer/crates/parser/test_data/parser/err",
    "src/tools/rust-analyzer/crates/parser/test_data/parser/inline/err",

    // Inputs that lex but do not necessarily parse
    "src/tools/rust-analyzer/crates/parser/test_data/lexer",

    // Inputs that used to crash rust-analyzer, but aren't necessarily supposed to parse
    "src/tools/rust-analyzer/crates/syntax/test_data/parser/fuzz-failures",
    "src/tools/rust-analyzer/crates/syntax/test_data/reparse/fuzz-failures",
];

pub fn base_dir_filter(entry: &DirEntry) -> bool {
    let path = entry.path();

    let mut path_string = path.to_string_lossy();
    if cfg!(windows) {
        path_string = path_string.replace('\\', "/").into();
    }
    let path_string = if path_string == "tests/rust" {
        return true;
    } else if let Some(path) = path_string.strip_prefix("tests/rust/") {
        path
    } else {
        panic!("unexpected path in Rust dist: {}", path_string);
    };

    if path.is_dir() {
        return !EXCLUDE_DIRS.contains(&path_string);
    }

    if path.extension().map_or(true, |e| e != "rs") {
        return false;
    }

    if path_string.starts_with("tests/ui") || path_string.starts_with("tests/rustdoc-ui") {
        let stderr_path = path.with_extension("stderr");
        if stderr_path.exists() {
            // Expected to fail in some way
            return false;
        }
    }

    !EXCLUDE_FILES.contains(&path_string)
}

#[allow(dead_code)]
pub fn edition(path: &Path) -> &'static str {
    if path.ends_with("dyn-2015-no-warnings-without-lints.rs") {
        "2015"
    } else {
        "2018"
    }
}

pub fn clone_rust() {
    let needs_clone = match fs::read_to_string("tests/rust/COMMIT") {
        Err(_) => true,
        Ok(contents) => contents.trim() != REVISION,
    };
    if needs_clone {
        download_and_unpack().unwrap();
    }
    let mut missing = String::new();
    let test_src = Path::new("tests/rust");
    for exclude in EXCLUDE_FILES {
        if !test_src.join(exclude).is_file() {
            missing += "\ntests/rust/";
            missing += exclude;
        }
    }
    for exclude in EXCLUDE_DIRS {
        if !test_src.join(exclude).is_dir() {
            missing += "\ntests/rust/";
            missing += exclude;
            missing += "/";
        }
    }
    if !missing.is_empty() {
        panic!("excluded test file does not exist:{}\n", missing);
    }
}

fn download_and_unpack() -> Result<()> {
    let url = format!(
        "https://github.com/rust-lang/rust/archive/{}.tar.gz",
        REVISION
    );
    let response = reqwest::blocking::get(url)?.error_for_status()?;
    let progress = Progress::new(response);
    let decoder = GzDecoder::new(progress);
    let mut archive = Archive::new(decoder);
    let prefix = format!("rust-{}", REVISION);

    let tests_rust = Path::new("tests/rust");
    if tests_rust.exists() {
        fs::remove_dir_all(tests_rust)?;
    }

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if path == Path::new("pax_global_header") {
            continue;
        }
        let relative = path.strip_prefix(&prefix)?;
        let out = tests_rust.join(relative);
        entry.unpack(&out)?;
    }

    fs::write("tests/rust/COMMIT", REVISION)?;
    Ok(())
}
