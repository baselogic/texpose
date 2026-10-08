use texpose::{parse, ParseErrorKind};

#[test]
fn split_cannot_share_its_enclosing_row_with_math_material() {
    let split = r"\begin{split}x&=y\\z&=w\end{split}";
    for source in [
        format!(r"\begin{{equation}}a{split}\end{{equation}}"),
        format!(r"\begin{{equation}}{split}+b\end{{equation}}"),
        format!(r"\begin{{equation}}\frac{{{split}}}{{2}}\end{{equation}}"),
        format!(r"\begin{{equation}}\left({split}\right)\end{{equation}}"),
        format!(r"\begin{{gather}}a{split}\end{{gather}}"),
        format!(r"\begin{{align}}a&{split}\end{{align}}"),
        format!(r"\begin{{align}}{split}&b\end{{align}}"),
        format!(r"\begin{{equation}}{split}{split}\end{{equation}}"),
    ] {
        let error = parse(&source).expect_err(&source);
        assert_eq!(error.kind(), ParseErrorKind::MalformedMatrix, "{source}");
        let split_name = source.rfind(r"\begin{split}").unwrap() + r"\begin{".len();
        assert_eq!(error.span().start(), split_name, "{source}");
    }
}

#[test]
fn split_occupying_a_row_and_nonprinting_metadata_remain_accepted() {
    let split = r"\begin{split}x&=y\\z&=w\end{split}";
    for source in [
        format!(r"\begin{{equation}}{split}\end{{equation}}"),
        format!(r"\begin{{equation}}\label{{eq:split}}{split}\end{{equation}}"),
        format!(r"\begin{{equation}}{split}\nonumber\end{{equation}}"),
        format!(r"\begin{{equation}}{{}}{split}\end{{equation}}"),
        format!(r"\begin{{gather}}a\\{split}\\b\end{{gather}}"),
        format!(r"\begin{{align}}a&=b\\{split}\\c&=d\end{{align}}"),
        format!(r"\begin{{gather}}\begin{{align}}a&=b\\{split}\end{{align}}\end{{gather}}"),
    ] {
        parse(&source).unwrap_or_else(|error| panic!("{source}: {error}"));
    }
}
