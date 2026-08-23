use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

pub struct TextCleaner {
    custom_dict: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for TextCleaner {
    fn default() -> Self {
        Self::new()
    }
}

impl TextCleaner {
    pub fn new() -> Self {
        let mut cleaner = Self {
            custom_dict: Arc::new(Mutex::new(HashMap::new())),
        };
        cleaner.load_custom_dict();
        cleaner
    }

    fn get_dict_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("VoiceTyping")
            .join("custom_dict.json")
    }

    pub fn load_custom_dict(&mut self) {
        let path = Self::get_dict_path();
        if path.exists() {
            if let Ok(mut file) = File::open(&path) {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    if let Ok(dict) = serde_json::from_str::<HashMap<String, String>>(&content) {
                        *self.custom_dict.lock() = dict;
                    }
                }
            }
        }
    }

    pub fn save_custom_dict(&self, dict: HashMap<String, String>) -> Result<(), String> {
        let path = Self::get_dict_path();
        if let Some(parent) = path.parent() {
            let _ = create_dir_all(parent);
        }

        let json = serde_json::to_string_pretty(&dict).map_err(|e| e.to_string())?;
        let mut file = File::create(&path).map_err(|e| e.to_string())?;
        file.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        *self.custom_dict.lock() = dict;
        Ok(())
    }

    pub fn get_custom_dict(&self) -> HashMap<String, String> {
        self.custom_dict.lock().clone()
    }

    pub fn clean(&self, input: &str) -> String {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        let mut text = trimmed.to_string();

        // 1. User custom dictionary replacements (if any)
        let user_rules = self.custom_dict.lock().clone();
        if !user_rules.is_empty() {
            let mut sorted_keys: Vec<String> = user_rules.keys().cloned().collect();
            sorted_keys.sort_by_key(|k| std::cmp::Reverse(k.len()));

            for key in sorted_keys {
                if let Some(target) = user_rules.get(&key) {
                    text = replace_case_insensitive(&text, &key, target);
                }
            }
        }

        // 2. Punctuation spacing normalization (e.g. "halo , apa kabar ?" -> "halo, apa kabar?")
        let mut normalized = String::new();
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            if chars[i] == ' ' && i + 1 < len {
                let next = chars[i + 1];
                if next == ',' || next == '.' || next == '?' || next == '!' || next == ':' || next == ';' {
                    i += 1;
                    continue;
                }
            }
            normalized.push(chars[i]);
            i += 1;
        }

        // 3. Normalize multiple spaces into single space
        let words: Vec<&str> = normalized.split_whitespace().collect();
        let merged = words.join(" ");

        // 4. Capitalize first letter of the sentence
        let mut final_chars = merged.chars();
        let first = match final_chars.next() {
            None => return String::new(),
            Some(c) => c.to_uppercase().collect::<String>(),
        };

        let rest: String = final_chars.collect();
        format!("{}{}", first, rest)
    }
}

fn replace_case_insensitive(text: &str, from: &str, to: &str) -> String {
    let lower_text = text.to_lowercase();
    let lower_from = from.to_lowercase();
    let mut result = String::new();
    let mut last_idx = 0;

    while let Some(match_idx) = lower_text[last_idx..].find(&lower_from) {
        let abs_idx = last_idx + match_idx;
        result.push_str(&text[last_idx..abs_idx]);
        result.push_str(to);
        last_idx = abs_idx + from.len();
    }

    result.push_str(&text[last_idx..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleaner_rules() {
        let cleaner = TextCleaner::new();
        let res = cleaner.clean("halo , ini adalah pengujian voice typing .");
        assert_eq!(res, "Halo, ini adalah pengujian voice typing.");
    }
}
