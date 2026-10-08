use texpose::{parse, ParseErrorKind};

#[test]
fn disallowed_ams_display_nesting_is_a_structured_parse_error() {
    for (source, nested_name) in [
        (r"\begin{split}x&=y\end{split}", "split"),
        (
            r"\begin{equation}\begin{align}x&=y\end{align}\end{equation}",
            "align",
        ),
        (
            r"\begin{align}\begin{gather}x\end{gather}\end{align}",
            "gather",
        ),
        (
            r"\begin{multline}\begin{split}x&=y\end{split}\end{multline}",
            "split",
        ),
        (
            r"\begin{array}{c}\begin{equation}x\end{equation}\end{array}",
            "equation",
        ),
        (
            r"\begin{aligned}\begin{equation}x\end{equation}\end{aligned}",
            "equation",
        ),
    ] {
        let error = parse(source).expect_err(source);
        assert_eq!(error.kind(), ParseErrorKind::MalformedMatrix, "{source}");
        let span_start =
            source.rfind(&format!(r"\begin{{{nested_name}}}")).unwrap() + r"\begin{".len();
        assert_eq!(error.span().start(), span_start, "{source}");
    }
}

#[test]
fn inner_ams_structures_and_matrices_remain_accepted() {
    for source in [
        r"\begin{equation}\begin{split}x&=y\\z&=w\end{split}\end{equation}",
        r"\begin{equation}\begin{aligned}x&=y\end{aligned}\end{equation}",
        r"\begin{gather}\begin{align}x&=y\end{align}\end{gather}",
        r"\begin{align}\begin{split}x&=y\end{split}\end{align}",
        r"\begin{aligned}\begin{matrix}x\end{matrix}&=y\end{aligned}",
        r"\begin{array}{c}\begin{matrix}x\end{matrix}\end{array}",
        r"\begin{equation}\begin{array}{c}x\end{array}\end{equation}",
    ] {
        parse(source).unwrap_or_else(|error| panic!("{source}: {error}"));
    }
}
