/// Sentence-ending characters.
const TERMINATORS: &[char] = &['.', '!', '?'];

/// Punctuation allowed between a terminator and the trailing whitespace
/// (closing quotes and the ellipsis).
const EXTRA: &[char] = &['\'', '"', '\u{201C}', '\u{201D}', '\u{2018}', '\u{2019}', '\u{2026}'];

/// Whitespace that can end a sentence fragment.
fn is_break_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{000C}')
}

/// Split text into sentence fragments. A boundary is a terminator, then any
/// closing punctuation, then a run of whitespace; the fragment keeps all three
/// so re-joining the fragments reproduces the original text exactly.
pub fn split_sentences(text: &str) -> Vec<&str> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut fragments = Vec::new();
    let mut start = 0;
    let mut i = 0;

    while i < chars.len() {
        if TERMINATORS.contains(&chars[i].1) {
            let mut j = i + 1;
            while j < chars.len() && EXTRA.contains(&chars[j].1) {
                j += 1;
            }
            if j < chars.len() && is_break_ws(chars[j].1) {
                while j < chars.len() && is_break_ws(chars[j].1) {
                    j += 1;
                }
                let split = chars.get(j).map(|&(b, _)| b).unwrap_or(text.len());
                fragments.push(&text[start..split]);
                start = split;
                i = j;
                continue;
            }
        }
        i += 1;
    }

    if start < text.len() {
        fragments.push(&text[start..]);
    }
    fragments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_fragment_without_terminators() {
        assert_eq!(split_sentences("just some words"), vec!["just some words"]);
    }

    #[test]
    fn empty_text_yields_nothing() {
        assert!(split_sentences("").is_empty());
    }

    #[test]
    fn splits_on_sentence_boundaries() {
        let frags = split_sentences("One. Two! Three?  Four");
        assert_eq!(frags, vec!["One. ", "Two! ", "Three?  ", "Four"]);
        assert_eq!(frags.concat(), "One. Two! Three?  Four");
    }

    #[test]
    fn keeps_closing_quote_with_the_sentence() {
        let frags = split_sentences("\u{201C}Stop!\u{201D} She ran.");
        assert_eq!(frags, vec!["\u{201C}Stop!\u{201D} ", "She ran."]);
    }

    #[test]
    fn does_not_split_without_trailing_whitespace() {
        assert_eq!(split_sentences("file.txt here"), vec!["file.txt here"]);
    }
}
