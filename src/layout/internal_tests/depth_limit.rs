//! Parser resource budgets: recursion stays stack-safe and hostile input is bounded.

use super::common;

use crate::test_support::{
    layout, layout_with_max_depth, parse, parse_with_options, MathStyle, ParseOptions,
    DEFAULT_MAX_AST_NODES, DEFAULT_MAX_ENVIRONMENT_CELLS, DEFAULT_MAX_ENVIRONMENT_ROWS,
    DEFAULT_MAX_NESTING_DEPTH, DEFAULT_MAX_TOKENS,
};

/// A named nesting shape that builds input `n` levels deep.
type Shape = (&'static str, fn(usize) -> String);

/// Nesting shapes, each building input `n` levels deep.
fn shapes() -> Vec<Shape> {
    vec![
        ("braces", |n| "{".repeat(n) + "x" + &"}".repeat(n)),
        ("frac", |n| "\\frac{1}{".repeat(n) + "2" + &"}".repeat(n)),
        ("sqrt", |n| "\\sqrt{".repeat(n) + "x" + &"}".repeat(n)),
        ("sup", |n| "x^{".repeat(n) + "y" + &"}".repeat(n)),
        ("left", |n| {
            "\\left(".repeat(n) + "x" + &"\\right)".repeat(n)
        }),
        ("mathrm", |n| "\\mathrm{".repeat(n) + "x" + &"}".repeat(n)),
    ]
}

/// Deepest `n` for which `make(n)` parses under `opts`.
fn deepest(make: fn(usize) -> String, opts: &ParseOptions) -> usize {
    let mut n = 0;
    while parse_with_options(&make(n + 1), opts).is_ok() {
        n += 1;
        assert!(n < 10_000, "limit never reached");
    }
    n
}

/// A deliberately small stack used to exercise the parser/layout depth budget.
const SMALL_STACK: usize = if cfg!(debug_assertions) {
    2 << 20
} else {
    1 << 20
};

fn on_small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("worker panicked");
}

#[test]
fn deepest_default_input_parses_and_lays_out_on_small_stack() {
    on_small_stack(|| {
        let font = common::stix_two_math().expect("font");
        let opts = ParseOptions::default();
        for (name, make) in shapes() {
            let n = deepest(make, &opts);
            assert!(n >= 5, "{name}: default admits only {n} levels");
            eprintln!("{name}: default admits {n} levels");
            let ast = parse(&make(n)).unwrap_or_else(|e| panic!("{name}@{n}: {e}"));
            layout(&ast, &font, MathStyle::Display)
                .unwrap_or_else(|e| panic!("{name}@{n}: parse accepted but layout refused: {e}"));
        }
    });
}

#[test]
fn pathological_input_errs_instead_of_aborting_on_small_stack() {
    on_small_stack(|| {
        for (name, make) in shapes() {
            for n in [DEFAULT_MAX_NESTING_DEPTH + 1, 1_000, 100_000] {
                assert!(parse(&make(n)).is_err(), "{name}@{n} should be refused");
            }
        }
    });
}

#[test]
fn caller_can_raise_and_lower_the_limit() {
    // A raised limit needs a caller that has the stack for it, as the docs say.
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let font = common::stix_two_math().expect("font");
            let tight = ParseOptions::new().with_max_depth(8);
            let roomy = ParseOptions::new().with_max_depth(128);
            for (name, make) in shapes() {
                let d_tight = deepest(make, &tight);
                let d_default = deepest(make, &ParseOptions::default());
                let d_roomy = deepest(make, &roomy);
                assert!(
                    d_tight < d_default && d_default < d_roomy,
                    "{name}: {d_tight} {d_default} {d_roomy}"
                );
                let (ast, _) = parse_with_options(&make(d_roomy), &roomy).expect("roomy parse");
                assert!(
                    layout_with_max_depth(&ast, &font, MathStyle::Display, 128).is_ok(),
                    "{name}@{d_roomy}: roomy layout"
                );
            }
        })
        .expect("spawn")
        .join()
        .expect("worker panicked");
}

