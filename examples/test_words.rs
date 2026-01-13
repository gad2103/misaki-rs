use misaki_rs::G2P;

fn main() {
    let g2p = G2P::new(false); // US English
    let words = vec!["restriction", "restrictions"];
    
    println!("=== Testing Phonemization (US English) ===");
    for word in words {
        let (phonemes, _) = g2p.g2p(word);
        println!("{}: {}", word, phonemes);
    }
}
