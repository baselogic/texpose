//! Equation counters, `\tag`, and `\label` / `\ref` (two-pass).

use std::collections::HashMap;

use crate::{
    parser::{EnvRow, EqNumber, MathNode, MatrixStyle},
    Error,
};

/// How auto equation numbers are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NumberStyle {
    /// `(1)`, `(2)`, …
    Arabic,
    /// `(i)`, `(ii)`, …
    Roman,
    /// `(a)`, `(b)`, …
    Alphabetic,
}

/// Wrapper around the number body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NumberFormat {
    /// `(1)`
    Parenthesized,
    /// `[1]`
    Bracketed,
    /// `1`
    Plain,
}

/// Counter style for one layout operation (or a sequence of [`layout_with_numbering`](super::layout_with_numbering) calls).
///
/// # Examples
///
/// ```
/// use texpose::{NumberFormat, NumberStyle, NumberingConfig};
///
/// let cfg = NumberingConfig::new()
///     .with_style(NumberStyle::Roman)
///     .with_start(4)
///     .with_format(NumberFormat::Bracketed);
/// assert_eq!(cfg.style(), NumberStyle::Roman);
/// assert_eq!(cfg.start(), 4);
/// assert_eq!(cfg.format(), NumberFormat::Bracketed);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumberingConfig {
    style: NumberStyle,
    start: usize,
    format: NumberFormat,
}

impl Default for NumberingConfig {
    fn default() -> Self {
        Self {
            style: NumberStyle::Arabic,
            start: 1,
            format: NumberFormat::Parenthesized,
        }
    }
}

impl NumberingConfig {
    /// Arabic, start at 1, parenthesized.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Automatic-number style.
    #[must_use]
    pub fn style(&self) -> NumberStyle {
        self.style
    }

    /// First automatic number.
    #[must_use]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Display wrapper used for automatic equation numbers.
    #[must_use]
    pub fn format(&self) -> NumberFormat {
        self.format
    }

    /// Return this configuration with a different automatic-number style.
    #[must_use]
    pub fn with_style(mut self, style: NumberStyle) -> Self {
        self.style = style;
        self
    }

    /// Return this configuration with a different first automatic number.
    #[must_use]
    pub fn with_start(mut self, start: usize) -> Self {
        self.start = start;
        self
    }

    /// Return this configuration with a different display wrapper.
    #[must_use]
    pub fn with_format(mut self, format: NumberFormat) -> Self {
        self.format = format;
        self
    }
}

/// Mutable numbering / label table. Survives across `layout_with_numbering` calls.
///
/// # Examples
///
/// ```
/// use texpose::{NumberingConfig, NumberingState};
///
/// let state = NumberingState::new(NumberingConfig::new());
/// assert!(state.label("eq:1").is_none());
/// ```
#[derive(Clone, Debug)]
pub struct NumberingState {
    config: NumberingConfig,
    next: usize,
    labels: HashMap<String, LabelValue>,
}

#[derive(Clone, Debug)]
struct LabelValue {
    display: String,
    reference: String,
}

#[derive(Clone, Debug)]
struct NumberAssignment {
    display: String,
    reference: String,
}

#[derive(Clone, Debug)]
pub(crate) struct NumberingPlan {
    next: usize,
    labels: HashMap<String, LabelValue>,
    assigned: Vec<Option<String>>,
}

impl NumberingPlan {
    pub(crate) fn assigned(&self, i: usize) -> Option<&str> {
        self.assigned.get(i).and_then(|o| o.as_deref())
    }

    pub(crate) fn lookup<'a>(&'a self, state: &'a NumberingState, key: &str) -> Option<&'a str> {
        self.labels
            .get(key)
            .or_else(|| state.labels.get(key))
            .map(|value| value.reference.as_str())
    }

    fn wrap(config: &NumberingConfig, body: &str) -> String {
        match config.format {
            NumberFormat::Parenthesized => format!("({body})"),
            NumberFormat::Bracketed => format!("[{body}]"),
            NumberFormat::Plain => body.to_string(),
        }
    }

