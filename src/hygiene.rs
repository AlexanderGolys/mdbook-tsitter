//! Macro hygiene, shown as background colours. In a Rust block marked
//! `hygiene`, the code is expanded by `rustc -Zunpretty=expanded,hygiene` and
//! the expansion is shown instead, each token on the background of the syntax
//! context it was resolved in: the code as written in one colour, and what each
//! macro produced in another.
//!
//! rustc prints a mark after every identifier, `a /* 2247#4 */`, where `2247`
//! is the interned symbol and `4` the syntax context. Only identifiers carry
//! marks, so the other tokens take a context from the syntax tree around them
//! (see [`token_contexts`]). The format is unstable: it is a debugging aid of
//! the compiler, and may change in any release.
//!
//! Stable rustc prints it too when `RUSTC_BOOTSTRAP=1` is set, so the book
//! does not need a nightly toolchain. `$RUSTC` chooses the compiler.

use std::collections::HashMap;
use std::io::Write;
use std::ops::Range;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use tree_sitter::{Language, Node, Parser};

use crate::markdown::escape_html;

/// The syntax context of code that no macro produced.
const ROOT: u32 = 0;

/// Class of every hygiene span; each also gets `hygiene-<n>`, where `n` is 0
/// for the code as written and counts the other contexts in order of
/// appearance.
const CLASS: &str = "hygiene";

/// Edition passed to rustc when the block names none.
const DEFAULT_EDITION: &str = "2021";

/// A run of tokens in one syntax context, as a span over the expansion.
#[derive(Debug, PartialEq, Eq)]
pub struct Region {
    pub range: Range<usize>,
    /// 0 for the code as written, then 1, 2, … in order of appearance.
    pub index: usize,
}

/// The expanded code of a block, with the context of every identifier.
#[derive(Debug, PartialEq, Eq)]
pub struct Expansion {
    pub source: String,
    /// Identifiers with their syntax context, in source order.
    pub idents: Vec<(Range<usize>, u32)>,
    /// What produced each context, e.g. `using_a!`, as far as rustc says.
    pub labels: HashMap<u32, String>,
    /// The context of each macro call, by the context of its output.
    pub call_sites: HashMap<u32, u32>,
}

/// Expand `code` with rustc. `edition` is the block's `edition20xx`
/// annotation, if any.
pub fn expand(code: &str, edition: Option<&str>) -> Result<Expansion> {
    let revealed = reveal_hidden_lines(code);
    let (input, wrapped) = match revealed.contains("fn main") {
        true => (revealed, false),
        false => (format!("fn main() {{\n{revealed}\n}}\n"), true),
    };
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let mut child = Command::new(&rustc)
        .args(["-Zunpretty=expanded,hygiene", "--crate-name", "hygiene"])
        .args(["--edition", edition.unwrap_or(DEFAULT_EDITION), "-"])
        .env("RUSTC_BOOTSTRAP", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("running `{rustc}` to expand macros"))?;
    child
        .stdin
        .take()
        .context("writing to rustc")?
        .write_all(input.as_bytes())?;
    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The expansion is printed even when the code then fails to compile, which
    // is often the point of a hygiene example.
    if !stdout.contains("SyntaxContexts:") {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("rustc could not expand the block:\n{stderr}");
    }
    Ok(parse(&stdout, wrapped))
}

/// Undo rustdoc's hidden lines (`# code`), which rustc must see.
fn reveal_hidden_lines(code: &str) -> String {
    code.split_inclusive('\n')
        .map(|line| {
            let indent = line.len() - line.trim_start().len();
            let rest = &line[indent..];
            match rest.strip_prefix('#') {
                Some(hidden) if hidden.starts_with(' ') => {
                    format!("{}{}", &line[..indent], &hidden[1..])
                }
                Some(bare) if bare.trim().is_empty() => format!("{}{bare}", &line[..indent]),
                _ => line.to_string(),
            }
        })
        .collect()
}

/// Read rustc's output: the code with its marks, then a comment listing the
/// expansions and syntax contexts. `wrapped` says the code was put in a
/// `fn main` of ours, to be taken off again.
pub fn parse(output: &str, wrapped: bool) -> Expansion {
    let (code, tables) = match output.find("\n/*\nExpansions:") {
        Some(at) => output.split_at(at),
        None => (output, ""),
    };
    let (labels, call_sites) = context_tables(tables);
    let (source, idents) = strip_hygiene_marks(code);
    let mut expansion = Expansion {
        source,
        idents,
        labels,
        call_sites,
    };
    let mut cuts = compiler_prelude(&expansion.source);
    cuts.extend(macro_definitions(&expansion.source));
    if wrapped {
        cuts.extend(our_main(&expansion.source));
    }
    expansion.cut(cuts);
    expansion.trim_blank_lines();
    expansion
}

