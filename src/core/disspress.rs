use rand::seq::IteratorRandom;
use rand::Rng;
use std::collections::HashMap;
use std::fs;

pub fn dissociated() -> HashMap<String, HashMap<String, usize>> {
  let mut dissociated: HashMap<String, HashMap<String, usize>> = HashMap::new();

  let file_path = "path/to/war-and-peace.txt";
  // let content = fs::read_to_string(file_path).expect("Unable to read file");
  let content = "The algorithm starts by printing a number of consecutive words (or letters) from the source text. Then it searches the source text for an occurrence of the few last words or letters printed out so far. If multiple occurrences are found, it picks a random one, and proceeds with printing the text following the chosen occurrence. After a predetermined length of text is printed out, the search procedure is repeated for the newly printed ending.";

  let separators = [
    ' ', ',', '.', '!', '?', ':', '—', '-', '$', '%', '=', '(', ')', ';', '/', '*', '#', '[', ']',
    '’', '”', '“', '\n',
  ];
  let mut words: Vec<&str> = content.split(&separators[..]).collect();
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

fn join_spaced(words: Vec<String>) -> String {
  words.join(" ")
}

fn dissociated_generate(
  dissociated: &HashMap<String, HashMap<String, usize>>,
  length: Option<usize>,
) -> String {
  let mut rng = rand::thread_rng();
  let length = length.unwrap_or(100);

  let mut words = Vec::new();
  let mut current_word: Option<String> = None;

  for _ in 0..length {
    current_word = match current_word {
      Some(word) => {
        if let Some(inner_map) = dissociated.get(&word) {
          inner_map
            .iter()
            .flat_map(|(k, v)| std::iter::repeat(k).take(*v))
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

  join_spaced(words)
}

fn run_dissociate_press() {
  let mut _dissociated: HashMap<String, HashMap<String, usize>> = HashMap::new();
  let diss = dissociated();
  let random_length = rand::thread_rng().gen_range(1..100);
  let result = dissociated_generate(&diss, Some(random_length));
  // println!("res::::::::::{:?}", result);
}
