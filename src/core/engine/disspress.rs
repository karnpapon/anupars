use rand::seq::IteratorRandom;
use std::collections::HashMap;

const DISSPRESS_LENGTH: usize = 200;
const INITIALIZER_TEXT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/texts.txt"));
pub(crate) const MANIFESTO_TEXT: &str =
  include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/manifesto.txt"));

pub fn dissociated() -> HashMap<String, HashMap<String, usize>> {
  let mut dissociated: HashMap<String, HashMap<String, usize>> = HashMap::new();

  let separators = [
    ' ', ',', '.', '!', '?', ':', '—', '-', '$', '%', '=', '(', ')', ';', '/', '*', '#', '[', ']',
    '\u{2018}', '"', '"', '\n',
  ];
  let mut words: Vec<&str> = INITIALIZER_TEXT.split(&separators[..]).collect();
  words.retain(|word| !word.is_empty());
  let mut prev_word: Option<String> = None;

  for word in words {
    let word = word.to_string();
    if let Some(prev) = &prev_word {
      let entry = dissociated.entry(prev.clone()).or_default();
      *entry.entry(word.clone()).or_insert(0) += 1;
    }

    prev_word = Some(word);
  }

  dissociated
}

fn dissociated_generate(
  dissociated: &HashMap<String, HashMap<String, usize>>,
  length: Option<usize>,
) -> String {
  let mut rng = rand::thread_rng();
  let length = length.unwrap_or(DISSPRESS_LENGTH);

  let mut words = Vec::new();
  let mut current_word: Option<String> = None;

  for _ in 0..length {
    current_word = match current_word {
      Some(word) => {
        if let Some(inner_map) = dissociated.get(&word) {
          inner_map
            .iter()
            .flat_map(|(k, v)| std::iter::repeat_n(k, *v))
            .choose(&mut rng)
            .cloned()
        } else {
          None
        }
      }
      None => dissociated.keys().choose(&mut rng).cloned(),
    };

    if let Some(word) = &current_word {
      words.push(word.clone());
    } else {
      break;
    }
  }

  words.join(" ")
}

pub fn run_with_length(length: usize) -> String {
  let diss = dissociated();
  dissociated_generate(&diss, Some(length))
}
