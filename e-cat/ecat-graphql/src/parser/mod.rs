// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
//! 手写 GraphQL 选择集解析器（两阶段）：
//! - Phase A 扫描：定位顶层字段名、参数原始区间与嵌套 selection 树
//! - Phase B 参数区间二次解析：字面量/枚举/列表/输入对象/变量解引用

mod strings;

use serde_json::{Map, Value};
use std::collections::HashMap;
use strings::{parse_string_literal, skip_string};

pub const MAX_DEPTH: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Query,
    Mutation,
    Subscription,
}

#[derive(Debug, PartialEq)]
pub struct ParsedField {
    pub operation: Operation,
    pub name: String,
    pub args: Map<String, Value>,
    pub selection: Option<SelectionSet>,
}

pub type SelectionSet = HashMap<String, FieldNode>;

#[derive(Debug, Clone, PartialEq)]
pub struct FieldNode {
    pub args: Map<String, Value>,
    pub selection: Option<SelectionSet>,
}

pub fn parse_query(query: &str, variables: &Value) -> Result<ParsedField, String> {
    let b = query.as_bytes();
    let mut i = 0;
    let n = b.len();

    skip_ws(b, &mut i);

    let mut operation = Operation::Query;
    for (kw, op) in [
        ("query".as_bytes(), Operation::Query),
        ("mutation".as_bytes(), Operation::Mutation),
        ("subscription".as_bytes(), Operation::Subscription),
    ] {
        if b[i..].starts_with(kw) {
            let after = i + kw.len();
            if after >= n || b[after].is_ascii_whitespace() || b[after] == b'(' || b[after] == b'{'
            {
                operation = op;
                i = after;
                break;
            }
        }
    }
    skip_ws(b, &mut i);

    if i < n && is_ident_byte(b[i]) {
        while i < n && is_ident_byte(b[i]) {
            i += 1;
        }
        skip_ws(b, &mut i);
    }

    // 变量定义与指令
    loop {
        if i < n && b[i] == b'(' {
            skip_parens(b, &mut i)?;
            skip_ws(b, &mut i);
        } else if i < n && b[i] == b'@' {
            skip_directive(b, &mut i)?;
            skip_ws(b, &mut i);
        } else {
            break;
        }
    }

    if i >= n || b[i] != b'{' {
        return Err("expected '{' before selection".into());
    }
    i += 1;
    skip_ws(b, &mut i);

    if i >= n || b[i] == b'}' {
        return Err("empty selection set".into());
    }
    if b[i] == b'.' {
        return Err("fragment spreads are not supported".into());
    }

    let field = parse_field(b, &mut i, 0, operation, variables)?;

    skip_ws(b, &mut i);
    if i >= n {
        return Err("expected '}'".into());
    }
    match b[i] {
        b'}' => {}
        b'.' => return Err("fragment spreads are not supported".into()),
        _ => return Err("multiple top-level fields not supported".into()),
    }
    Ok(field)
}

/// 解析单个字段：名、别名检测、参数区间、嵌套 selection。
/// operation 仅顶层字段有意义（嵌套字段转 FieldNode 时丢弃）。
fn parse_field(
    b: &[u8],
    i: &mut usize,
    depth: usize,
    operation: Operation,
    variables: &Value,
) -> Result<ParsedField, String> {
    if depth > MAX_DEPTH {
        return Err("selection too deep".into());
    }
    skip_ws(b, i);
    let start = *i;
    while *i < b.len() && is_ident_byte(b[*i]) {
        *i += 1;
    }
    if *i == start {
        return Err("expected field name".into());
    }
    let name = String::from_utf8_lossy(&b[start..*i]).into_owned();
    skip_ws(b, i);

    if *i < b.len() && b[*i] == b':' {
        return Err("aliases not supported".into());
    }

    let mut args = Map::new();
    if *i < b.len() && b[*i] == b'(' {
        let args_start = *i;
        skip_parens(b, i)?;
        args = parse_args(&b[args_start..*i], variables, depth)?;
        skip_ws(b, i);
    }

    // 字段级指令：Phase A 忽略（与操作级指令一致）
    while *i < b.len() && b[*i] == b'@' {
        skip_directive(b, i)?;
        skip_ws(b, i);
    }

    let selection = if *i < b.len() && b[*i] == b'{' {
        Some(parse_selection_set(b, i, depth + 1, variables)?)
    } else {
        None
    };
    Ok(ParsedField {
        operation,
        name,
        args,
        selection,
    })
}

