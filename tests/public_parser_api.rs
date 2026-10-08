//! Public parser boundary: crate-root entry points, read-only values, and non-exhaustive enums.

use texpose::{
    parse, parse_with_options, AccentKind, AtomKind, ColSpec, DelimSize, Delimiter, EnvRow,
    EqNumber, FractionAlignment, FractionRule, FractionSpec, FractionStyle, IntegralKind, Length,
    LimitMode, MathNode, MathStyleDeclaration, MatrixStyle, ParseErrorDetail, ParseErrorKind,
    ParseOptions, ParseResource, PhantomKind, SpaceKind, TextStyle, DEFAULT_MAX_AST_NODES,
    DEFAULT_MAX_ENVIRONMENT_CELLS, DEFAULT_MAX_ENVIRONMENT_ROWS, DEFAULT_MAX_NESTING_DEPTH,
    DEFAULT_MAX_TOKENS,
};

#[test]
fn supported_ast_types_are_root_reexported() {
    fn public<T>() {}

    public::<AccentKind>();
    public::<AtomKind>();
    public::<ColSpec>();
    public::<DelimSize>();
    public::<Delimiter>();
    public::<EnvRow>();
    public::<EqNumber>();
    public::<FractionAlignment>();
    public::<FractionRule>();
    public::<FractionSpec>();
    public::<FractionStyle>();
    public::<IntegralKind>();
    public::<Length>();
    public::<LimitMode>();
    public::<MathNode>();
    public::<MathStyleDeclaration>();
    public::<MatrixStyle>();
    public::<PhantomKind>();
    public::<SpaceKind>();
    public::<TextStyle>();
    let fraction = FractionSpec::ordinary(
        MathNode::Atom('1', AtomKind::Ord),
        MathNode::Atom('2', AtomKind::Ord),
    );
    let _ = MathNode::Fraction(fraction);
    let _ = EnvRow::cells(vec![MathNode::Atom('x', AtomKind::Ord)]);
}

#[test]
fn parse_options_are_configured_and_inspected_without_public_fields() {
    let defaults = ParseOptions::new();
    assert_eq!(defaults.max_depth(), DEFAULT_MAX_NESTING_DEPTH);
    assert_eq!(defaults.max_ast_nodes(), DEFAULT_MAX_AST_NODES);
    assert_eq!(
        defaults.max_environment_rows(),
        DEFAULT_MAX_ENVIRONMENT_ROWS
    );
    assert_eq!(
        defaults.max_environment_cells(),
        DEFAULT_MAX_ENVIRONMENT_CELLS
    );
    assert_eq!(defaults.max_tokens(), DEFAULT_MAX_TOKENS);

    let options = defaults
        .with_max_depth(48)
        .with_max_ast_nodes(2_048)
        .with_max_environment_rows(128)
        .with_max_environment_cells(512)
        .with_max_tokens(4_096);
    assert_eq!(options.max_depth(), 48);
    assert_eq!(options.max_ast_nodes(), 2_048);
    assert_eq!(options.max_environment_rows(), 128);
    assert_eq!(options.max_environment_cells(), 512);
    assert_eq!(options.max_tokens(), 4_096);

    let ast: MathNode = parse_with_options(r"x^2", &options).expect("parse");
    assert!(matches!(ast, MathNode::Superscript(_, _)));
}

#[test]
fn parser_errors_expose_typed_read_only_provenance() {
    let error = parse_with_options("αβ", &ParseOptions::new().with_max_tokens(1))
        .expect_err("second token exceeds the configured budget");
    assert_eq!(error.kind(), ParseErrorKind::ResourceLimit);

    let span = error.span();
    assert_eq!((span.start(), span.end(), span.len()), (2, 4, 2));
    assert!(!span.is_empty());

    match error.detail() {
        ParseErrorDetail::ResourceLimit { resource, limit } => {
            assert_eq!(*resource, ParseResource::Tokens);
            assert_eq!(*limit, 1);
        }
        _ => panic!("unexpected resource-limit detail: {:?}", error.detail()),
    }
}

#[test]
fn parse_local_color_definitions_are_resolved_into_the_ast() {
    let ast = parse(r"\definecolor{ok}{named}{red}\textcolor{ok}{x}").expect("parse color");
    match ast {
        MathNode::TextColor(color, body) => {
            assert_eq!(color.css_hex(), "#ff0000");
            assert!(matches!(body.as_ref(), MathNode::Atom('x', _)));
        }
        _ => panic!("expected resolved text color"),
    }
}

#[test]
fn public_ast_is_typed_and_requires_future_variant_fallbacks() {
    let ast = parse(r"\frac{1}{2}").expect("parse");
    match ast {
        MathNode::Fraction(spec) => {
            assert!(matches!(spec.numerator.as_ref(), MathNode::Atom('1', _)));
            assert!(matches!(spec.denominator.as_ref(), MathNode::Atom('2', _)));
        }
        _ => panic!("expected fraction"),
    }
}

#[test]
fn parser_module_and_test_helpers_do_not_leak_from_the_crate_root() {
    let lib = include_str!("../src/lib.rs");
    assert!(!lib.lines().any(|line| line.trim() == "pub mod parser;"));

    let export_start = lib.find("pub use parser::{").expect("parser re-export");
    let export_tail = &lib[export_start..];
    let export_end = export_tail.find("};").expect("parser re-export end");
    let public_export = &export_tail[..export_end];
    for removed in [
        "parse_with_colors",
        "preprocess",
        "format_tokens",
        " tokenize,",
        "tokenize_spanned",
        " SpannedToken",
        " Token",
    ] {
        assert!(
            !public_export.contains(removed),
            "removed parser implementation helper leaked from crate root: {removed}"
        );
    }
}
