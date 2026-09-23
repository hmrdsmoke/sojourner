// ── Add your header block above ──

//! Lists the text's proper names for the names tool (tools/names/compose.py):
//! every capitalized word that never appears in lowercase anywhere in the
//! text, with how often it occurs, most frequent first. "Job" is added by
//! hand, since "job" the word occurs too. Writes tab-separated lines to
//! standard output:
//!
//!     cargo run --bin names-list > assets/names-source/web-names.tsv

use std::collections::{HashMap, HashSet};

fn main() {
    let lib = sojourner::Library::load_bundled().unwrap_or_else(|e| panic!("{e}"));
    let mut capitalized: HashMap<String, usize> = HashMap::new();
    let mut lowercase: HashSet<String> = HashSet::new();
    for book in &lib.bible.books {
        for (_, _, plain) in sojourner::text::plain_verses(book) {
            for raw in plain.split(|c: char| !c.is_alphabetic() && c != '’' && c != '\'') {
                let word = raw.trim_matches(['’', '\'']);
                let word = word.strip_suffix("’s").unwrap_or(word);
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) if first.is_uppercase() && chars.all(char::is_lowercase) => {
                        *capitalized.entry(word.to_string()).or_default() += 1;
                    }
                    Some(_) if word.chars().all(char::is_lowercase) => {
                        lowercase.insert(word.to_string());
                    }
                    _ => {}
                }
            }
        }
    }
    let mut names: Vec<(String, usize)> = capitalized
        .into_iter()
        .filter(|(w, _)| w.chars().count() > 1 && (!lowercase.contains(&w.to_lowercase()) || w == "Job"))
        .collect();
    names.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (word, count) in names {
        println!("{word}\t{count}");
    }
}
