use fancy_regex::Regex;
use num2words::Num2Words;
use crate::token::{MToken, Underscore};
use crate::lexicon::Lexicon;
use std::collections::HashMap;
use crate::tagger::PerceptronTagger;

pub struct G2P {
    pub lexicon: Lexicon,
    pub unk: String,
    subtoken_regex: Regex,
    tagger: PerceptronTagger,
}

impl G2P {
    pub fn new(british: bool) -> Self {
        // Regex for subtokenization with better UTF-8 support using Unicode properties
        let subtoken_regex = Regex::new(r"(?x)
            ^['‘’]+ |
            \p{Lu}(?=\p{Lu}\p{Ll}) |
            (?:^-)?(?:\d?[,.]?\d)+ |
            [-_]+ |
            ['‘’]{2,} |
            \p{L}*?(?:[-'‘’]\p{L})*?\p{Ll}(?=\p{Lu}) |
            \p{L}+(?:[-'‘’]\p{L})* |
            [^- _ \p{L} '‘’ \d] |
            ['‘’]+$
        ").unwrap();

        let weights_json = include_str!("resources/tagger/weights.json");
        let classes_txt = include_str!("resources/tagger/classes.txt");
        let tags_json = include_str!("resources/tagger/tags.json");

        Self {
            lexicon: Lexicon::new(british),
            unk: "❓".to_string(),
            subtoken_regex,
            tagger: PerceptronTagger::new(weights_json, classes_txt, tags_json),
        }
    }

    pub fn preprocess(&self, text: &str) -> (String, Vec<String>, HashMap<usize, String>) {
        // Simplified preprocess: just return the text and tokens for now
        // Python handles links like [text](phonemes), we'll skip that for simplicity unless needed
        let tokens: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
        (text.to_string(), tokens, HashMap::new())
    }

    pub fn tokenize(&self, text: &str) -> Vec<MToken> {
        let mut tokens = Vec::new();
        for mat in self.subtoken_regex.find_iter(text) {
            if let Ok(m) = mat {
                let sub = m.as_str();
                let tk = MToken::new(sub.to_string(), "NN".to_string(), " ".to_string());
                tokens.push(tk);
            }
        }
        tokens
    }


    pub fn g2p(&self, text: &str) -> (String, Vec<MToken>) {
        let (processed_text, _, _) = self.preprocess(text);
        let mut tokens = self.tokenize(&processed_text);

        // Collect words for tagging
        let words_owned: Vec<String> = tokens.iter().map(|tk| tk.text.clone()).collect();
        let words: Vec<&str> = words_owned.iter().map(|s| s.as_str()).collect();
        let tags = self.tagger.tag(&words);

        eprintln!("DEBUG: g2p '{}' -> {} tokens, {} tags", text, tokens.len(), tags.len());

        for (tk, tag) in tokens.iter_mut().zip(tags.into_iter()) {
            tk.tag = tag.tag;
            if tk.phonemes.is_none() {
                let word = tk.text.clone();
                let tag = tk.tag.clone();
                eprintln!("DEBUG: processing token '{}' with tag '{}'", word, tag);
                
                // Try dictionary lookup
                if let Some((ps, _)) = self.lexicon.lookup(&word, &tag, None) {
                    tk.phonemes = Some(ps);
                } else if let Some((ps, _)) = self.lexicon.stem_s(&word, &tag, None) {
                    tk.phonemes = Some(ps);
                } else if let Some((ps, _)) = self.lexicon.stem_ed(&word, &tag, None) {
                    tk.phonemes = Some(ps);
                } else if let Some((ps, _)) = self.lexicon.stem_ing(&word, &tag, None) {
                    tk.phonemes = Some(ps);
                } else if word.contains('-') && word.len() > 1 {
                    // Handle hyphenated words like "twenty-one"
                    let parts: Vec<&str> = word.split('-').filter(|s| !s.is_empty()).collect();
                    let mut sub_ps = Vec::new();
                    for part in parts {
                        let (p, _) = self.g2p(part);
                        sub_ps.push(p);
                    }
                    tk.phonemes = Some(sub_ps.join(" "));
                } else if self.is_number(&word) {
                    let spoken = self.convert_number(&word);
                    let (p, _) = self.g2p(&spoken);
                    tk.phonemes = Some(p);
                } else if word.chars().count() > 1 {
                    // Try character-by-character if the whole word is unknown
                    let mut char_ps = Vec::new();
                    for c in word.chars() {
                        let (p, _) = self.g2p(&c.to_string());
                        char_ps.push(p);
                    }
                    tk.phonemes = Some(char_ps.join(" "));
                } else {
                    // Try to normalize the character or return unknown
                    let normalized: String = word.chars()
                        .map(|c| match c {
                            'é' | 'è' | 'ê' | 'ë' => 'e',
                            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
                            'í' | 'ì' | 'î' | 'ï' => 'i',
                            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
                            'ú' | 'ù' | 'û' | 'ü' => 'u',
                            'ñ' => 'n',
                            'ç' => 'c',
                            '—' | '–' => ' ', // map dashes to spaces
                            _ => c,
                        })
                        .collect();
                    
                    if normalized != word {
                        let (p, _) = self.g2p(&normalized);
                        tk.phonemes = Some(p);
                    } else {
                        // Handle standard punctuation and symbols gracefully
                        if word.chars().count() == 1 {
                            let c = word.chars().next().unwrap();
                            if c.is_ascii_punctuation() || "—–…".contains(c) {
                                tk.phonemes = Some(" ".to_string());
                            } else {
                                tk.phonemes = Some(self.unk.clone());
                            }
                        } else {
                            tk.phonemes = Some(self.unk.clone());
                        }
                    }
                }
            }
        }

        let result = tokens.iter()
            .map(|tk| tk.phonemes.as_ref().unwrap_or(&self.unk).clone() + &tk.whitespace)
            .collect::<String>();

        (result, tokens)
    }

    fn is_number(&self, word: &str) -> bool {
        word.chars().any(|c| c.is_digit(10)) && word.chars().all(|c| c.is_digit(10) || c == ',' || c == '.')
    }

    fn convert_number(&self, word: &str) -> String {
        let clean = word.replace(",", "");
        if let Ok(val) = clean.parse::<i64>() {
            if let Ok(spoken) = Num2Words::new(val).to_words() {
                 return spoken;
            }
        }
        word.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_g2p_basic() {
        let g2p = G2P::new();
        let (phonemes, _) = g2p.g2p("Hello, world!");
        println!("Phonemes: {}", phonemes);
        assert!(!phonemes.contains("❓"));
    }
}
