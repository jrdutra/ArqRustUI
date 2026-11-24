//! Minimal SVG XML reader: elements, attributes, text, comments, CDATA, ignored processing instructions/DOCTYPE and
//! common entities.

/// XML element.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Element {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Element>,
    /// Concatenated direct text for `<style>`.
    pub text: String,
}

impl Element {
    /// Attribute value ignoring namespace prefixes (`xlink:href` = `href`).
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name || key.rsplit(':').next() == Some(name))
            .map(|(_, value)| value.as_str())
    }

    /// Local name without prefix.
    pub fn local_name(&self) -> &str {
        self.name.rsplit(':').next().unwrap_or(&self.name)
    }
}

/// Decodes entities (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`, `&#N;`, `&#xN;`).
pub(crate) fn unescape(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';').filter(|&e| e <= 12) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                u32::from_str_radix(&entity[2..], 16).ok().and_then(char::from_u32)
            }
            _ if entity.starts_with('#') => entity[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Parses document and returns root element.
pub(crate) fn parse(source: &str) -> Result<Element, String> {
    let mut stack: Vec<Element> = vec![Element { name: "#document".into(), ..Default::default() }];
    let mut rest = source.trim_start_matches('\u{feff}');
    while !rest.is_empty() {
        let Some(lt) = rest.find('<') else {
            push_text(&mut stack, rest);
            break;
        };
        push_text(&mut stack, &rest[..lt]);
        rest = &rest[lt..];
        if let Some(body) = rest.strip_prefix("<!--") {
            let end = body.find("-->").ok_or("unterminated comment")?;
            rest = &body[end + 3..];
        } else if let Some(body) = rest.strip_prefix("<![CDATA[") {
            let end = body.find("]]>").ok_or("unterminated CDATA")?;
            if let Some(top) = stack.last_mut() {
                top.text.push_str(&body[..end]);
            }
            rest = &body[end + 3..];
        } else if rest.starts_with("<?") {
            let end = rest.find("?>").ok_or("unterminated processing instruction")?;
            rest = &rest[end + 2..];
        } else if rest.starts_with("<!") {
            // DOCTYPE, possivelmente com subconjunto interno entre colchetes.
            let mut depth = 0i32;
            let mut end = None;
            for (i, c) in rest.char_indices() {
                match c {
                    '[' => depth += 1,
                    ']' => depth -= 1,
                    '>' if depth <= 0 => {
                        end = Some(i);
                        break;
                    }
                    _ => {}
                }
            }
            rest = &rest[end.ok_or("unterminated declaration")? + 1..];
        } else if let Some(body) = rest.strip_prefix("</") {
            let end = body.find('>').ok_or("unterminated closing tag")?;
            let name = body[..end].trim();
            let element = stack.pop().ok_or("closing tag without opening tag")?;
            if element.name != name {
                return Err(format!("esperado </{}>, encontrado </{name}>", element.name));
            }
            stack.last_mut().ok_or("closing tag without opening tag")?.children.push(element);
            rest = &body[end + 1..];
        } else {
            let (element, closed, used) = open_tag(&rest[1..])?;
            rest = &rest[1 + used..];
            if closed {
                stack.last_mut().ok_or("invalid structure")?.children.push(element);
            } else {
                stack.push(element);
            }
        }
    }
    if stack.len() != 1 {
        return Err(format!("element <{}> is not closed", stack.last().map_or("", |e| e.name.as_str())));
    }
    let document = stack.pop().ok_or("empty document")?;
    document.children.into_iter().next().ok_or_else(|| "document without elements".to_string())
}

fn push_text(stack: &mut [Element], text: &str) {
    if let Some(top) = stack.last_mut() {
        if top.name != "#document" && !text.is_empty() {
            top.text.push_str(&unescape(text));
        }
    }
}

/// Parses `name attr="v" ...>` or `.../>`; returns element, closed flag and consumed bytes.
fn open_tag(source: &str) -> Result<(Element, bool, usize), String> {
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' && bytes[i] != b'/' {
        i += 1;
    }
    if i == 0 {
        return Err("tag without name".into());
    }
    let mut element = Element { name: source[..i].to_string(), ..Default::default() };
    loop {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        match bytes.get(i) {
            None => return Err(format!("unterminated <{}>", element.name)),
            Some(b'>') => return Ok((element, false, i + 1)),
            Some(b'/') if bytes.get(i + 1) == Some(&b'>') => return Ok((element, true, i + 2)),
            _ => {}
        }
        let start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'>'
            && bytes[i] != b'/'
        {
            i += 1;
        }
        let key = source[start..i].to_string();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if bytes.get(i) != Some(&b'=') {
            element.attributes.push((key, String::new()));
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let quote = *bytes.get(i).ok_or("attribute without value")?;
        if quote != b'"' && quote != b'\'' {
            return Err(format!("value of attribute {key} is not quoted"));
        }
        let value_start = i + 1;
        let end = source[value_start..].find(quote as char).ok_or("unterminated quotes")? + value_start;
        element.attributes.push((key, unescape(&source[value_start..end])));
        i = end + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_elements_attributes_and_text() {
        let doc = r##"<?xml version="1.0"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "x" [ <!ENTITY a "b"> ]>
<!-- comentário -->
<svg xmlns="http://www.w3.org/2000/svg" width='10'>
  <g id="a"><rect x="1" y='2'/></g>
  <style><![CDATA[.c{fill:red}]]></style>
  <text>A &amp; B &#x263A; &#65;</text>
  <path d="M0 0" xlink:href="#x"/>
</svg>"##;
        let svg = parse(doc).unwrap();
        assert_eq!(svg.name, "svg");
        assert_eq!(svg.attribute("width"), Some("10"));
        assert_eq!(svg.children.len(), 4);
        assert_eq!(svg.children[0].children[0].attribute("y"), Some("2"));
        assert_eq!(svg.children[1].text, ".c{fill:red}");
        assert_eq!(svg.children[2].text, "A & B ☺ A");
        assert_eq!(svg.children[3].attribute("href"), Some("#x"), "prefixo de espaço de nomes ignorado");
        assert!(parse("<svg><g></svg>").is_err());
        assert!(parse("<svg").is_err());
        assert!(parse("").is_err());
        assert_eq!(unescape("a &unknown; &lt;b&gt;"), "a &unknown; <b>");
    }
}