/// The code without its `/* symbol#context */` marks, and the context of the
/// identifier before each mark.
fn strip_hygiene_marks(code: &str) -> (String, Vec<(Range<usize>, u32)>) {
    let mut source = String::with_capacity(code.len());
    let mut idents = Vec::new();
    let mut rest = code;
    while let Some(open) = rest.find("/*") {
        let Some(close) = rest[open..].find("*/").map(|close| open + close) else {
            break;
        };
        let inside = rest[open + 2..close].trim();
        let context = inside
            .split_once('#')
            .filter(|(symbol, _)| symbol.chars().all(|c| c.is_ascii_digit()))
            .and_then(|(_, context)| context.parse::<u32>().ok());
        let Some(context) = context else {
            // Not a mark: keep it as written.
            source.push_str(&rest[..close + 2]);
            rest = &rest[close + 2..];
            continue;
        };
        // The pretty-printer puts a space, or a line break and indentation,
        // before a mark; neither is part of the code.
        let before = rest[..open].trim_end();
        source.push_str(before);
        let end = source.len();
        let start = source
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_alphanumeric() || *c == '_' || *c == '\'' || *c == '#')
            .last()
            .map_or(end, |(index, _)| index);
        if start < end {
            idents.push((start..end, context));
        }
        rest = &rest[close + 2..];
    }
    source.push_str(rest);
    (source, idents)
}

/// What made each syntax context, and the context of the call that made it,
/// from the tables after the code: a context's outer mark names an expansion,
/// and the expansion's kind names the macro.
fn context_tables(tables: &str) -> (HashMap<u32, String>, HashMap<u32, u32>) {
    let mut expansions: HashMap<&str, (String, Option<u32>)> = HashMap::new();
    let mut contexts: HashMap<u32, &str> = HashMap::new();
    for line in tables.lines() {
        if let Some((id, rest)) = line.split_once(": parent: ") {
            if let Some(context) = id.strip_prefix('#').and_then(|n| n.parse().ok()) {
                if let Some(mark) = rest.split_once("outer_mark: (").map(|(_, mark)| mark) {
                    contexts.insert(context, mark.split(',').next().unwrap_or_default());
                }
            } else if let Some((_, kind)) = rest.split_once("kind: ") {
                let call_site = rest
                    .split_once("call_site_ctxt: #")
                    .and_then(|(_, n)| n.split(',').next()?.parse().ok());
                expansions.insert(id, (describe_kind(kind), call_site));
            }
        }
    }
    let mut labels = HashMap::new();
    let mut call_sites = HashMap::new();
    for (context, expansion) in contexts {
        if let Some((label, call_site)) = expansions.get(expansion) {
            labels.insert(context, label.clone());
            if let Some(call_site) = call_site.filter(|&site| site != context) {
                call_sites.insert(context, call_site);
            }
        }
    }
    (labels, call_sites)
}

/// `Macro(Bang, "using_a")` as `using_a!`, and other kinds as rustc names them.
fn describe_kind(kind: &str) -> String {
    let Some(args) = kind
        .strip_prefix("Macro(")
        .and_then(|k| k.strip_suffix(')'))
    else {
        return kind.to_string();
    };
    let (form, name) = args.split_once(", ").unwrap_or(("", args));
    let name = name.trim_matches('"');
    match form {
        "Bang" => format!("{name}!"),
        "Attr" => format!("#[{name}]"),
        "Derive" => format!("#[derive({name})]"),
        _ => name.to_string(),
    }
}

impl Expansion {
    /// Remove byte ranges of the source, keeping each identifier on its text.
    fn cut(&mut self, mut cuts: Vec<Range<usize>>) {
        cuts.sort_by_key(|cut| cut.start);
        let mut source = String::with_capacity(self.source.len());
        let mut kept: Vec<(usize, usize)> = Vec::new(); // (old start, new start)
        let mut cursor = 0;
        for cut in &cuts {
            if cut.start > cursor {
                kept.push((cursor, source.len()));
                source.push_str(&self.source[cursor..cut.start]);
            }
            cursor = cursor.max(cut.end);
        }
        kept.push((cursor, source.len()));
        source.push_str(&self.source[cursor..]);
        let inside_cut = |at: usize| cuts.iter().any(|cut| cut.contains(&at));
        let moved = |at: usize| {
            let (old, new) = kept
                .iter()
                .rev()
                .find(|(old, _)| *old <= at)
                .copied()
                .unwrap_or((0, 0));
            at - old + new
        };
        self.idents = std::mem::take(&mut self.idents)
            .into_iter()
            .filter(|(range, _)| !inside_cut(range.start))
            .map(|(range, context)| {
                (
                    moved(range.start)..moved(range.start) + range.len(),
                    context,
                )
            })
            .collect();
        self.source = source;
    }

