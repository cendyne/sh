use once_cell::sync::OnceCell;

use crate::settings::*;

const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
static CHOSEN_ALPHABET: OnceCell<String> = OnceCell::new();

pub fn chosen_alphabet() -> &'static str {
    CHOSEN_ALPHABET.get_or_init(|| {
        std::env::var("ALPHABET")
            .ok()
            .unwrap_or_else(|| ALPHABET.to_string())
    })
}

static INCREMENT_SECRET: OnceCell<[u8; blake3::OUT_LEN]> = OnceCell::new();

fn load_token_increment_secret() -> Result<[u8; blake3::OUT_LEN], String> {
    let input =
        std::env::var("INCREMENT_SECRET").map_err(|e| format!("INCREMENT_SECRET: {}", e))?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(input.as_bytes());
    let hash = hasher.finalize();
    let hash_bytes = hash.as_bytes();
    Ok(*hash_bytes)
}

struct IncrementedString {
    position: usize,
    next_string: String,
}

impl IncrementedString {
    fn new(
        last_string: &str,
        increment_secret: &[u8; blake3::OUT_LEN],
        alphabet: &str,
    ) -> IncrementedString {
        let mut hasher = blake3::Hasher::new_keyed(increment_secret);
        hasher.update(last_string.as_bytes());
        let hash = hasher.finalize();
        let data = hash.as_bytes();
        IncrementedString {
            position: 2, // Start at 3 (2 + 1) characters instead, reserve single characters for other use
            next_string: data
                .iter()
                .map(|b| {
                    let position = (*b as usize) % alphabet.len();
                    alphabet.chars().nth(position).unwrap_or('_')
                })
                .collect::<String>(),
        }
    }
}

impl Iterator for IncrementedString {
    // we will be counting with usize
    type Item = String;

    // next() is the only required method
    fn next(&mut self) -> Option<Self::Item> {
        // Increment our count.
        self.position += 1;

        // Check to see if we've finished counting or not.
        if self.position < self.next_string.len() {
            Some(self.next_string[..self.position].to_string())
        } else {
            None
        }
    }
}

pub fn next_symbol_by_hash(settings: &JsonSettings, alphabet: &str) -> Result<String, String> {
    let increment_secret = INCREMENT_SECRET.get_or_try_init(load_token_increment_secret)?;
    settings
        .last_symbol
        .lock()
        .map(|mut symbol| {
            let mut inc = IncrementedString::new(&*symbol, increment_secret, alphabet);

            loop {
                match inc.next() {
                    None => {
                        break None;
                    }
                    Some(next_symbol) => {
                        if settings.redirects.contains_key(&next_symbol) {
                            continue;
                        }

                        *symbol = next_symbol.clone();
                        break Some(next_symbol);
                    }
                }
            }
        })
        .map_err(|f| format!("{}", f))
        .and_then(|o| match o {
            Some(o) => Ok(o),
            None => Err("Could not find a next symbol".to_string()),
        })
}