    fn auto_body(config: &NumberingConfig, n: usize) -> String {
        match config.style {
            NumberStyle::Arabic => n.to_string(),
            NumberStyle::Roman => to_roman(n),
            NumberStyle::Alphabetic => to_alpha(n),
        }
    }

    fn auto_assignment(&mut self, config: &NumberingConfig) -> Result<NumberAssignment, Error> {
        let n = self.next;
        let next = n.checked_add(1).ok_or_else(|| Error::InvalidOption {
            what: "equation counter overflow".into(),
        })?;
        let reference = Self::auto_body(config, n);
        self.next = next;
        Ok(NumberAssignment {
            display: Self::wrap(config, &reference),
            reference,
        })
    }

    fn bind(&mut self, labels: &[String], assignment: &NumberAssignment) {
        for key in labels {
            self.labels.insert(
                key.clone(),
                LabelValue {
                    display: assignment.display.clone(),
                    reference: assignment.reference.clone(),
                },
            );
        }
    }
}

impl Default for NumberingState {
    fn default() -> Self {
        Self::new(NumberingConfig::default())
    }
}

impl NumberingState {
    /// Counter starts at `config.start`.
    #[must_use]
    pub fn new(config: NumberingConfig) -> Self {
        let next = config.start;
        Self {
            config,
            next,
            labels: HashMap::new(),
        }
    }

    /// Formatted equation number bound to `key`, if `\label{key}` was seen.
    ///
    /// This is the display form, including [`NumberFormat`]. `\ref` uses the
    /// unwrapped reference payload internally.
    #[must_use]
    pub fn label(&self, key: &str) -> Option<&str> {
        self.labels.get(key).map(|value| value.display.as_str())
    }

    pub(crate) fn prepare(&self, node: &MathNode) -> Result<NumberingPlan, Error> {
        let mut plan = NumberingPlan {
            next: self.next,
            labels: HashMap::new(),
            assigned: Vec::new(),
        };
        collect_node(node, &self.config, &mut plan)?;
        Ok(plan)
    }

    pub(crate) fn commit(&mut self, plan: NumberingPlan) {
        self.next = plan.next;
        self.labels.extend(plan.labels);
    }
}

fn collect_node(
    node: &MathNode,
    config: &NumberingConfig,
    plan: &mut NumberingPlan,
) -> Result<(), Error> {
    match node {
        MathNode::Matrix(style, _, rows) => collect_matrix(*style, rows, config, plan)?,
        MathNode::Row(v) | MathNode::Substack(v) => {
            for n in v {
                collect_node(n, config, plan)?;
            }
        }
        MathNode::Fraction(spec) => {
            collect_node(&spec.numerator, config, plan)?;
            collect_node(&spec.denominator, config, plan)?;
        }
        MathNode::Superscript(a, b) | MathNode::Subscript(a, b) | MathNode::CancelTo(a, b) => {
            collect_node(a, config, plan)?;
            collect_node(b, config, plan)?;
        }
        MathNode::SubSup(a, b, c) => {
            collect_node(a, config, plan)?;
            collect_node(b, config, plan)?;
            collect_node(c, config, plan)?;
        }
        MathNode::Radical(deg, r) => {
            if let Some(d) = deg {
                collect_node(d, config, plan)?;
            }
            collect_node(r, config, plan)?;
        }
        MathNode::Delimited(_, b, _)
        | MathNode::Pmb(b)
        | MathNode::Limits(b, _)
        | MathNode::Accent(b, _)
        | MathNode::Color(_, b)
        | MathNode::TextColor(_, b)
        | MathNode::ColorBox(_, b)
        | MathNode::Phantom(_, b)
        | MathNode::Intertext(b)
        | MathNode::Tag { body: b, .. } => collect_node(b, config, plan)?,
        MathNode::FColorBox(_, _, b) => collect_node(b, config, plan)?,
        MathNode::Sum(lo, hi) | MathNode::Product(lo, hi) | MathNode::Integral(_, lo, hi) => {
            if let Some(n) = lo {
                collect_node(n, config, plan)?;
            }
            if let Some(n) = hi {
                collect_node(n, config, plan)?;
            }
        }
        MathNode::Limit(lo) => {
            if let Some(n) = lo {
                collect_node(n, config, plan)?;
            }
        }
        MathNode::OverUnder(b, over, under) => {
            collect_node(b, config, plan)?;
            if let Some(n) = over {
                collect_node(n, config, plan)?;
            }
            if let Some(n) = under {
                collect_node(n, config, plan)?;
            }
        }
        MathNode::StackRel(b, over) => {
            collect_node(b, config, plan)?;
            collect_node(over, config, plan)?;
        }
        MathNode::Atom(_, _)
        | MathNode::SizedDelim(_, _, _)
        | MathNode::MathAlphabet(_, _)
        | MathNode::LiteralText(_)
        | MathNode::Space(_)
        | MathNode::Style(_)
        | MathNode::Operator(_, _)
        | MathNode::Symbol(_)
        | MathNode::Strut(_, _)
        | MathNode::Rule(_, _)
        | MathNode::Ref(_)
        | MathNode::Label(_)
        | MathNode::NoNumber
        | MathNode::Hline => {}
    }
    Ok(())
}

