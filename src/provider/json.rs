//! The JSON subset a provider manifest uses: objects, arrays, strings,
//! non-negative integers, booleans and null. Std-only, bounded.
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(u64),
    String(String),
    Array(Vec<Value>),
    /// Members in order; keys are unique.
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn str(&self, key: &str) -> Result<&str, String> {
        match self.get(key) {
            Some(Value::String(s)) => Ok(s),
            _ => Err(format!("manifest lacks the string {key:?}")),
        }
    }

    /// Pretty-printed with two-space indentation and a final newline.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    fn write(&self, out: &mut String, depth: usize) {
        let indent = |out: &mut String, depth: usize| out.push_str(&"  ".repeat(depth));
        match self {
            Value::Null => out.push_str("null"),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Integer(n) => write!(out, "{n}").expect("String write"),
            Value::String(s) => quote(out, s),
            Value::Array(items) if items.is_empty() => out.push_str("[]"),
            Value::Object(members) if members.is_empty() => out.push_str("{}"),
            Value::Array(items) => {
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    indent(out, depth + 1);
                    item.write(out, depth + 1);
                    out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
                }
                indent(out, depth);
                out.push(']');
            }
            Value::Object(members) => {
                out.push_str("{\n");
                for (i, (key, value)) in members.iter().enumerate() {
                    indent(out, depth + 1);
                    quote(out, key);
                    out.push_str(": ");
                    value.write(out, depth + 1);
                    out.push_str(if i + 1 < members.len() { ",\n" } else { "\n" });
                }
                indent(out, depth);
                out.push('}');
            }
        }
    }
}

fn quote(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => write!(out, "\\u{:04x}", c as u32).expect("String write"),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Parses one JSON value; at most 64 levels deep.
pub fn parse(text: &str) -> Result<Value, String> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        at: 0,
    };
    let value = parser.value(0)?;
    parser.space();
    if parser.at != parser.bytes.len() {
        return Err("trailing characters after JSON value".into());
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn space(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| matches!(b, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.at += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        self.space();
        if self.bytes.get(self.at) != Some(&byte) {
            return Err(format!(
                "expected {:?} at byte {} of the JSON",
                byte as char, self.at
            ));
        }
        self.at += 1;
        Ok(())
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value, String> {
        if self.bytes[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(format!("invalid JSON at byte {}", self.at))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > 64 {
            return Err("JSON nested too deeply".into());
        }
        self.space();
        match self.bytes.get(self.at) {
            Some(b'{') => {
                self.at += 1;
                let mut members: Vec<(String, Value)> = Vec::new();
                self.space();
                if self.bytes.get(self.at) == Some(&b'}') {
                    self.at += 1;
                    return Ok(Value::Object(members));
                }
                loop {
                    self.space();
                    let key = self.string()?;
                    if members.iter().any(|(k, _)| *k == key) {
                        return Err(format!("duplicate JSON key {key:?}"));
                    }
                    self.expect(b':')?;
                    members.push((key, self.value(depth + 1)?));
                    self.space();
                    match self.bytes.get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b'}') => {
                            self.at += 1;
                            return Ok(Value::Object(members));
                        }
                        _ => return Err(format!("invalid JSON object at byte {}", self.at)),
                    }
                }
            }
            Some(b'[') => {
                self.at += 1;
                let mut items = Vec::new();
                self.space();
                if self.bytes.get(self.at) == Some(&b']') {
                    self.at += 1;
                    return Ok(Value::Array(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    self.space();
                    match self.bytes.get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b']') => {
                            self.at += 1;
                            return Ok(Value::Array(items));
                        }
                        _ => return Err(format!("invalid JSON array at byte {}", self.at)),
                    }
                }
            }
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b'0'..=b'9') => {
                let start = self.at;
                while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                    self.at += 1;
                }
                let text = std::str::from_utf8(&self.bytes[start..self.at]).expect("ASCII");
                if text.len() > 1 && text.starts_with('0') {
                    return Err("JSON numbers have no leading zeros".into());
                }
                text.parse()
                    .map(Value::Integer)
                    .map_err(|_| "JSON integer out of range".into())
            }
            _ => Err(format!("invalid JSON at byte {}", self.at)),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if self.bytes.get(self.at) != Some(&b'"') {
            return Err(format!("expected a JSON string at byte {}", self.at));
        }
        self.at += 1;
        let mut out = String::new();
        loop {
            let rest = std::str::from_utf8(&self.bytes[self.at..])
                .map_err(|_| "JSON is not UTF-8".to_string())?;
            let mut chars = rest.chars();
            let c = chars.next().ok_or("unterminated JSON string")?;
            self.at += c.len_utf8();
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let escape = chars.next().ok_or("unterminated JSON escape")?;
                    self.at += 1;
                    out.push(match escape {
                        '"' => '"',
                        '\\' => '\\',
                        '/' => '/',
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        'b' => '\u{8}',
                        'f' => '\u{c}',
                        'u' => {
                            let hex = rest.get(2..6).ok_or("short JSON \\u escape")?;
                            self.at += 4;
                            u32::from_str_radix(hex, 16)
                                .ok()
                                .and_then(char::from_u32)
                                .ok_or("unsupported JSON \\u escape")?
                        }
                        _ => return Err("invalid JSON escape".into()),
                    });
                }
                c if (c as u32) < 0x20 => return Err("control character in JSON string".into()),
                c => out.push(c),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_manifests() {
        let value = Value::Object(vec![
            ("mode".into(), Value::String("source \"x\"\n\\ é".into())),
            ("size".into(), Value::Integer(1234)),
            (
                "files".into(),
                Value::Array(vec![
                    Value::Object(vec![("ok".into(), Value::Bool(true))]),
                    Value::Null,
                    Value::Array(vec![]),
                ]),
            ),
        ]);
        let text = value.to_text();
        assert_eq!(parse(&text).unwrap(), value);
        assert_eq!(value.str("mode").unwrap(), "source \"x\"\n\\ é");
        assert!(value.str("size").is_err());
    }

    #[test]
    fn rejects_invalid_json() {
        for bad in [
            "",
            "{",
            "{\"a\":1,}",
            "{\"a\":1,\"a\":2}",
            "[1 2]",
            "01",
            "-1",
            "1.5",
            "\"\u{1}\"",
            "\"\\x\"",
            "nul",
            "{} x",
            &"[".repeat(100),
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
    }
}
