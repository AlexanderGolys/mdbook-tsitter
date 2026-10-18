//! Reading a Lua file that returns a table literal, such as nvim-treesitter's
//! parser list, without a Lua interpreter. Only literals are understood:
//! tables, strings, numbers, booleans, and `nil`, with comments anywhere.

use std::collections::BTreeMap;

/// A Lua table: fields with a name or string key, and positional values.
#[derive(Debug, Default)]
pub struct Table {
    pub named: BTreeMap<String, Value>,
    pub positional: Vec<Value>,
}

impl Table {
    pub fn field(&self, name: &str) -> Option<&Value> {
        self.named.get(name)
    }

    pub fn string(&self, name: &str) -> Option<&str> {
        match self.field(name)? {
            Value::String(text) => Some(text),
            _ => None,
        }
    }
}

/// A Lua literal. Numbers, booleans, and `nil` are read past but not kept,
/// since a list of grammars is made of tables and strings.
#[derive(Debug)]
pub enum Value {
    Table(Table),
    String(String),
    Other,
}

impl Value {
    pub fn as_table(&self) -> Option<&Table> {
        match self {
            Value::Table(table) => Some(table),
            _ => None,
        }
    }
}

/// The table literal a Lua file returns, as in `return { … }`.
pub fn parse_returned_table(source: &str) -> Result<Table, String> {
    let mut parser = Parser {
        source,
        position: 0,
    };
    parser.skip_trivia()?;
    if parser.identifier() != Some("return") {
        return Err(parser.error("expected `return`"));
    }
    parser.skip_trivia()?;
    let table = parser.table()?;
    parser.skip_trivia()?;
    if parser.position < source.len() {
        return Err(parser.error("unexpected text after the returned table"));
    }
    Ok(table)
}

struct Parser<'a> {
    source: &'a str,
    position: usize,
}

impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str {
        &self.source[self.position..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn error(&self, message: &str) -> String {
        let line = self.source[..self.position].matches('\n').count() + 1;
        format!("line {line}: {message}")
    }

    fn expect(&mut self, token: char) -> Result<(), String> {
        self.skip_trivia()?;
        if self.peek() != Some(token) {
            return Err(self.error(&format!("expected `{token}`")));
        }
        self.position += token.len_utf8();
        Ok(())
    }

    /// Skip whitespace and comments: `-- line` and `--[[ long ]]`, the long
    /// form with any number of `=` between its brackets.
    fn skip_trivia(&mut self) -> Result<(), String> {
        loop {
            let trimmed = self.rest().trim_start();
            self.position = self.source.len() - trimmed.len();
            let Some(comment) = trimmed.strip_prefix("--") else {
                return Ok(());
            };
            self.position += 2;
            if let Some(close) = long_bracket_close(comment) {
                let end = comment
                    .find(&close)
                    .ok_or_else(|| self.error("unterminated comment"))?;
                self.position += end + close.len();
            } else {
                self.position += comment.find('\n').unwrap_or(comment.len());
            }
        }
    }

    fn identifier(&mut self) -> Option<&'a str> {
        let rest = self.rest();
        let length = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        let identifier = &rest[..length];
        if identifier.is_empty() || identifier.starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        self.position += length;
        Some(identifier)
    }

    fn value(&mut self) -> Result<Value, String> {
        self.skip_trivia()?;
        match self.peek() {
            Some('{') => self.table().map(Value::Table),
            Some('\'' | '"') => self.string().map(Value::String),
            Some(c) if c.is_ascii_digit() || c == '-' || c == '.' => {
                self.skip_number();
                Ok(Value::Other)
            }
            _ => match self.identifier() {
                Some("true" | "false" | "nil") => Ok(Value::Other),
                _ => Err(self.error("expected a literal")),
            },
        }
    }

    fn table(&mut self) -> Result<Table, String> {
        self.expect('{')?;
        let mut table = Table::default();
        loop {
            self.skip_trivia()?;
            if self.peek() == Some('}') {
                self.position += 1;
                return Ok(table);
            }
            match self.field_name()? {
                Some(name) => {
                    let value = self.value()?;
                    table.named.insert(name, value);
                }
                None => {
                    let value = self.value()?;
                    table.positional.push(value);
                }
            }
            self.skip_trivia()?;
            match self.peek() {
                Some(',' | ';') => self.position += 1,
                Some('}') => {}
                _ => return Err(self.error("expected `,` or `}`")),
            }
        }
    }

    /// The name of a `name = value` or `['name'] = value` field, consuming it
    /// and the `=`, or `None` for a positional value.
    fn field_name(&mut self) -> Result<Option<String>, String> {
        let start = self.position;
        if self.peek() == Some('[') && long_bracket_close(self.rest()).is_none() {
            self.position += 1;
            self.skip_trivia()?;
            let Value::String(name) = self.value()? else {
                return Err(self.error("expected a string key"));
            };
            self.expect(']')?;
            self.expect('=')?;
            return Ok(Some(name));
        }
        if let Some(name) = self.identifier() {
            self.skip_trivia()?;
            if self.peek() == Some('=') && !self.rest().starts_with("==") {
                self.position += 1;
                return Ok(Some(name.to_string()));
            }
        }
        self.position = start;
        Ok(None)
    }

    fn string(&mut self) -> Result<String, String> {
        let quote = self.peek().ok_or_else(|| self.error("expected a string"))?;
        self.position += quote.len_utf8();
        let mut text = String::new();
        let mut chars = self.rest().char_indices();
        while let Some((offset, c)) = chars.next() {
            match c {
                '\\' => {
                    let (_, escaped) = chars
                        .next()
                        .ok_or_else(|| self.error("unterminated string"))?;
                    text.push(match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        other => other,
                    });
                }
                '\n' => return Err(self.error("unterminated string")),
                c if c == quote => {
                    self.position += offset + c.len_utf8();
                    return Ok(text);
                }
                c => text.push(c),
            }
        }
        Err(self.error("unterminated string"))
    }

    fn skip_number(&mut self) {
        let rest = self.rest();
        self.position += rest
            .char_indices()
            .skip(1)
            .find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '.'))
            .map_or(rest.len(), |(offset, _)| offset);
    }
}

/// The closing bracket of a long bracket (`[[`, `[=[`, …) opening `text`.
fn long_bracket_close(text: &str) -> Option<String> {
    let after_open = text.strip_prefix('[')?;
    let level = after_open.len() - after_open.trim_start_matches('=').len();
    after_open[level..]
        .starts_with('[')
        .then(|| format!("]{}]", "=".repeat(level)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nested_tables_strings_and_comments() {
        let table = parse_returned_table(
            "-- list\n--[==[ long\ncomment ]==]\nreturn {\n  ['c-sharp'] = { url = \"x\\\"y\", tier = 2, on = true },\n  'positional'; -- trailing\n}\n",
        )
        .unwrap();
        let entry = table.field("c-sharp").and_then(Value::as_table).unwrap();
        assert_eq!(entry.string("url"), Some("x\"y"));
        assert!(matches!(entry.field("on"), Some(Value::Other)));
        assert!(matches!(&table.positional[..], [Value::String(text)] if text == "positional"));
    }

    #[test]
    fn rejects_code() {
        assert!(parse_returned_table("return require('x')").is_err());
        assert!(parse_returned_table("local t = {}").is_err());
    }
}
