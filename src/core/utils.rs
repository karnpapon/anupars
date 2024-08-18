use super::config;

/// build documentation to string splitted by newline.
pub fn build_doc_string(src: &config::StaticStrStr) -> String {
  let mut doc_str = String::new();
  for (command, desc) in src.iter() {
    doc_str.push_str(format!("{}: {}\n", command, desc).as_str());
  }

  doc_str
}