#[test]
fn default_resource_budgets_are_stable() {
    assert_eq!(DEFAULT_MAX_NESTING_DEPTH, 32);
    assert_eq!(DEFAULT_MAX_AST_NODES, 65_536);
    assert_eq!(DEFAULT_MAX_ENVIRONMENT_ROWS, 4_096);
    assert_eq!(DEFAULT_MAX_ENVIRONMENT_CELLS, 16_384);
    assert_eq!(DEFAULT_MAX_TOKENS, 131_072);

    let defaults = ParseOptions::default();
    assert_eq!(defaults.max_depth, DEFAULT_MAX_NESTING_DEPTH);
    assert_eq!(defaults.max_ast_nodes, DEFAULT_MAX_AST_NODES);
    assert_eq!(defaults.max_environment_rows, DEFAULT_MAX_ENVIRONMENT_ROWS);
    assert_eq!(
        defaults.max_environment_cells,
        DEFAULT_MAX_ENVIRONMENT_CELLS
    );
    assert_eq!(defaults.max_tokens, DEFAULT_MAX_TOKENS);
}

fn assert_resource_limit(
    source: &str,
    options: &ParseOptions,
    resource: texpose::ParseResource,
    limit: usize,
) {
    let error = parse_with_options(source, options).expect_err(source);
    assert_eq!(
        error.kind(),
        texpose::ParseErrorKind::ResourceLimit,
        "{error}"
    );
    assert!(
        matches!(
            error.detail(),
            texpose::ParseErrorDetail::ResourceLimit {
                resource: got,
                limit: got_limit,
            } if *got == resource && *got_limit == limit
        ),
        "{source}: {error:?}"
    );
}

#[test]
fn parser_resource_budgets_cover_hostile_shapes() {
    let long_stream = "x".repeat(4_096);
    assert_resource_limit(
        &long_stream,
        &ParseOptions::new().with_max_tokens(1_024),
        texpose::ParseResource::Tokens,
        1_024,
    );

    let deep_groups = "{".repeat(64) + "x" + &"}".repeat(64);
    assert_resource_limit(
        &deep_groups,
        &ParseOptions::new().with_max_depth(8),
        texpose::ParseResource::NestingDepth,
        8,
    );

    let deep_fraction = "\\frac{1}{".repeat(24) + "2" + &"}".repeat(24);
    assert_resource_limit(
        &deep_fraction,
        &ParseOptions::new().with_max_depth(8),
        texpose::ParseResource::NestingDepth,
        8,
    );

    let wide_matrix = format!(
        r"\begin{{matrix}}{}\end{{matrix}}",
        (0..64).map(|_| "x").collect::<Vec<_>>().join("&")
    );
    assert_resource_limit(
        &wide_matrix,
        &ParseOptions::new().with_max_environment_cells(16),
        texpose::ParseResource::EnvironmentCells,
        16,
    );

    let many_rows = format!(
        r"\begin{{matrix}}{}\end{{matrix}}",
        (0..64).map(|_| "x").collect::<Vec<_>>().join(r"\\")
    );
    assert_resource_limit(
        &many_rows,
        &ParseOptions::new().with_max_environment_rows(16),
        texpose::ParseResource::EnvironmentRows,
        16,
    );

    let repeated_scripts = (0..128).map(|_| "x^1").collect::<Vec<_>>().join(" ");
    assert_resource_limit(
        &repeated_scripts,
        &ParseOptions::new().with_max_ast_nodes(128),
        texpose::ParseResource::AstNodes,
        128,
    );
}

#[test]
fn exact_resource_boundaries_accept_limit_and_reject_next_unit() {
    let ast_source = "xy";
    assert!(parse_with_options(ast_source, &ParseOptions::new().with_max_ast_nodes(3),).is_ok());
    assert_resource_limit(
        ast_source,
        &ParseOptions::new().with_max_ast_nodes(2),
        texpose::ParseResource::AstNodes,
        2,
    );

    let matrix = r"\begin{matrix}x&y\end{matrix}";
    assert!(parse_with_options(
        matrix,
        &ParseOptions::new()
            .with_max_environment_rows(1)
            .with_max_environment_cells(2),
    )
    .is_ok());
    assert_resource_limit(
        matrix,
        &ParseOptions::new().with_max_environment_cells(1),
        texpose::ParseResource::EnvironmentCells,
        1,
    );

    assert!(parse_with_options("αβ", &ParseOptions::new().with_max_tokens(2)).is_ok());
    let error = parse_with_options("αβ", &ParseOptions::new().with_max_tokens(1))
        .expect_err("second UTF-8 token must exceed budget");
    assert_eq!(error.kind(), texpose::ParseErrorKind::ResourceLimit);
    assert_eq!(error.span(), texpose::SourceSpan { start: 2, end: 4 });
}
