// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
//! Minimal `ListBucketResult` XML parsing (quick_xml).

/// Returns (object keys, next continuation token) from a ListObjectsV2
/// response body. Malformed/truncated XML stops parsing and returns what was
/// collected so far.
pub(crate) fn parse_list_xml(xml: &str) -> (Vec<String>, Option<String>) {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut keys = Vec::new();
    let mut token = None;
    let mut in_key = false;
    let mut in_token = false;
    let mut text = String::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"Key" => in_key = true,
            Ok(Event::Start(e)) if e.name().as_ref() == b"NextContinuationToken" => in_token = true,
            Ok(Event::Text(t)) if in_key || in_token => {
                // quick-xml >= 0.37 会在实体引用处拆分文本，需追加而非覆盖
                text.push_str(&t.xml10_content().unwrap_or_default());
            }
            Ok(Event::GeneralRef(r)) if in_key || in_token => {
                // 实体引用以独立事件给出（无 &...; 定界符），按 XML 1.0 预定义实体还原
                match r.as_ref() {
                    b"amp" => text.push('&'),
                    b"lt" => text.push('<'),
                    b"gt" => text.push('>'),
                    b"quot" => text.push('"'),
                    b"apos" => text.push('\''),
                    [b'#', rest @ ..] => {
                        let (radix, digits) = match rest {
                            [b'x', hex @ ..] => (16, hex),
                            dec => (10, dec),
                        };
                        if let Ok(n) =
                            u32::from_str_radix(std::str::from_utf8(digits).unwrap_or(""), radix)
                            && let Some(c) = char::from_u32(n)
                        {
                            text.push(c);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"Key" => {
                keys.push(std::mem::take(&mut text));
                in_key = false;
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"NextContinuationToken" => {
                token = Some(std::mem::take(&mut text));
                in_token = false;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    (keys, token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_xml_parses_keys_and_continuation_token() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <IsTruncated>true</IsTruncated>
  <Contents><Key>logs/2026/01.log</Key></Contents>
  <Contents><Key>logs/2026/02&amp;special.log</Key></Contents>
  <NextContinuationToken>tok==</NextContinuationToken>
</ListBucketResult>"#;
        let (keys, token) = parse_list_xml(xml);
        assert_eq!(keys, vec!["logs/2026/01.log", "logs/2026/02&special.log"]);
        assert_eq!(token.as_deref(), Some("tok=="));
    }

    #[test]
    fn list_xml_without_token_ends_paging() {
        let xml = "<ListBucketResult><Contents><Key>a</Key></Contents></ListBucketResult>";
        let (keys, token) = parse_list_xml(xml);
        assert_eq!(keys, vec!["a"]);
        assert!(token.is_none());
    }
}
