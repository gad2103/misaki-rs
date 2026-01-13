use crate::lexicon::Lexicon;
use super::LanguageRules;

pub struct English;

impl LanguageRules for English {
    fn apply_rules(&self, word: &str, tag: &str, lexicon: &Lexicon) -> Option<String> {
        if let Some((ps, _)) = lexicon.stem_s(word, tag, None) {
            Some(ps)
        } else if let Some((ps, _)) = lexicon.stem_ed(word, tag, None) {
            Some(ps)
        } else if let Some((ps, _)) = lexicon.stem_ing(word, tag, None) {
            Some(ps)
        } else {
            None
        }
    }
}