    /// Drop blank lines at either end, left by what was cut.
    fn trim_blank_lines(&mut self) {
        let start = self.source.len() - self.source.trim_start_matches(['\n', ' ']).len();
        let start = self.source[..start].rfind('\n').map_or(0, |n| n + 1);
        let end = self.source.trim_end().len();
        self.cut(vec![0..start, end..self.source.len()]);
    }
}

/// Lines of the expansion with their byte ranges, newline included.
fn lines(source: &str) -> impl Iterator<Item = (Range<usize>, &str)> {
    source.split_inclusive('\n').scan(0, |offset, line| {
        let start = *offset;
        *offset += line.len();
        Some((start..*offset, line))
    })
}

/// The prelude import and `extern crate std` that rustc adds to every crate.
fn compiler_prelude(source: &str) -> Vec<Range<usize>> {
    lines(source)
        .filter(|(_, line)| {
            let line = line.trim();
            line == "#![feature(prelude_import)]"
                || line == "#![no_std]"
                || line == "#[prelude_import]"
                || line == "#[macro_use]"
                || line == "extern crate std;"
                || (line.starts_with("use ") && line.contains("::prelude::rust_"))
        })
        .map(|(range, _)| range)
        .collect()
}

/// `macro_rules!` items, which the pretty-printer shows unexpanded with every
/// token in the context of the code as written, and attributes above them.
fn macro_definitions(source: &str) -> Vec<Range<usize>> {
    let mut cuts = Vec::new();
    let mut search = 0;
    while let Some(found) = source[search..].find("macro_rules!") {
        let at = search + found;
        search = at + 1;
        let line_start = source[..at].rfind('\n').map_or(0, |n| n + 1);
        if !source[line_start..at].trim().is_empty() {
            continue;
        }
        let Some(end) = matching_close(source, at) else {
            continue;
        };
        let end = source[end..]
            .find('\n')
            .map_or(source.len(), |n| end + n + 1);
        // Attributes on the lines just above, e.g. `#[macro_export]`.
        let mut start = line_start;
        while start > 0 {
            let previous = source[..start - 1].rfind('\n').map_or(0, |n| n + 1);
            if !source[previous..start].trim_start().starts_with("#[") {
                break;
            }
            start = previous;
        }
        cuts.push(start..end);
        search = end;
    }
    cuts
}

/// The end of the bracketed body after `from`, past a closing `;` if any.
fn matching_close(source: &str, from: usize) -> Option<usize> {
    let open_at = from + source[from..].find(['{', '(', '['])?;
    let mut depth = 0usize;
    for (offset, c) in source[open_at..].char_indices() {
        match c {
            '{' | '(' | '[' => depth += 1,
            '}' | ')' | ']' => {
                depth -= 1;
                if depth == 0 {
                    let end = open_at + offset + 1;
                    return Some(match source[end..].starts_with(';') {
                        true => end + 1,
                        false => end,
                    });
                }
            }
            _ => {}
        }
    }
    None
}

/// The `fn main() {` and `}` lines of the wrapper we added, and one level of
/// indentation of every line between them.
fn our_main(source: &str) -> Vec<Range<usize>> {
    let all: Vec<_> = lines(source).collect();
    let Some(open) = all
        .iter()
        .position(|(_, line)| line.starts_with("fn main() {"))
    else {
        return Vec::new();
    };
    let Some(close) = all.iter().rposition(|(_, line)| line.trim_end() == "}") else {
        return Vec::new();
    };
    let mut cuts = vec![all[open].0.clone(), all[close].0.clone()];
    for (range, line) in &all[open + 1..close] {
        let indent = line.len() - line.trim_start_matches(' ').len();
        cuts.push(range.start..range.start + indent.min(4));
    }
    cuts
}

