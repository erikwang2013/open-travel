use serde_json::Value;

/// 解析字符串字面量 token（含引号）。双引号走 serde_json 解码转义；
/// 单引号手动处理（serde_json 不支持单引号）。
pub(super) fn parse_string_literal(token: &[u8]) -> Result<Value, String> {
    if token.first() == Some(&b'"') {
        return serde_json::from_slice(token).map_err(|e| format!("invalid string literal: {e}"));
    }
    let inner = &token[1..token.len() - 1];
    let mut out = Vec::with_capacity(inner.len());
    let mut it = inner.iter().peekable();
    while let Some(&c) = it.next() {
        if c != b'\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some(b'\\') => out.push(b'\\'),
            Some(b'"') => out.push(b'"'),
            Some(b'\'') => out.push(b'\''),
            Some(b'n') => out.push(b'\n'),
            Some(b't') => out.push(b'\t'),
            Some(b'r') => out.push(b'\r'),
            Some(b'b') => out.push(0x08),
            Some(b'f') => out.push(0x0c),
            Some(b'u') => {
                let hex: String = it.by_ref().take(4).map(|c| *c as char).collect();
                if hex.len() != 4 {
                    return Err("invalid unicode escape".into());
                }
                let code = u32::from_str_radix(&hex, 16)
                    .map_err(|_| "invalid unicode escape".to_string())?;
                if let Some(ch) = char::from_u32(code) {
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                } else {
                    return Err("invalid unicode escape".into());
                }
            }
            _ => return Err("invalid escape in string literal".into()),
        }
    }
    Ok(Value::String(String::from_utf8_lossy(&out).into_owned()))
}

/// 跳过字符串字面量（单/双引号，支持反斜杠转义与三引号块字符串）。
pub(super) fn skip_string(b: &[u8], i: &mut usize) -> Result<(), String> {
    let quote = b[*i];
    *i += 1;
    if *i + 1 < b.len() && b[*i] == quote && b[*i + 1] == quote {
        *i += 2;
        while *i < b.len() {
            if b[*i] == quote {
                if *i + 2 < b.len() && b[*i + 1] == quote && b[*i + 2] == quote {
                    *i += 3;
                    return Ok(());
                }
                *i += 1;
            } else {
                *i += 1;
            }
        }
        return Err("unterminated string literal".into());
    }
    while *i < b.len() {
        match b[*i] {
            b'\\' => {
                if *i + 1 < b.len() {
                    *i += 2;
                } else {
                    return Err("unterminated string literal".into());
                }
            }
            c if c == quote => {
                *i += 1;
                return Ok(());
            }
            _ => *i += 1,
        }
    }
    Err("unterminated string literal".into())
}