fn parse_selection_set(
    b: &[u8],
    i: &mut usize,
    depth: usize,
    variables: &Value,
) -> Result<SelectionSet, String> {
    if depth > MAX_DEPTH {
        return Err("selection too deep".into());
    }
    // *i 停在 '{'
    *i += 1;
    skip_ws(b, i);
    if *i >= b.len() || b[*i] == b'}' {
        return Err("empty selection set".into());
    }
    let mut set = SelectionSet::new();
    loop {
        if *i >= b.len() {
            return Err("expected '}'".into());
        }
        match b[*i] {
            b'}' => {
                *i += 1;
                return Ok(set);
            }
            b'.' => return Err("fragment spreads are not supported".into()),
            _ => {
                let f = parse_field(b, i, depth, Operation::Query, variables)?;
                set.insert(
                    f.name,
                    FieldNode {
                        args: f.args,
                        selection: f.selection,
                    },
                );
                skip_ws(b, i);
            }
        }
    }
}

fn skip_directive(b: &[u8], i: &mut usize) -> Result<(), String> {
    *i += 1; // '@'
    while *i < b.len() && is_ident_byte(b[*i]) {
        *i += 1;
    }
    skip_ws(b, i);
    if *i < b.len() && b[*i] == b'(' {
        skip_parens(b, i)?;
    }
    Ok(())
}

/// Phase B：解析 "(name: value, ...)" 参数区间（含首尾括号）。
fn parse_args(raw: &[u8], variables: &Value, depth: usize) -> Result<Map<String, Value>, String> {
    let b = raw;
    let mut i = 0;
    let n = b.len();
    let mut args = Map::new();
    skip_ws(b, &mut i);
    if i >= n || b[i] != b'(' {
        return Err("expected '(' in arguments".into());
    }
    i += 1;
    loop {
        skip_ws(b, &mut i);
        if i >= n {
            return Err("expected ')' in arguments".into());
        }
        if b[i] == b')' {
            return Ok(args);
        }
        let start = i;
        while i < n && is_ident_byte(b[i]) {
            i += 1;
        }
        if i == start {
            return Err("expected argument name".into());
        }
        let name = String::from_utf8_lossy(&b[start..i]).into_owned();
        skip_ws(b, &mut i);
        if i >= n || b[i] != b':' {
            return Err(format!("expected ':' after argument '{name}'"));
        }
        i += 1;
        skip_ws(b, &mut i);
        let value = parse_value(b, &mut i, variables, depth)?;
        if args.contains_key(&name) {
            return Err(format!("duplicate argument '{name}'"));
        }
        args.insert(name, value);
        skip_ws(b, &mut i);
        if i < n && b[i] == b',' {
            i += 1;
        }
    }
}

/// 解析单个值。variables 为 Null 时视为空对象（$var 缺失即报错）。
pub fn parse_value(
    b: &[u8],
    i: &mut usize,
    variables: &Value,
    depth: usize,
) -> Result<Value, String> {
    if depth > MAX_DEPTH {
        return Err("selection too deep".into());
    }
    skip_ws(b, i);
    if *i >= b.len() {
        return Err("expected value".into());
    }
    match b[*i] {
        b'"' | b'\'' => {
            // 三连引号（""" 与 '''）都是 block string 语法，显式拒绝
            if *i + 2 < b.len() && b[*i + 1] == b[*i] && b[*i + 2] == b[*i] {
                return Err("block string value not supported".into());
            }
            let start = *i;
            skip_string(b, i)?;
            parse_string_literal(&b[start..*i])
        }
        b'[' => {
            let start = *i;
            skip_balanced(b, i, b'[', b']')?;
            parse_list(&b[start..*i], variables, depth)
        }
        b'{' => {
            let start = *i;
            skip_balanced(b, i, b'{', b'}')?;
            parse_input_object(&b[start..*i], variables, depth)
        }
        b'$' => {
            *i += 1;
            let start = *i;
            while *i < b.len() && is_ident_byte(b[*i]) {
                *i += 1;
            }
            let name = String::from_utf8_lossy(&b[start..*i]).into_owned();
            match variables {
                Value::Object(m) => m
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| format!("variable '{name}' is not provided")),
                _ => Err(format!("variable '{name}' is not provided")),
            }
        }
        _ => {
            // 字面量 token：数字 / true / false / null / 枚举标识符
            let start = *i;
            while *i < b.len()
                && !b[*i].is_ascii_whitespace()
                && !matches!(b[*i], b',' | b')' | b']' | b'}')
            {
                *i += 1;
            }
            let token = &b[start..*i];
            match serde_json::from_slice::<Value>(token) {
                Ok(v) => Ok(v),
                Err(_) => {
                    if token.iter().all(|c| is_ident_byte(*c))
                        && !token.first().is_some_and(|c| c.is_ascii_digit())
                    {
                        // 裸标识符：无类型系统，枚举值映射为字符串
                        Ok(Value::String(String::from_utf8_lossy(token).into_owned()))
                    } else {
                        Err(format!(
                            "invalid value '{}'",
                            String::from_utf8_lossy(token)
                        ))
                    }
                }
            }
        }
    }
}

