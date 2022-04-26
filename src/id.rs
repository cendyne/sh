use crate::settings::*;

pub fn increment_string(alphabet: &str, symbols: &str) -> String {
    let mut result: Vec<char> = Vec::with_capacity(symbols.len() + 1);
    let mut carry = 1;
    let length = alphabet.len();
    symbols.chars().rev().for_each(|c| {
        if carry == 0 {
            result.push(c);
        } else if let Some(value) = alphabet.find(c) {
            if value == length - 1 {
                if let Some(first) = alphabet.chars().next() {
                    result.push(first);
                }
            } else if let Some(next) = alphabet.chars().nth(value + 1) {
                result.push(next);
                carry = 0;
            }
        } else {
            println!("Unsupported character {}, using last", c);
            if let Some(last) = alphabet.chars().last() {
                result.push(last);
            }
        }
    });

    if carry != 0 {
        if let Some(first) = alphabet.chars().next() {
            result.push(first);
        }
    }

    result.reverse();
    result.iter().collect()
}

pub fn next_symbol(settings: &JsonSettings, alphabet: &str) -> Option<String> {
    settings
        .last_symbol
        .lock()
        .map(|mut symbol| {
            let mut last = (*symbol).clone();

            loop {
                let next_symbol = increment_string(alphabet, &last);

                if settings.redirects.contains_key(&next_symbol) {
                    last = next_symbol;
                    continue;
                }

                *symbol = next_symbol.clone();
                break next_symbol;
            }
        })
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashmap::DashMap;
    use std::sync::Mutex;
    const NUMBERS: &'static str = "0123456789";
    #[test]
    fn increment_none() {
        assert_eq!(increment_string(NUMBERS, ""), "0");
    }

    #[test]
    fn increment_some() {
        assert_eq!(increment_string(NUMBERS, "0"), "1");
        assert_eq!(increment_string(NUMBERS, "1"), "2");
        assert_eq!(increment_string(NUMBERS, "8"), "9");
    }

    #[test]
    fn increment_two_digits() {
        assert_eq!(increment_string(NUMBERS, "9"), "00");
        assert_eq!(increment_string(NUMBERS, "00"), "01");
        assert_eq!(increment_string(NUMBERS, "09"), "10");
        assert_eq!(increment_string(NUMBERS, "89"), "90");
        assert_eq!(increment_string(NUMBERS, "98"), "99");
    }

    #[test]
    fn increment_three_digits() {
        assert_eq!(increment_string(NUMBERS, "99"), "000");
        assert_eq!(increment_string(NUMBERS, "000"), "001");
        assert_eq!(increment_string(NUMBERS, "099"), "100");
        assert_eq!(increment_string(NUMBERS, "899"), "900");
        assert_eq!(increment_string(NUMBERS, "998"), "999");
    }

    #[test]
    fn next_symbol_test1() {
        let settings = JsonSettings {
            redirects: DashMap::new(),
            last_symbol: Mutex::new("".to_string()),
        };
        assert_eq!(next_symbol(&settings, NUMBERS), Some("0".to_string()));
        settings.redirects.insert("1".to_string(), "".to_string());
        // It does not overwrite
        assert_eq!(next_symbol(&settings, NUMBERS), Some("2".to_string()));
    }
    #[test]
    fn next_symbol_test2() {
        let settings = JsonSettings {
            redirects: DashMap::new(),
            last_symbol: Mutex::new("".to_string()),
        };
        settings.redirects.insert("0".to_string(), "".to_string());
        settings.redirects.insert("1".to_string(), "".to_string());
        settings.redirects.insert("2".to_string(), "".to_string());
        // It does not overwrite
        assert_eq!(next_symbol(&settings, NUMBERS), Some("3".to_string()));
        // Even if not written, it won't go back
        assert_eq!(next_symbol(&settings, NUMBERS), Some("4".to_string()));
    }
}
