//! Server-side challenge phrase generator (master-list #89).
//!
//! Ports `pulse-sdk/src/challenge/phrase.ts:generatePhrase` verbatim so the
//! server can issue the phrase alongside the challenge nonce. Without server
//! issuance, phrase content binding is defeated trivially: an attacker would
//! submit their own phrase matching whatever audio they captured.
//!
//! The 70-syllable alphabet is kept byte-for-byte identical to the SDK array
//! so that `iam-validation`'s precomputed syllable→Metaphone lookup table
//! covers every syllable this generator emits. Any drift between the two
//! lists would cause `iam_validation::phrase_binding::compute_expected_phonemes`
//! to log warnings and emit empty token streams, degrading the check to skip.
//!
//! Each phrase is 5 words (default) of 2-3 syllables each, rolled fresh via
//! `rand::thread_rng()` on every call. Output shape:
//!
//! ```text
//! "bada lita mupe ruso poto"
//! ```

use rand::Rng;

/// 70 phonetically-balanced syllables mirroring
/// `pulse-sdk/src/challenge/phrase.ts:3-11`. Order does not matter downstream;
/// only set equality with the SDK array matters.
const SYLLABLES: &[&str] = &[
    "ba", "da", "fa", "ga", "ha", "ja", "ka", "la", "ma", "na", "pa", "ra",
    "sa", "ta", "wa", "za", "be", "de", "fe", "ge", "ke", "le", "me", "ne",
    "pe", "re", "se", "te", "we", "ze", "bi", "di", "fi", "gi", "ki", "li",
    "mi", "ni", "pi", "ri", "si", "ti", "wi", "zi", "bo", "do", "fo", "go",
    "ko", "lo", "mo", "no", "po", "ro", "so", "to", "wo", "zo", "bu", "du",
    "fu", "gu", "ku", "lu", "mu", "nu", "pu", "ru", "su", "tu",
];

/// Generate a random phonetically-balanced phrase of `word_count` words.
/// Each word is 2-3 syllables concatenated. Uses `rand::thread_rng()` for
/// unpredictable output (equivalent of the SDK's `crypto.getRandomValues`).
pub fn generate_phrase(word_count: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut words: Vec<String> = Vec::with_capacity(word_count);

    for _ in 0..word_count {
        let syllable_count = 2 + rng.gen_range(0..2); // 2 or 3 syllables per word
        let mut word = String::with_capacity(syllable_count * 2);
        for _ in 0..syllable_count {
            word.push_str(SYLLABLES[rng.gen_range(0..SYLLABLES.len())]);
        }
        words.push(word);
    }

    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_five_words_by_default() {
        let phrase = generate_phrase(5);
        let words: Vec<&str> = phrase.split_whitespace().collect();
        assert_eq!(words.len(), 5);
    }

    #[test]
    fn each_word_has_4_or_6_chars() {
        let phrase = generate_phrase(5);
        for word in phrase.split_whitespace() {
            let len = word.len();
            assert!(len == 4 || len == 6, "word length must be 4 or 6, got {len} for '{word}'");
        }
    }

    #[test]
    fn successive_calls_differ() {
        // Not a hard guarantee — theoretically two calls could produce the
        // same phrase — but with 70^5 to 70^15 possibilities per word and
        // 5 words, collision is astronomically unlikely.
        let a = generate_phrase(5);
        let b = generate_phrase(5);
        assert_ne!(a, b);
    }

    #[test]
    fn every_syllable_is_in_the_alphabet() {
        let phrase = generate_phrase(5);
        for word in phrase.split_whitespace() {
            let bytes = word.as_bytes();
            let mut i = 0;
            while i + 2 <= bytes.len() {
                let syl = &word[i..i + 2];
                assert!(
                    SYLLABLES.contains(&syl),
                    "syllable '{syl}' not in alphabet"
                );
                i += 2;
            }
        }
    }

    #[test]
    fn syllable_count_is_seventy() {
        assert_eq!(SYLLABLES.len(), 70);
    }
}
