use espeak_rs::text_to_phonemes;
use std::sync::Mutex;

static ESPEAK_MUTEX: Mutex<()> = Mutex::new(());

/// Trait for OOV (out-of-vocabulary) word fallback mechanisms
pub trait Fallback: Send + Sync {
    /// Convert unknown word to phonemes
    /// Returns (phonemes, rating) tuple
    /// Note: espeak-ng is rule-based and always produces output
    fn phonemize(&self, word: &str) -> Result<(String, u8), String>;
}

/// espeak-ng based fallback
pub struct EspeakFallback {
    british: bool,
}

impl EspeakFallback {
    pub fn new(british: bool) -> Result<Self, String> {
        Ok(Self { british })
    }

    /// Convert espeak IPA output to misaki phoneme format
    fn convert_espeak_to_misaki(&self, espeak_ipa: &str) -> String {
        let mut result = espeak_ipa.to_string();

        // Conversions to match misaki phoneme set
        result = result.replace("iː", "ˈi").replace("i:", "ˈi");
        result = result.replace("uː", "ˈu").replace("u:", "ˈu");
        result = result.replace("ɜː", "ɜ").replace("ɜ:", "ɜ");
        result = result.replace("ɔː", "ɔ").replace("ɔ:", "ɔ");
        result = result.replace("ɑː", "ɑ").replace("ɑ:", "ɑ");
        result = result.replace("ː", ""); // remove remaining length markers
        result = result.replace("_", ""); // stop syllables

        result.replace("ˈˈ", "ˈ").replace("ˌˌ", "ˌ")
    }
}

impl Fallback for EspeakFallback {
    fn phonemize(&self, word: &str) -> Result<(String, u8), String> {
        let _lock = ESPEAK_MUTEX.lock().map_err(|e| format!("mutex poisoned: {:?}", e))?;
        let voice = if self.british { "en" } else { "en-us" };

        // Use the portable espeak-rs call (used in kokoros)
        match text_to_phonemes(word, voice, None, true, false) {
            Ok(phonemes) => {
                if phonemes.is_empty() {
                    return Ok((word.to_string(), 0));
                }
                let cleaned = self.convert_espeak_to_misaki(&phonemes.join(""));
                Ok((cleaned, 1))
            }
            Err(e) => Err(format!("espeak error for '{}': {:?}", word, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_espeak_fallback() {
        let fallback = EspeakFallback::new(false).expect("espeak should initialize");

        // Test unknown word - espeak ALWAYS returns something
        let (phonemes, rating) = fallback.phonemize("ilili").unwrap();
        assert!(!phonemes.is_empty());
        assert_eq!(rating, 1);  // fallback rating

        // Verify it doesn't spell out character-by-character
        assert!(!phonemes.contains("ˈɛl"));  // Should not have spelled-out 'L'
    }

    #[test]
    fn test_espeak_nonsense_word() {
        let fallback = EspeakFallback::new(false).unwrap();

        // espeak handles even nonsense words
        let (phonemes, _) = fallback.phonemize("xyzqwop").unwrap();
        assert!(!phonemes.is_empty(), "espeak should phonemize nonsense words");
    }

    #[test]
    fn test_espeak_phonemes_beat() {
        let fallback = EspeakFallback::new(false).unwrap();
        let (phonemes, _) = fallback.phonemize("beat").unwrap();
        // Misaki for beat should probably be bˈit or similar
        assert!(phonemes.contains("ˈi"), "Should contain stressed i, got: {}", phonemes);
    }

    #[test]
    fn test_espeak_american_vs_british() {
        let us = EspeakFallback::new(false).unwrap();
        let gb = EspeakFallback::new(true).unwrap();

        // Test word with different pronunciations
        let (us_phonemes, _) = us.phonemize("schedule").unwrap();
        let (gb_phonemes, _) = gb.phonemize("schedule").unwrap();

        assert!(!us_phonemes.is_empty());
        assert!(!gb_phonemes.is_empty());
        // Pronunciations should differ (US: sked-, GB: shed-)
        // In IPA: US often starts with 'sk', GB often starts with 'ʃ'
        assert!(us_phonemes.contains("sk"), "US schedule usually has 'sk', got: {}", us_phonemes);
        assert!(gb_phonemes.contains("ʃ"), "GB schedule usually starts with 'ʃ', got: {}", gb_phonemes);
    }
}