fn collect_matrix(
    style: MatrixStyle,
    rows: &[EnvRow],
    config: &NumberingConfig,
    plan: &mut NumberingPlan,
) -> Result<(), Error> {
    for row in rows {
        match row {
            EnvRow::Cells { cells, .. } => {
                for c in cells {
                    collect_node(c, config, plan)?;
                }
            }
            EnvRow::Intertext(n) => collect_node(n, config, plan)?,
            EnvRow::Hline => {}
        }
    }
    if style.numbers_rows() {
        for row in rows {
            match row {
                EnvRow::Intertext(_) | EnvRow::Hline => {}
                EnvRow::Cells { number, labels, .. } => {
                    let assignment = assign(number, config, plan)?;
                    if let Some(value) = &assignment {
                        plan.bind(labels, value);
                    }
                    plan.assigned.push(assignment.map(|value| value.display));
                }
            }
        }
    } else if style.numbers_once() {
        let mut number = EqNumber::Default;
        let mut labels = Vec::new();
        for row in rows {
            if let EnvRow::Cells {
                number: n,
                labels: l,
                ..
            } = row
            {
                match n {
                    EqNumber::Default => {}
                    other => number = other.clone(),
                }
                labels.extend(l.iter().cloned());
            }
        }
        let assignment = assign(&number, config, plan)?;
        if let Some(value) = &assignment {
            plan.bind(&labels, value);
        }
        plan.assigned.push(assignment.map(|value| value.display));
    }
    Ok(())
}

fn assign(
    number: &EqNumber,
    config: &NumberingConfig,
    plan: &mut NumberingPlan,
) -> Result<Option<NumberAssignment>, Error> {
    match number {
        EqNumber::Suppress => Ok(None),
        EqNumber::Tag { star, body } => {
            let reference = node_plain(body);
            let display = if *star {
                reference.clone()
            } else {
                NumberingPlan::wrap(config, &reference)
            };
            Ok(Some(NumberAssignment { display, reference }))
        }
        EqNumber::Default => plan.auto_assignment(config).map(Some),
    }
}

fn node_plain(n: &MathNode) -> String {
    match n {
        MathNode::Atom(c, _) => c.to_string(),
        MathNode::MathAlphabet(s, _) | MathNode::LiteralText(s) => s.clone(),
        MathNode::Symbol(name) => name.clone(),
        MathNode::Row(v) => v.iter().map(node_plain).collect(),
        MathNode::Tag { body, .. } | MathNode::Pmb(body) => node_plain(body),
        other => other.gold(),
    }
}

fn to_roman(mut n: usize) -> String {
    if n == 0 {
        return "0".into();
    }
    let pairs: [(usize, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut s = String::new();
    for (v, g) in pairs {
        while n >= v {
            s.push_str(g);
            n -= v;
        }
    }
    s
}

fn to_alpha(mut n: usize) -> String {
    if n == 0 {
        return "0".into();
    }
    let mut s = String::new();
    while n > 0 {
        n -= 1;
        s.insert(0, char::from(b'a' + (n % 26) as u8));
        n /= 26;
    }
    s
}
