//! Localized string tables (`*.eng`): UTF-16 text made of
//! `FIELD_ID="KEY"` / `TEXT="value"` records.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

use super::ini::decode_text;
use crate::text::TextKey;

#[derive(Debug, Default, Clone)]
pub struct Strings {
    map: HashMap<String, String>,
}

impl Strings {
    pub fn parse_into(&mut self, text: &str) {
        let mut field: Option<String> = None;
        let mut rest = text;
        while let Some(eq) = rest.find("=\"") {
            let key = rest[..eq]
                .rsplit(|c: char| c.is_whitespace())
                .next()
                .unwrap_or("")
                .to_ascii_uppercase();
            let after = &rest[eq + 2..];
            // Values may span lines; they end at the first quote that is
            // followed by a line break or the end of the file.
            let mut end = after.len();
            let bytes = after.as_bytes();
            for (i, &b) in bytes.iter().enumerate() {
                if b == b'"' {
                    let next = bytes.get(i + 1).copied();
                    if matches!(next, None | Some(b'\r') | Some(b'\n')) {
                        end = i;
                        break;
                    }
                }
            }
            let value = &after[..end];
            match key.as_str() {
                "FIELD_ID" => field = Some(value.trim().to_ascii_uppercase()),
                "TEXT" => {
                    if let Some(f) = field.take() {
                        self.map.insert(f, value.replace("\\n", "\n"));
                    }
                }
                _ => {}
            }
            rest = &after[(end + 1).min(after.len())..];
        }
    }

    pub fn load_dir(dir: &Path, ext: &str) -> anyhow::Result<Strings> {
        let mut s = Strings::default();
        let mut files: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case(ext))
            })
            .collect();
        files.sort();
        for f in files {
            let bytes = std::fs::read(&f)?;
            s.parse_into(&decode_text(&bytes));
        }
        Ok(s)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.map.get(&key.to_ascii_uppercase()).map(|s| s.as_str())
    }

    /// A key from the game's data files, falling back to the key itself so
    /// missing strings are visible rather than blank.
    pub fn lookup<'a>(&'a self, key: &'a str) -> &'a str {
        self.get(key).unwrap_or(key)
    }

    /// A text the code names, falling back to its key.
    pub fn text(&self, k: impl TextKey) -> Cow<'_, str> {
        let key = k.key();
        match self.get(&key) {
            Some(s) => Cow::Borrowed(s),
            None => Cow::Owned(key.into_owned()),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_records() {
        let mut s = Strings::default();
        s.parse_into(
            "FILE_VERSION=\"6\"\r\nFIELD_ID=\"A_KEY\"\r\nTEXT=\"Hello \"there\"\"\r\nMAX_LINES=\"1\"\r\nFIELD_ID=\"B\"\r\nTEXT=\"two\r\nlines\"\r\n",
        );
        assert_eq!(s.get("a_key"), Some("Hello \"there\""));
        assert_eq!(s.get("B"), Some("two\r\nlines"));
    }
}