/// The syntax context of every token of `source`, parsed with `language`.
/// An identifier has its own; any other token takes the context its syntax
/// node got from its children, bottom up: a marked identifier counts twice as
/// much as a child node. In a tie between a macro's output and the code it was
/// called from, the macro's output wins, since a macro wraps what it is given;
/// a tie between unrelated outputs goes to the code that called them both.
/// So in `let four = { let a = 42; a / 10 };`, with the first `a` from the
/// macro and the rest as written, `let a = 42;` and the braces go to the
/// macro, and `/ 10`, the outer `let` and the `=` before the block to the code
/// as written; in `square!(n) + square!(m)`, the `+` goes to the code as
/// written.
fn token_contexts(expansion: &Expansion, language: &Language) -> Result<Vec<(Range<usize>, u32)>> {
    let source = &expansion.source;
    let idents = &expansion.idents;
    let mut parser = Parser::new();
    parser.set_language(language)?;
    let tree = parser
        .parse(source, None)
        .context("parsing the expansion")?;
    let by_start: HashMap<usize, u32> = idents.iter().map(|(r, c)| (r.start, *c)).collect();
    let mut contexts: HashMap<usize, u32> = HashMap::new(); // node id -> context
    let tie = |tied: &[u32]| break_tie(tied, &expansion.call_sites);
    own_context(tree.root_node(), &by_start, &tie, &mut contexts);

    let mut tokens = Vec::new();
    collect_tokens(tree.root_node(), &by_start, &contexts, ROOT, &mut tokens);
    Ok(tokens)
}

/// The contexts a context was called from, itself first and the code as
/// written last.
fn call_chain(context: u32, call_sites: &HashMap<u32, u32>) -> Vec<u32> {
    let mut chain = vec![context];
    while let Some(&site) = call_sites.get(chain.last().unwrap()) {
        if chain.contains(&site) {
            break;
        }
        chain.push(site);
    }
    if chain.last() != Some(&ROOT) {
        chain.push(ROOT);
    }
    chain
}

/// The context of a node whose children are split evenly between `tied`.
fn break_tie(tied: &[u32], call_sites: &HashMap<u32, u32>) -> u32 {
    let chains: Vec<Vec<u32>> = tied.iter().map(|&c| call_chain(c, call_sites)).collect();
    // The innermost, when it was called, directly or not, from all the others.
    if let Some(inner) = chains
        .iter()
        .find(|chain| tied.iter().all(|c| chain.contains(c)))
    {
        return inner[0];
    }
    // Otherwise the innermost context that called them all.
    chains[0]
        .iter()
        .copied()
        .find(|site| chains.iter().all(|chain| chain.contains(site)))
        .unwrap_or(ROOT)
}

fn own_context(
    node: Node,
    idents: &HashMap<usize, u32>,
    tie: &dyn Fn(&[u32]) -> u32,
    out: &mut HashMap<usize, u32>,
) -> Option<u32> {
    if node.child_count() == 0 {
        return idents.get(&node.start_byte()).copied();
    }
    let mut weights: HashMap<u32, u32> = HashMap::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let leaf = child.child_count() == 0;
        if let Some(context) = own_context(child, idents, tie, out) {
            *weights.entry(context).or_default() += if leaf { 2 } else { 1 };
        }
    }
    let most = *weights.values().max()?;
    let mut tied: Vec<u32> = weights
        .into_iter()
        .filter(|&(_, weight)| weight == most)
        .map(|(context, _)| context)
        .collect();
    tied.sort_unstable();
    let context = match tied[..] {
        [only] => only,
        _ => tie(&tied),
    };
    out.insert(node.id(), context);
    Some(context)
}

fn collect_tokens(
    node: Node,
    idents: &HashMap<usize, u32>,
    contexts: &HashMap<usize, u32>,
    inherited: u32,
    out: &mut Vec<(Range<usize>, u32)>,
) {
    if node.child_count() == 0 {
        let context = idents.get(&node.start_byte()).copied().unwrap_or(inherited);
        if !node.byte_range().is_empty() {
            out.push((node.byte_range(), context));
        }
        return;
    }
    let here = contexts.get(&node.id()).copied().unwrap_or(inherited);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_tokens(child, idents, contexts, here, out);
    }
}