/// 解析 "[item, ...]" 列表（含首尾括号）：顶层逗号切分后递归解析各项。
fn parse_list(raw: &[u8], variables: &Value, depth: usize) -> Result<Value, String> {
    let mut items = Vec::new();
    for item in split_top_level(&raw[1..raw.len() - 1]) {
        if item.iter().all(|c| c.is_ascii_whitespace()) {
            continue;
        }
        let mut i = 0;
        items.push(parse_value(item, &mut i, variables, depth + 1)?);
    }
    Ok(Value::Array(items))
}

/// 解析 "{name: value, ...}" 输入对象（含首尾大括号）。
fn parse_input_object(raw: &[u8], variables: &Value, depth: usize) -> Result<Value, String> {
    let mut map = Map::new();
    for item in split_top_level(&raw[1..raw.len() - 1]) {
        if item.iter().all(|c| c.is_ascii_whitespace()) {
            continue;
        }
        let mut i = 0;
        skip_ws(item, &mut i);
        let start = i;
        while i < item.len() && is_ident_byte(item[i]) {
            i += 1;
        }
        if i == start {
            if matches!(item.get(start), Some(b'"') | Some(b'\'')) {
                return Err("quoted field names in input objects not supported".into());
            }
            return Err("expected field name in input object".into());
        }
        let name = String::from_utf8_lossy(&item[start..i]).into_owned();
        skip_ws(item, &mut i);
        if i >= item.len() || item[i] != b':' {
            return Err(format!("expected ':' after '{name}' in input object"));
        }
        i += 1;
        let value = parse_value(item, &mut i, variables, depth + 1)?;
        if map.contains_key(&name) {
            return Err(format!("duplicate argument '{name}'"));
        }
        map.insert(name, value);
    }
    Ok(Value::Object(map))
}

/// 按顶层逗号切分（括号/大括号组与字符串内的逗号不算）。
fn split_top_level(raw: &[u8]) -> Vec<&[u8]> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut round = 0usize;
    let mut square = 0usize;
    let mut curly = 0usize;
    let mut i = 0;
    while i < raw.len() {
        match raw[i] {
            b'"' | b'\'' => {
                let mut j = i;
                let _ = skip_string(raw, &mut j);
                i = j;
                continue;
            }
            b'(' => round += 1,
            b')' => round = round.saturating_sub(1),
            b'[' => square += 1,
            b']' => square = square.saturating_sub(1),
            b'{' => curly += 1,
            b'}' => curly = curly.saturating_sub(1),
            b',' if round == 0 && square == 0 && curly == 0 => {
                parts.push(&raw[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push(&raw[start..]);
    parts
}

/// 跳过平衡的 (/) / [ ] / { } 组，起始字符为 open。
fn skip_balanced(b: &[u8], i: &mut usize, open: u8, close: u8) -> Result<(), String> {
    let mut depth = 0usize;
    while *i < b.len() {
        match b[*i] {
            c if c == open => {
                depth += 1;
                *i += 1;
            }
            c if c == close => {
                depth = depth.saturating_sub(1);
                *i += 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            b'"' | b'\'' => skip_string(b, i)?,
            _ => *i += 1,
        }
    }
    Err("unbalanced value group".into())
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// 跳过空白与 `#` 注释。
fn skip_ws(b: &[u8], i: &mut usize) {
    while *i < b.len() && (b[*i].is_ascii_whitespace() || b[*i] == b'#') {
        if b[*i] == b'#' {
            while *i < b.len() && b[*i] != b'\n' {
                *i += 1;
            }
        } else {
            *i += 1;
        }
    }
}

/// 跳过平衡的括号组；字符串内的括号/大括号不算结构字符。
fn skip_parens(b: &[u8], i: &mut usize) -> Result<(), String> {
    let mut depth = 0usize;
    while *i < b.len() {
        match b[*i] {
            b'(' => {
                depth += 1;
                *i += 1;
            }
            b')' => {
                depth -= 1;
                *i += 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            b'"' | b'\'' => skip_string(b, i)?,
            _ => *i += 1,
        }
    }
    Err("unbalanced parentheses".into())
}

#[cfg(test)]
mod parser_tests;
