//! Strict parser for the repository gold format.
//!
//! The format is deliberately a narrow TOML subset: `[[gold]]` array tables,
//! TOML bare keys, and TOML basic-string values. Per-kind schemas live in the
//! owning runner. The parser fails closed on malformed evidence instead of
//! silently skipping or repairing it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug)]
pub(crate) struct GoldRecord {
    source: String,
    start_line: usize,
    fields: BTreeMap<String, String>,
}

impl GoldRecord {
    pub(crate) fn name(&self) -> &str {
        self.required("name")
    }

    pub(crate) fn kind(&self) -> &str {
        self.required("kind")
    }

    pub(crate) fn required(&self, key: &str) -> &str {
        self.fields.get(key).map(String::as_str).unwrap_or_else(|| {
            panic!(
                "{}:{}: gold {} is missing required field {key}",
                self.source,
                self.start_line,
                self.fields
                    .get("name")
                    .map(String::as_str)
                    .unwrap_or("<unnamed>")
            )
        })
    }

    pub(crate) fn optional(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }

    pub(crate) fn expect_fields(&self, required: &[&str], optional: &[&str]) {
        for key in required {
            if !self.fields.contains_key(*key) {
                panic!(
                    "{}:{}: gold {} kind {} is missing required field {key}",
                    self.source,
                    self.start_line,
                    self.name(),
                    self.kind()
                );
            }
        }

        let allowed: BTreeSet<&str> = required.iter().chain(optional.iter()).copied().collect();
        let unexpected: Vec<&str> = self
            .fields
            .keys()
            .map(String::as_str)
            .filter(|key| !allowed.contains(key))
            .collect();
        if !unexpected.is_empty() {
            panic!(
                "{}:{}: gold {} kind {} has fields not consumed by this contract: {}",
                self.source,
                self.start_line,
                self.name(),
                self.kind(),
                unexpected.join(", ")
            );
        }
    }
}

pub(crate) fn load(relative_path: &str) -> Vec<GoldRecord> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    parse_text(relative_path, &text)
}

pub(crate) fn parse_text(source: &str, text: &str) -> Vec<GoldRecord> {
    let mut records = Vec::new();
    let mut names = BTreeSet::new();
    let mut current: Option<(usize, BTreeMap<String, String>)> = None;

    for (index, raw_line) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = trim_toml_ws_start(raw_line);
        if line.is_empty() {
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            validate_comment(source, line_no, comment);
            continue;
        }
        if let Some(rest) = line.strip_prefix("[[gold]]") {
            let rest = trim_toml_ws_start(rest);
            if rest.is_empty() {
                finish_record(source, &mut current, &mut records, &mut names);
                current = Some((line_no, BTreeMap::new()));
                continue;
            }
            if let Some(comment) = rest.strip_prefix('#') {
                validate_comment(source, line_no, comment);
                finish_record(source, &mut current, &mut records, &mut names);
                current = Some((line_no, BTreeMap::new()));
                continue;
            }
        }
        if line.starts_with('[') {
            panic!("{source}:{line_no}: unknown table header {line:?}; expected [[gold]]");
        }

        let (_, fields) = current.as_mut().unwrap_or_else(|| {
            panic!("{source}:{line_no}: gold field appears before the first [[gold]]")
        });
        let (raw_key, raw_value) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("{source}:{line_no}: malformed gold line {line:?}"));
        let key = trim_toml_ws_end(raw_key);
        if key.is_empty()
            || !key
                .chars()
                .all(|ch| ch == '_' || ch == '-' || ch.is_ascii_alphanumeric())
        {
            panic!("{source}:{line_no}: invalid gold field name {key:?}");
        }
        let value = parse_basic_string(source, line_no, raw_value);
        if fields.insert(key.to_string(), value).is_some() {
            panic!("{source}:{line_no}: duplicate gold field {key}");
        }
    }

    finish_record(source, &mut current, &mut records, &mut names);
    if records.is_empty() {
        panic!("{source}: no [[gold]] records loaded");
    }
    records
}