/// Runs of tokens in one context, each within a line, and the label of each
/// context index used, in index order.
pub fn regions(expansion: &Expansion, language: &Language) -> Result<(Vec<Region>, Vec<String>)> {
    let tokens = token_contexts(expansion, language)?;
    let mut order: Vec<u32> = vec![ROOT];
    let mut regions: Vec<Region> = Vec::new();
    for (range, context) in tokens {
        let index = match order.iter().position(|&c| c == context) {
            Some(index) => index,
            None => {
                order.push(context);
                order.len() - 1
            }
        };
        if let Some(last) = regions.last_mut() {
            let between = &expansion.source[last.range.end..range.start];
            if last.index == index && !between.contains('\n') {
                last.range.end = range.end;
                continue;
            }
        }
        regions.push(Region { range, index });
    }
    let labels = order
        .iter()
        .map(|context| match *context {
            ROOT => "as written".to_string(),
            other => expansion
                .labels
                .get(&other)
                .cloned()
                .unwrap_or_else(|| format!("context #{other}")),
        })
        .collect();
    Ok((regions, labels))
}

/// The `class` and `title` attributes of a region's span.
pub fn attribute(index: usize, label: &str) -> Vec<u8> {
    format!(
        "class=\"{CLASS} {CLASS}-{index}\" title=\"{}\"",
        escape_html(label)
    )
    .into_bytes()
}

/// A legend naming the colour of each context.
pub fn legend(labels: &[String]) -> String {
    let entries: String = labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            format!(
                "<span class=\"{CLASS} {CLASS}-{index}\">{}</span>",
                escape_html(label)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("<div class=\"{CLASS}-legend\">{entries}</div>")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What nightly rustc printed for the Little Book of Rust Macros'
    /// `using_a!` example, trimmed of most of the tables.
    const USING_A: &str = "#![feature /* 839#0 */(prelude_import /* 1465#0 */)]
extern crate std /* 1876#1 */;
#[prelude_import /* 1465#1 */]
use std /* 1876#1 */::prelude /* 1464#1 */::rust_2021 /* 1597#1 */::*;
macro_rules! using_a
    /*
    2352#0
    */ {
    ($e /* 2251#0 */:expr /* 807#0 */) =>
    { { let /* 17#0 */ a /* 2247#0 */ = 42; $e /* 2251#0 */ } }
}

fn main /* 1126#0 */() {
    let four /* 2354#0 */ = { let a /* 2247#4 */ = 42; a /* 2247#0 */ / 10 };
}

/*
Expansions:
crate0::{{expn0}}: parent: crate0::{{expn0}}, call_site_ctxt: #0, def_site_ctxt: #0, kind: Root
crate0::{{expn2}}: parent: crate0::{{expn0}}, call_site_ctxt: #0, def_site_ctxt: #0, kind: Macro(Bang, \"using_a\")

SyntaxContexts:
#0: parent: #0, outer_mark: (crate0::{{expn0}}, Opaque)
#4: parent: #0, outer_mark: (crate0::{{expn2}}, SemiOpaque)
*/
";

    #[test]
    fn marks_are_removed_and_the_wrapper_and_definitions_cut() {
        let expansion = parse(USING_A, true);
        assert_eq!(expansion.source, "let four = { let a = 42; a / 10 };");
        let contexts: Vec<(&str, u32)> = expansion
            .idents
            .iter()
            .map(|(range, context)| (&expansion.source[range.clone()], *context))
            .collect();
        assert_eq!(contexts, [("four", 0), ("a", 4), ("a", 0)]);
        assert_eq!(expansion.labels[&4], "using_a!");
    }

    #[test]
    fn a_main_of_the_reader_stays() {
        let expansion = parse(USING_A, false);
        assert!(expansion.source.starts_with("fn main() {\n    let four"));
    }

    #[test]
    fn ties_go_to_the_inner_output_or_the_shared_caller() {
        // 1 and 2 were called from 0, and 3 from 1.
        let call_sites = HashMap::from([(1, 0), (2, 0), (3, 1)]);
        assert_eq!(break_tie(&[0, 1], &call_sites), 1);
        assert_eq!(break_tie(&[0, 3], &call_sites), 3);
        assert_eq!(break_tie(&[1, 2], &call_sites), 0);
        assert_eq!(break_tie(&[2, 3], &call_sites), 0);
        assert_eq!(break_tie(&[1, 3], &call_sites), 3);
    }

    #[test]
    fn kinds_are_named_as_written() {
        assert_eq!(describe_kind("Macro(Bang, \"vec\")"), "vec!");
        assert_eq!(
            describe_kind("Macro(Derive, \"Debug\")"),
            "#[derive(Debug)]"
        );
        assert_eq!(describe_kind("AstPass(StdImports)"), "AstPass(StdImports)");
    }

    #[test]
    fn hidden_lines_are_shown_to_rustc() {
        assert_eq!(
            reveal_hidden_lines("# fn f() {}\nlet x = 1;\n#\n"),
            "fn f() {}\nlet x = 1;\n\n"
        );
    }
}
