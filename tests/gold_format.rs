//! Adversarial checks for the shared golden-file evidence parser.

#[path = "support/golds.rs"]
mod gold_support;

use std::panic::{catch_unwind, AssertUnwindSafe};

#[test]
fn repository_gold_files_load_through_strict_parser() {
    for path in [
        "golds/milestone1.toml",
        "golds/parse.toml",
        "golds/layout.toml",
        "golds/symbols.toml",
        "golds/accents.toml",
        "golds/envs.toml",
    ] {
        let records = gold_support::load(path);
        assert!(!records.is_empty(), "{path}");
    }
}

fn rejects(text: &str) {
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            gold_support::parse_text("test.gold.toml", text)
        }))
        .is_err(),
        "malformed gold evidence was accepted: {text:?}"
    );
}

#[test]
fn strict_gold_parser_accepts_supported_toml_strings_and_whitespace() {
    let records = gold_support::parse_text(
        "test.gold.toml",
        "\t[[gold]] \t# table comment\nname = \"one\"\nkind = \"ast\"\ninput = \"\\\\alpha\\n\"\nextra-key \t= \t\"v\"\nexpect = \"value]]\" # comment\n",
    );
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].name(), "one");
    assert_eq!(records[0].required("input"), "\\alpha\n");
    assert_eq!(records[0].required("extra-key"), "v");
    assert_eq!(records[0].required("expect"), "value]]");
    assert_eq!(records[0].optional("missing"), None);
}

#[test]
fn strict_gold_parser_rejects_corrupt_evidence() {
    for text in [
        "[[g0ld]]\nname = \"one\"\nkind = \"ast\"\n",
        "[gold]\nname = \"one\"\nkind = \"ast\"\n",
        "name = \"one\"\nkind = \"ast\"\n",
        "[[gold]]\n",
        "[[gold]]\nkind = \"ast\"\n",
        "[[gold]]\nname = \"one\"\n",
        "[[gold]]\nname = \"\"\nkind = \"ast\"\n",
        "[[gold]]\nname = \"bad name\"\nkind = \"ast\"\n",
        "[[gold]]\nname = \"one\"\nkind = \"\"\n",
        "[[gold]]\nname = \"one\"\nkind = \"bad kind\"\n",
        "[[gold]]\nname = \"one\"\nname = \"two\"\nkind = \"ast\"\n",
        "[[gold]]\nname = \"one\"\nkind = \"ast\"\ninput = \"\\q\"\n",
        "[[gold]]\nname = \"one\"\nkind = \"ast\"\ninput = \"abc\\\"\n",
        "[[gold]]\nname = \"one\"\nkind = \"ast\"\ninput = \"unterminated\n",
        "[[gold]]\nname = \"one\"\nkind = \"ast\"\ninput = 1\n",
        "[[gold]]\nname = \"one\"\nkind = \"ast\"\ninput = \"\\uD800\"\n",
        "[[gold]]\nname = \"one\"\nkind = \"ast\"\nexpect = \"x\" trailing\n",
        "[[gold]]\nname = \"one\"\nkind = \"ast\"\nmalformed\n",
        "[[gold]]\nname = \"one\"\nkind = \"ast\"\n[[gold]]\nname = \"one\"\nkind = \"ast\"\n",
        "\u{00A0}[[gold]]\nname = \"one\"\nkind = \"ast\"\n",
        "[[gold]]\nname\u{00A0}= \"one\"\nkind = \"ast\"\n",
        "[[gold]]\nname = \"one\"\u{00A0}# comment\nkind = \"ast\"\n",
        "# invalid\u{0001}comment\n[[gold]]\nname = \"one\"\nkind = \"ast\"\n",
        "[[gold]]\nname = \"one\" # invalid\u{0001}comment\nkind = \"ast\"\n",
        "\u{000B}# not TOML whitespace\n[[gold]]\nname = \"one\"\nkind = \"ast\"\n",
    ] {
        rejects(text);
    }
}

#[test]
fn strict_gold_schema_rejects_inert_fields() {
    let records = gold_support::parse_text(
        "test.gold.toml",
        "[[gold]]\nname = \"one\"\nkind = \"ast\"\ninput = \"x\"\nexpect = \"x\"\nunknown = \"y\"\n",
    );
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            records[0].expect_fields(&["name", "kind", "input", "expect"], &[])
        }))
        .is_err(),
        "schema accepted an unknown field"
    );
}
