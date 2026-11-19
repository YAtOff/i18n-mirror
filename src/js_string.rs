//! Decoding and encoding of JavaScript string literals.
//!
//! The extractor reads string literals from the original source (single quotes,
//! double quotes, or template literals without substitutions) and must re-emit
//! them as double-quoted literals in the dummy mirror files. To guarantee the
//! msgid that lands in the `.po` file is byte-identical to what the running
//! application would display, we fully *decode* the source literal (interpreting
//! escape sequences exactly like the JS engine does) and then *re-encode* it
//! with minimal, always-safe escaping.

/// Decode the raw inner text of a JS string literal or template literal into
/// the string value it denotes at runtime.
///
/// `raw` is the text between the quotes/backticks (source bytes, escapes still
/// present). `template` marks template-literal semantics (raw newlines allowed
/// and normalized to `\n`).
///
/// Returns `None` when the literal contains a malformed escape sequence; the
/// caller then skips the call site with a warning rather than risking a
/// silently-wrong msgid.
pub fn decode(raw: &str, template: bool) -> Option<String> {
    let bytes = raw.as_bytes();
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' {
            i += 1;
            if i >= bytes.len() {
                return None; // dangling backslash: invalid in every context
            }
            let esc = bytes[i];
            i += 1;
            match esc {
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                b'b' => out.push('\u{8}'),
                b'f' => out.push('\u{c}'),
                b'v' => out.push('\u{b}'),
                b'0' => {
                    // \0 is NUL only when not followed by a digit
                    if i < bytes.len() && bytes[i].is_ascii_digit() {
                        return None; // legacy octal: refuse rather than guess
                    }
                    out.push('\0');
                }
                b'x' => {
                    let hex = raw.get(i..i + 2)?;
                    let v = u8::from_str_radix(hex, 16).ok()?;
                    i += 2;
                    out.push(v as char);
                }
                b'u' => {
                    if i < bytes.len() && bytes[i] == b'{' {
                        let end = raw[i..].find('}')? + i;
                        let hex = raw.get(i + 1..end)?;
                        let v = u32::from_str_radix(hex, 16).ok()?;
                        let c = char::from_u32(v)?;
                        // lone surrogates cannot appear in a Rust String
                        if v > 0x10FFFF || (0xD800..=0xDFFF).contains(&v) {
                            return None;
                        }
                        i = end + 1;
                        out.push(c);
                    } else {
                        let hex = raw.get(i..i + 4)?;
                        let v = u32::from_str_radix(hex, 16).ok()?;
                        let c = char::from_u32(v)?;
                        if (0xD800..=0xDFFF).contains(&v) {
                            return None;
                        }
                        i += 4;
                        out.push(c);
                    }
                }
                b'\n' => {} // line continuation
                b'\r' => {
                    // CRLF line continuation
                    if i < bytes.len() && bytes[i] == b'\n' {
                        i += 1;
                    }
                }
                other => out.push(other as char), // \' \" \` \\ and identity escapes
            }
        } else if b == b'\r' {
            // Line terminators normalize to \n (matters for templates; plain
            // strings cannot contain raw newlines but harmless either way).
            out.push('\n');
            i += 1;
            if i < bytes.len() && bytes[i] == b'\n' {
                i += 1;
            }
        } else if b == b'\n' {
            if !template {
                return None; // raw newline inside a quoted string: parse error
            }
            out.push('\n');
            i += 1;
        } else {
            // Copy one UTF-8 scalar. Find its length by advancing until the
            // next ASCII byte boundary start.
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i] & 0b1100_0000) == 0b1000_0000 {
                i += 1;
            }
            out.push_str(raw.get(start..i)?);
        }
    }
    Some(out)
}

/// Encode a decoded string value as a double-quoted JS string literal body
/// (without the quotes).
pub fn encode_body(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 8);
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\u{b}' => out.push_str("\\v"),
            '\u{0}' => out.push_str("\\u0000"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// Convenience: value -> full quoted literal, e.g. `"Save changes?"`.
pub fn encode(value: &str) -> String {
    format!("\"{}\"", encode_body(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_plain_and_escaped() {
        assert_eq!(decode("plain", false).unwrap(), "plain");
        assert_eq!(decode("a\\nb", false).unwrap(), "a\nb");
        assert_eq!(decode("\\t\\r\\b\\f\\v", false).unwrap(), "\t\r\u{8}\u{c}\u{b}");
        assert_eq!(decode("it\\'s", false).unwrap(), "it's");
        assert_eq!(decode("say \\\"hi\\\"", false).unwrap(), "say \"hi\"");
        assert_eq!(decode("back\\\\slash", false).unwrap(), "back\\slash");
        assert_eq!(decode("\\x41\\x42", false).unwrap(), "AB");
        assert_eq!(decode("\\u00e9\\u00e8", false).unwrap(), "éè");
        assert_eq!(decode("\\u{1F600}", false).unwrap(), "\u{1F600}");
        assert_eq!(decode("line\\\ncont", false).unwrap(), "linecont");
        assert_eq!(decode("identity \\q", false).unwrap(), "identity q");
    }

    #[test]
    fn decode_template_newlines() {
        assert_eq!(decode("a\nb", true).unwrap(), "a\nb");
        assert_eq!(decode("a\r\nb", true).unwrap(), "a\nb");
        assert!(decode("a\nb", false).is_none(), "raw newline invalid in quoted string");
    }

    #[test]
    fn decode_malformed() {
        assert!(decode("\\xZZ", false).is_none());
        assert!(decode("\\u12", false).is_none());
        assert!(decode("\\u{110000}", false).is_none());
        assert!(decode("trailing\\", false).is_none());
    }

    #[test]
    fn decode_multibyte_boundaries() {
        assert_eq!(decode("здравей", false).unwrap(), "здравей");
        assert_eq!(decode("\\u4e2d\\u6587", false).unwrap(), "中文");
    }

    #[test]
    fn encode_round_trip() {
        for s in [
            "Save",
            "it's \"quoted\"",
            "back\\slash",
            "tab\there",
            "mul\nti\nline",
            "здравейте, свят! \u{1F600}",
            "ctrl\u{1}\u{7f}",
        ] {
            let lit = encode(s);
            let inner = &lit[1..lit.len() - 1];
            assert_eq!(decode(inner, false).unwrap(), s, "round trip failed for {s:?}");
        }
    }
}
