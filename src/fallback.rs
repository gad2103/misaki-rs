/// Trait for OOV (out-of-vocabulary) word fallback mechanisms
pub trait Fallback: Send + Sync {
    /// Convert unknown word to phonemes
    /// Returns (phonemes, rating) tuple
    /// Note: espeak-ng is rule-based and always produces output
    fn phonemize(&self, word: &str) -> (String, u8);
}

/// espeak-ng based fallback
pub struct EspeakFallback {
    british: bool,
}

impl EspeakFallback {
    pub fn new(british: bool) -> Result<Self, String> {
        // Initialize espeak-ng
        // espeak-ng uses IPA output by default which matches misaki
        Ok(Self { british })
    }
}

impl Fallback for EspeakFallback {
    fn phonemize(&self, word: &str) -> (String, u8) {
        use espeakng::{initialise, PhonemeGenOptions, PhonemeMode, TextMode};

        // Initialize espeak (idempotent - safe to call multiple times)
        let speaker_mutex = match initialise(None) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("espeak init error: {:?}", e);
                // Return word as-is if espeak unavailable
                return (word.to_string(), 0);
            }
        };

        let mut speaker = speaker_mutex.lock();

        // Select language variant
        let voice = if self.british {
            "en"
        } else {
            "en-us"
        };

        if let Err(e) = speaker.set_voice_raw(voice) {
            eprintln!("espeak set_voice error for '{}': {:?}", voice, e);
            // Continue anyway, it will use default voice
        }

        // Convert to phonemes using espeak
        // espeak is rule-based and ALWAYS produces output
        let options = PhonemeGenOptions::Standard {
            text_mode: TextMode::Utf8,
            // In espeak-ng, value 2 (bit 1) is IPA.
            // In the espeakng crate, bit 1 is named IncludeZeroWidthJoiners.
            phoneme_mode: PhonemeMode::IncludeZeroWidthJoiners,
        };

        match speaker.text_to_phonemes(word, options) {
            Ok(Some(phonemes)) => {
                // Clean up phonemes to match misaki format
                let cleaned = self.convert_espeak_to_misaki(&phonemes);
                (cleaned, 1)  // rating=1 for fallback
            },
            Ok(None) => {
                (word.to_string(), 0)
            }
            Err(e) => {
                // Should never happen with valid espeak installation
                eprintln!("Unexpected espeak error for '{}': {:?}", word, e);
                // Return word as-is as last resort
                (word.to_string(), 0)
            }
        }
    }
}

impl EspeakFallback {
    /// Convert espeak IPA output to misaki phoneme format
    fn convert_espeak_to_misaki(&self, espeak_ipa: &str) -> String {
        // espeak outputs IPA, misaki uses similar but slightly different symbols
        // Map espeak's IPA to misaki's phoneme set

        let mut result = espeak_ipa.to_string();

        // Common conversions (based on misaki phoneme set):
        // espeak uses standard IPA, misaki uses custom symbols

        // Vowels
        result = result.replace("ɪ", "ɪ");  // same
        result = result.replace("iː", "ˈi");  // long i -> stressed i
        result = result.replace("i:", "ˈi");
        result = result.replace("ʊ", "ʊ");  // same
        result = result.replace("uː", "ˈu");  // long u -> stressed u
        result = result.replace("u:", "ˈu");
        result = result.replace("ɛ", "ɛ");  // same
        result = result.replace("ə", "ə");  // schwa - same
        result = result.replace("ɜː", "ɜ");  // remove length marker
        result = result.replace("ɜ:", "ɜ");
        result = result.replace("ɔː", "ɔ");  // remove length marker
        result = result.replace("ɔ:", "ɔ");
        result = result.replace("æ", "æ");  // same
        result = result.replace("ʌ", "ʌ");  // same
        result = result.replace("ɑː", "ɑ");  // remove length marker
        result = result.replace("ɑ:", "ɑ");

        // Consonants (mostly same in espeak and misaki)
        result = result.replace("ŋ", "ŋ");  // ng
        result = result.replace("ʃ", "ʃ");  // sh
        result = result.replace("ʒ", "ʒ");  // zh
        result = result.replace("θ", "θ");  // th (thin)
        result = result.replace("ð", "ð");  // th (this)
        result = result.replace("ɹ", "ɹ");  // r
        result = result.replace("ʤ", "ʤ");  // j (judge)
        result = result.replace("ʧ", "ʧ");  // ch

        // Stress markers - espeak uses ' for primary, ˌ for secondary
        // Keep as-is (matches misaki)

        // Remove espeak-specific markers we don't need
        result = result.replace("ː", "");  // length marker
        result = result.replace("_", "");  // syllable boundaries

        // Final cleanup: remove duplicate stress markers that might have been
        // introduced by our mappings if espeak already had a stress marker
        result = result.replace("ˈˈ", "ˈ");
        result = result.replace("ˌˌ", "ˌ");
        result = result.replace("ˈˌ", "ˈ");
        result = result.replace("ˌˈ", "ˌ");

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_espeak_fallback() {
        let fallback = EspeakFallback::new(false).expect("espeak should initialize");

        // Test unknown word - espeak ALWAYS returns something
        let (phonemes, rating) = fallback.phonemize("ilili");
        assert!(!phonemes.is_empty());
        assert_eq!(rating, 1);  // fallback rating

        // Verify it doesn't spell out character-by-character
        assert!(!phonemes.contains("ˈɛl"));  // Should not have spelled-out 'L'
    }

    #[test]
    fn test_espeak_nonsense_word() {
        let fallback = EspeakFallback::new(false).unwrap();

        // espeak handles even nonsense words
        let (phonemes, _) = fallback.phonemize("xyzqwop");
        assert!(!phonemes.is_empty(), "espeak should phonemize nonsense words");
    }

    #[test]
    fn test_espeak_phonemes_beat() {
        let fallback = EspeakFallback::new(false).unwrap();
        let (phonemes, _) = fallback.phonemize("beat");
        // Misaki for beat should probably be bˈit or similar
        assert!(phonemes.contains("ˈi"), "Should contain stressed i, got: {}", phonemes);
    }

    #[test]
    fn test_espeak_american_vs_british() {
        let us = EspeakFallback::new(false).unwrap();
        let gb = EspeakFallback::new(true).unwrap();

        // Test word with different pronunciations
        let (us_phonemes, _) = us.phonemize("schedule");
        let (gb_phonemes, _) = gb.phonemize("schedule");

        assert!(!us_phonemes.is_empty());
        assert!(!gb_phonemes.is_empty());
        // Pronunciations should differ (US: sked-, GB: shed-)
        // In IPA: US often starts with 'sk', GB often starts with 'ʃ'
        assert!(us_phonemes.contains("sk"), "US schedule usually has 'sk', got: {}", us_phonemes);
        assert!(gb_phonemes.contains("ʃ"), "GB schedule usually starts with 'ʃ', got: {}", gb_phonemes);
    }
}
