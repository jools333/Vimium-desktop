use crate::core::element::{HintItem, UIElement};

/// Alphabet of comfortable keys for hint generation (home row + nearby keys)
pub const DEFAULT_HINT_KEYS: &[char] = &[
    'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l',
    'q', 'w', 'e', 'r', 'u', 'i', 'o', 'p',
    'z', 'x', 'c', 'v', 'b', 'n', 'm',
];

/// Converts Russian Cyrillic keyboard characters to their English QWERTY equivalents
pub fn normalize_key(c: char) -> char {
    let lower = c.to_lowercase().next().unwrap_or(c);
    match lower {
        'ф' => 'a',
        'ы' => 's',
        'в' => 'd',
        'а' => 'f',
        'п' => 'g',
        'р' => 'h',
        'о' => 'j',
        'л' => 'k',
        'д' => 'l',
        'й' => 'q',
        'ц' => 'w',
        'у' => 'e',
        'к' => 'r',
        'е' => 't',
        'н' => 'y',
        'г' => 'u',
        'ш' => 'i',
        'щ' => 'o',
        'з' => 'p',
        'я' => 'z',
        'ч' => 'x',
        'с' => 'c',
        'м' => 'v',
        'и' => 'b',
        'т' => 'n',
        'ь' => 'm',
        other => other.to_ascii_lowercase(),
    }
}

/// Generates `count` unique hint strings using the given alphabet.
pub fn generate_hints(count: usize, alphabet: &[char]) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }
    let base = alphabet.len();
    if base == 0 {
        return Vec::new();
    }

    let mut digits = 1;
    let mut capacity = base;
    while capacity < count {
        digits += 1;
        capacity = capacity.saturating_mul(base);
    }

    let mut result = Vec::with_capacity(count);
    for mut n in 0..count {
        let mut chars = Vec::with_capacity(digits);
        for _ in 0..digits {
            let rem = n % base;
            chars.push(alphabet[rem]);
            n /= base;
        }
        chars.reverse();
        result.push(chars.into_iter().collect::<String>());
    }

    result
}

/// Combines elements with generated hints
pub fn assign_hints(elements: Vec<UIElement>) -> Vec<HintItem> {
    let hints = generate_hints(elements.len(), DEFAULT_HINT_KEYS);
    elements
        .into_iter()
        .zip(hints)
        .map(|(element, hint)| HintItem { hint, element })
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
pub enum MatchResult {
    /// Hint was fully matched and uniquely identifies an element
    Selected(UIElement),
    /// Current input matches prefixes of `count` hints
    Narrowed(usize),
    /// No hints match current prefix
    Mismatch,
}

pub struct HintMatcher {
    items: Vec<HintItem>,
    pub input_buffer: String,
}

impl HintMatcher {
    pub fn new(items: Vec<HintItem>) -> Self {
        Self {
            items,
            input_buffer: String::new(),
        }
    }

    #[allow(dead_code)]
    pub fn items(&self) -> &[HintItem] {
        &self.items
    }

    /// Feed a character from user keystroke (normalizes Cyrillic automatically)
    pub fn feed_char(&mut self, c: char) -> MatchResult {
        let normalized = normalize_key(c);
        self.input_buffer.push(normalized);

        // Find all hints that start with current buffer
        let matching: Vec<&HintItem> = self
            .items
            .iter()
            .filter(|item| item.hint.starts_with(&self.input_buffer))
            .collect();

        match matching.len() {
            0 => {
                self.input_buffer.pop();
                MatchResult::Mismatch
            }
            1 => {
                let item = matching[0];
                if item.hint == self.input_buffer || item.hint.len() == self.input_buffer.len() {
                    MatchResult::Selected(item.element.clone())
                } else {
                    MatchResult::Narrowed(1)
                }
            }
            count => MatchResult::Narrowed(count),
        }
    }

    #[allow(dead_code)]
    pub fn backspace(&mut self) -> usize {
        self.input_buffer.pop();
        self.active_count()
    }

    #[allow(dead_code)]
    pub fn active_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.hint.starts_with(&self.input_buffer))
            .count()
    }

    #[allow(dead_code)]
    pub fn is_hint_active(&self, hint: &str) -> bool {
        hint.starts_with(&self.input_buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::element::Rect;

    #[test]
    fn test_hint_generation() {
        let alphabet = &['a', 's', 'd'];
        let hints = generate_hints(5, alphabet);
        assert_eq!(hints.len(), 5);
        assert_eq!(hints, vec!["aa", "as", "ad", "sa", "ss"]);
    }

    #[test]
    fn test_cyrillic_normalization() {
        assert_eq!(normalize_key('ф'), 'a');
        assert_eq!(normalize_key('Ы'), 's');
        assert_eq!(normalize_key('В'), 'd');
        assert_eq!(normalize_key('а'), 'f');
    }

    #[test]
    fn test_hint_matcher() {
        let elem1 = UIElement::new(0, "Btn1", "Button", Rect::new(0, 0, 10, 10));
        let elem2 = UIElement::new(1, "Btn2", "Button", Rect::new(20, 20, 10, 10));
        let items = vec![
            HintItem { hint: "as".to_string(), element: elem1 },
            HintItem { hint: "ad".to_string(), element: elem2 },
        ];

        let mut matcher = HintMatcher::new(items);
        // Test typing Russian 'ф' -> maps to 'a'
        assert_eq!(matcher.feed_char('ф'), MatchResult::Narrowed(2));
        assert_eq!(matcher.feed_char('z'), MatchResult::Mismatch);
        // Test typing Russian 'ы' -> maps to 's'
        assert_eq!(matcher.feed_char('ы'), MatchResult::Selected(matcher.items[0].element.clone()));
    }
}
