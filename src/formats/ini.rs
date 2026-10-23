//! Loose INI parser matching the game's data files.
//!
//! The original files mix `//` and `;` comments, allow whitespace around `=`,
//! repeat section names occasionally and sometimes append comments after a
//! value. Section and key lookups are case-insensitive.

use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct Section {
    pub name: String,
    /// Keys in file order (lower-cased), with their raw string values.
    pub entries: Vec<(String, String)>,
}

impl Section {
    pub fn get(&self, key: &str) -> Option<&str> {
        let key = key.to_ascii_lowercase();
        self.entries
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn str_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get(key).unwrap_or(default)
    }

    pub fn f32_or(&self, key: &str, default: f32) -> f32 {
        self.get(key).and_then(parse_f32).unwrap_or(default)
    }

    pub fn i32_or(&self, key: &str, default: i32) -> i32 {
        self.get(key)
            .and_then(|v| parse_f32(v).map(|f| f as i32))
            .unwrap_or(default)
    }

    pub fn bool_or(&self, key: &str, default: bool) -> bool {
        self.get(key)
            .and_then(parse_f32)
            .map(|f| f != 0.0)
            .unwrap_or(default)
    }

    /// Parses a comma separated pair such as `size=8,8`.
    pub fn pair_or(&self, key: &str, default: (i32, i32)) -> (i32, i32) {
        let Some(v) = self.get(key) else { return default };
        let mut it = v.split(',').map(|s| parse_f32(s).map(|f| f as i32));
        match (it.next().flatten(), it.next().flatten()) {
            (Some(a), Some(b)) => (a, b),
            _ => default,
        }
    }
}

/// Parses a number the way the original `atof` calls would: leading numeric
/// prefix, trailing junk ignored (`0.7f`, `40.0f`).
pub fn parse_f32(s: &str) -> Option<f32> {
    let s = s.trim();
    let end = s
        .char_indices()
        .take_while(|(i, c)| {
            c.is_ascii_digit() || *c == '.' || ((*c == '-' || *c == '+') && *i == 0)
        })
        .map(|(i, c)| i + c.len_utf8())
        .last()
        .unwrap_or(0);
    s[..end].parse::<f32>().ok()
}

#[derive(Debug, Default, Clone)]
pub struct Ini {
    pub sections: Vec<Section>,
    index: HashMap<String, usize>,
}

impl Ini {
    pub fn parse(text: &str) -> Ini {
        let mut ini = Ini::default();
        let mut cur: Option<usize> = None;
        for raw in text.lines() {
            let line = strip_comment(raw).trim();
            if line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix('[') {
                let name = rest.split(']').next().unwrap_or("").trim().to_string();
                let key = name.to_ascii_lowercase();
                let idx = match ini.index.get(&key) {
                    Some(&i) => i,
                    None => {
                        ini.sections.push(Section {
                            name,
                            entries: Vec::new(),
                        });
                        let i = ini.sections.len() - 1;
                        ini.index.insert(key, i);
                        i
                    }
                };
                cur = Some(idx);
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let Some(idx) = cur else { continue };
            ini.sections[idx]
                .entries
                .push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
        ini
    }

    pub fn load(path: &std::path::Path) -> anyhow::Result<Ini> {
        let bytes = std::fs::read(path)?;
        Ok(Ini::parse(&decode_text(&bytes)))
    }

    pub fn section(&self, name: &str) -> Option<&Section> {
        self.index
            .get(&name.to_ascii_lowercase())
            .map(|&i| &self.sections[i])
    }
}

fn strip_comment(line: &str) -> &str {
    let mut end = line.len();
    if let Some(i) = line.find("//") {
        end = end.min(i);
    }
    if let Some(i) = line.find(';') {
        end = end.min(i);
    }
    &line[..end]
}

/// Decodes a text file that may be UTF-16LE (with BOM) or Windows-1252.
pub fn decode_text(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    bytes.iter().map(|&b| cp1252(b)).collect()
}

fn cp1252(b: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    match b {
        0x80..=0x9F => HIGH[(b - 0x80) as usize],
        _ => b as char,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_loose_ini() {
        let ini = Ini::parse(
            "[Init]\r\n// comment\r\nseconds_per_sim_day=60\r\nx = 0.7f // tail\r\n;y=1\r\nsize=8,8\r\n",
        );
        let s = ini.section("init").unwrap();
        assert_eq!(s.i32_or("seconds_per_sim_day", 0), 60);
        assert!((s.f32_or("X", 0.0) - 0.7).abs() < 1e-6);
        assert_eq!(s.get("y"), None);
        assert_eq!(s.pair_or("size", (0, 0)), (8, 8));
    }
}
