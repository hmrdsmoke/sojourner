// ── Add your header block above ──

//! Proof for the names table: that a word in it is replaced by its sounds
//! before espeak-ng sees the text, possessives included; that the sounds
//! come out of the whole pipeline as intended; and how much of the text's
//! proper names the bundled table covers.

use std::collections::{HashMap, HashSet};

use sojourner::voice::names::Names;
use sojourner::voice::phonemes;

#[test]
fn the_table_rewrites_only_its_words_and_folds_possessives() {
    let table = Names::parse("# a test table\nJob\tdZ'oUb\tJH OW1 B\ttest\nMoses\tm'oUzIz\t-\ttest\nLot\tl'0t\t-\ttest\n").unwrap();
    assert_eq!(table.len(), 3);
    assert_eq!(
        table.respell("Then Job answered Yahweh, and Job’s wife spoke; job done."),
        "Then [[dZ'oUb]] answered Yahweh, and [[dZ'oUbz]] wife spoke; job done."
    );
    // The three possessive endings, and a plain apostrophe left alone.
    assert_eq!(table.respell("Moses’s law, Lot’s wife, Job's ox"), "[[m'oUzIzIz]] law, [[l'0ts]] wife, [[dZ'oUbz]] ox");
    assert_eq!(table.respell("Jesus’ disciples"), "Jesus’ disciples");
    // A word that merely begins with a table word is not touched.
    assert_eq!(table.respell("Jobs Lottery Mosesville"), "Jobs Lottery Mosesville");
    assert_eq!(table.respell(""), "");
}

#[test]
fn a_bad_table_is_refused() {
    assert!(Names::parse("Job\tdZ'oUb\n").is_err(), "three columns");
    assert!(Names::parse("Job\tdʒoʊb\t-\tx\n").is_err(), "non-ASCII mnemonics");
    assert!(Names::parse("Job\ta\t-\tx\nJob\tb\t-\tx\n").is_err(), "duplicate");
}

#[test]
fn job_is_a_man_not_work() {
    phonemes::init(None).unwrap_or_else(|e| panic!("{e}"));
    let said = phonemes::sentences("Then Job answered Yahweh.", "en").unwrap();
    assert_eq!(said, vec!["ðˈɛn dʒˈəʊb ˈansəd jˈɑːweɪ."]);
    let said = phonemes::sentences("Job’s wife said to him, “Curse God, and die.”", "en").unwrap();
    assert!(said[0].starts_with("dʒˈəʊbz wˈaɪf"), "{said:?}");
    // The bare rules, for comparison: the work, not the man.
    let raw = phonemes::sentences_of("Then Job answered Yahweh.", "en").unwrap();
    assert_eq!(raw, vec!["ðˈɛn dʒˈɒb ˈansəd jˈɑːweɪ."]);
}

/// The text's proper names — capitalized words that never appear in
/// lowercase anywhere in the text — against the bundled table. The counts
/// are pinned so that growth (or loss) is noticed.
#[test]
fn how_many_names_the_table_covers() {
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
    // The heuristic misses a name that is also an ordinary word ("Job" —
    // "job" occurs twice in the text), so the table is checked against the
    // capitalized words directly as well.
    let names: Vec<(&String, &usize)> = capitalized
        .iter()
        .filter(|(w, _)| !lowercase.contains(&w.to_lowercase()) && w.len() > 1)
        .collect();
    let occurrences: usize = names.iter().map(|(_, n)| **n).sum();
    let table = Names::bundled();
    let covered: Vec<(&String, &usize)> = capitalized.iter().filter(|(w, _)| table.get(w).is_some()).collect();
    let covered_occurrences: usize = covered.iter().map(|(_, n)| **n).sum();
    eprintln!(
        "{} proper names ({occurrences} occurrences); the table has {} entries, all found in the text, covering {covered_occurrences} occurrences",
        names.len(),
        table.len(),
    );
    assert_eq!(names.len(), 3_836);
    assert_eq!(occurrences, 38_808);
    assert_eq!(covered.len(), table.len(), "every table entry should be a word in the text");
    assert_eq!(table.len(), 682);
    assert_eq!(covered_occurrences, 16_701);
}

/// Every row in the table changes what the voice says: the phonemes the
/// bare rules give for the word differ from the table's. A row that changed
/// nothing would be dead weight, and one whose mnemonics espeak-ng could not
/// read would come out spelled letter by letter — both show up here.
#[test]
fn every_row_changes_what_the_voice_says() {
    phonemes::init(None).unwrap_or_else(|e| panic!("{e}"));
    let table = Names::bundled();
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/names.tsv")).unwrap();
    let mut checked = 0usize;
    for line in text.lines().filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let word = line.split('\t').next().unwrap();
        let bare = phonemes::sentences_of(word, "en").unwrap();
        let said = phonemes::sentences(word, "en").unwrap();
        assert_ne!(bare, said, "{word}: the table's row changes nothing");
        // Letter-by-letter spelling shows as several stressed syllables in a row.
        assert!(!said[0].contains("ˈɛs ˈ"), "{word}: spelled out: {said:?}");
        checked += 1;
    }
    assert_eq!(checked, table.len());
}