fn finish_record(
    source: &str,
    current: &mut Option<(usize, BTreeMap<String, String>)>,
    records: &mut Vec<GoldRecord>,
    names: &mut BTreeSet<String>,
) {
    let Some((start_line, fields)) = current.take() else {
        return;
    };
    if fields.is_empty() {
        panic!("{source}:{start_line}: empty [[gold]] record");
    }
    let name = fields
        .get("name")
        .unwrap_or_else(|| panic!("{source}:{start_line}: gold record is missing name"));
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch == '-' || ch == '_' || ch.is_ascii_alphanumeric())
    {
        panic!("{source}:{start_line}: invalid gold name {name:?}");
    }
    let kind = fields
        .get("kind")
        .unwrap_or_else(|| panic!("{source}:{start_line}: gold {name} is missing kind"));
    if kind.is_empty()
        || !kind
            .chars()
            .all(|ch| ch == '-' || ch == '_' || ch.is_ascii_alphanumeric())
    {
        panic!("{source}:{start_line}: invalid gold kind {kind:?}");
    }
    if !names.insert(name.to_string()) {
        panic!("{source}:{start_line}: duplicate gold name {name}");
    }
    records.push(GoldRecord {
        source: source.to_string(),
        start_line,
        fields,
    });
}

fn parse_basic_string(source: &str, line_no: usize, raw: &str) -> String {
    let mut chars = trim_toml_ws_start(raw).chars();
    if chars.next() != Some('"') {
        panic!("{source}:{line_no}: gold values must be TOML basic strings");
    }

    let mut out = String::new();
    loop {
        match chars.next() {
            Some('"') => {
                let trailing: String = chars.collect();
                let trailing = trim_toml_ws_start(&trailing);
                if trailing.is_empty() {
                    return out;
                }
                if let Some(comment) = trailing.strip_prefix('#') {
                    validate_comment(source, line_no, comment);
                    return out;
                }
                panic!("{source}:{line_no}: unexpected text after gold string: {trailing:?}");
            }
            Some('\\') => match chars.next() {
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('b') => out.push('\u{0008}'),
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some('f') => out.push('\u{000C}'),
                Some('r') => out.push('\r'),
                Some('u') => out.push(parse_unicode_escape(source, line_no, &mut chars, 4)),
                Some('U') => out.push(parse_unicode_escape(source, line_no, &mut chars, 8)),
                Some(other) => panic!("{source}:{line_no}: unsupported TOML escape \\{other}"),
                None => panic!("{source}:{line_no}: truncated escape in gold string"),
            },
            Some(ch) if is_forbidden_control(ch) => {
                panic!("{source}:{line_no}: unescaped control character in gold string")
            }
            Some(ch) => out.push(ch),
            None => panic!("{source}:{line_no}: unterminated gold string"),
        }
    }
}

fn parse_unicode_escape<I: Iterator<Item = char>>(
    source: &str,
    line_no: usize,
    chars: &mut I,
    digits: usize,
) -> char {
    let mut value = 0_u32;
    for _ in 0..digits {
        let ch = chars
            .next()
            .unwrap_or_else(|| panic!("{source}:{line_no}: truncated Unicode escape"));
        let digit = ch
            .to_digit(16)
            .unwrap_or_else(|| panic!("{source}:{line_no}: invalid Unicode escape digit {ch:?}"));
        value = (value << 4) | digit;
    }
    char::from_u32(value)
        .unwrap_or_else(|| panic!("{source}:{line_no}: invalid Unicode scalar U+{value:04X}"))
}

fn trim_toml_ws_start(text: &str) -> &str {
    text.trim_start_matches([' ', '\t'])
}

fn trim_toml_ws_end(text: &str) -> &str {
    text.trim_end_matches([' ', '\t'])
}

fn validate_comment(source: &str, line_no: usize, comment: &str) {
    if comment.chars().any(is_forbidden_control) {
        panic!("{source}:{line_no}: control character in TOML comment");
    }
}

fn is_forbidden_control(ch: char) -> bool {
    (ch <= '\u{001F}' && ch != '\t') || ch == '\u{007F}'
}
