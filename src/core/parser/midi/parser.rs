use nom::bytes::complete::tag;
use nom::character::complete::digit1;
use nom::character::complete::one_of;
use nom::character::streaming::space1;
use nom::combinator::map_res;
use nom::combinator::opt;
use nom::multi::separated_list1;
use nom::sequence::tuple;
use nom::IResult;

type MidiParser = (Vec<(String, u8)>, Vec<u8>, Vec<u8>, u8);

fn parse_note_octave(input: &str) -> IResult<&str, (String, u8)> {
  let (input, note) = one_of("CDEFGAB")(input)?; // Parse note (C, D, E, F, G, A, B)
  let (input, sharp) = opt(tag("#"))(input)?; // Parse optional sharp symbol (#)
  let (input, octave) = map_res(digit1, |s: &str| s.parse::<u8>())(input)?; // Parse octave

  let note_with_sharp = format!("{}{}", note, sharp.unwrap_or(""));

  Ok((input, (note_with_sharp, octave)))
}

fn parse_note_octave_array(input: &str) -> IResult<&str, Vec<(String, u8)>> {
  separated_list1(tag(","), parse_note_octave)(input)
}

fn parse_bounded_u8(input: &str, max: u8) -> IResult<&str, u8> {
  let (input, value) = map_res(digit1, |s: &str| s.parse::<u8>())(input)?;

  if value <= max {
    Ok((input, value))
  } else {
    Err(nom::Err::Error(nom::error::Error {
      input,
      code: nom::error::ErrorKind::Eof,
    }))
  }
}

fn parse_midi_channel(input: &str) -> IResult<&str, u8> {
  parse_bounded_u8(input, 16)
}

fn parse_midi_length(input: &str) -> IResult<&str, u8> {
  parse_bounded_u8(input, 127)
}

fn parse_midi_velocity(input: &str) -> IResult<&str, u8> {
  parse_bounded_u8(input, 127)
}

fn parse_midi_length_array(input: &str) -> IResult<&str, Vec<u8>> {
  separated_list1(tag(","), parse_midi_length)(input)
}

fn parse_midi_velocity_array(input: &str) -> IResult<&str, Vec<u8>> {
  separated_list1(tag(","), parse_midi_velocity)(input)
}

pub fn parse_midi_msg(input: &str) -> IResult<&str, MidiParser> {
  let (input, (note_octave, _, len, _, vel, _, channel)) = tuple((
    parse_note_octave_array,
    space1,
    parse_midi_length_array,
    space1,
    parse_midi_velocity_array,
    space1,
    parse_midi_channel,
  ))(input)?;

  if !input.is_empty() {
    return Err(nom::Err::Error(nom::error::Error {
      input,
      code: nom::error::ErrorKind::Eof,
    }));
  }

  Ok((input, (note_octave, len, vel, channel)))
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Parse `input`, assert it succeeds with no remaining input, and return the parsed fields.
  fn assert_parse_ok(input: &str) -> (Vec<(String, u8)>, Vec<u8>, Vec<u8>, u8) {
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, parsed) = result.unwrap();
    assert_eq!(remaining, "");
    parsed
  }

  /// Parse `input` and assert it fails.
  fn assert_parse_err(input: &str) {
    assert!(parse_midi_msg(input).is_err());
  }

  #[test]
  fn test_parse_midi_msg_single_note() {
    let (notes, len, vel, channel) = assert_parse_ok("C4 64 100 1");
    assert_eq!(notes, vec![("C".to_string(), 4)]);
    assert_eq!(len, vec![64]);
    assert_eq!(vel, vec![100]);
    assert_eq!(channel, 1);
  }

  #[test]
  fn test_parse_midi_msg_multiple_notes() {
    let (notes, len, vel, channel) = assert_parse_ok("C4,D5,E6 127 127 16");
    assert_eq!(
      notes,
      vec![
        ("C".to_string(), 4),
        ("D".to_string(), 5),
        ("E".to_string(), 6)
      ]
    );
    assert_eq!(len, vec![127]);
    assert_eq!(vel, vec![127]);
    assert_eq!(channel, 16);
  }

  #[test]
  fn test_parse_midi_msg_with_sharp_notes() {
    let (notes, len, vel, channel) = assert_parse_ok("C#4,D#5 64 80 5");
    assert_eq!(notes, vec![("C#".to_string(), 4), ("D#".to_string(), 5)]);
    assert_eq!(len, vec![64]);
    assert_eq!(vel, vec![80]);
    assert_eq!(channel, 5);
  }

  #[test]
  fn test_parse_midi_msg_min_values() {
    let (notes, len, vel, channel) = assert_parse_ok("A0 0 0 1");
    assert_eq!(notes, vec![("A".to_string(), 0)]);
    assert_eq!(len, vec![0]);
    assert_eq!(vel, vec![0]);
    assert_eq!(channel, 1);
  }

  #[test]
  fn test_parse_midi_msg_max_values() {
    let (notes, len, vel, channel) = assert_parse_ok("G9 127 127 16");
    assert_eq!(notes, vec![("G".to_string(), 9)]);
    assert_eq!(len, vec![127]);
    assert_eq!(vel, vec![127]);
    assert_eq!(channel, 16);
  }

  #[test]
  fn test_parse_midi_msg_invalid_channel_too_high() {
    assert_parse_err("C4 64 100 17");
  }

  #[test]
  fn test_parse_midi_msg_invalid_velocity_too_high() {
    assert_parse_err("C4 64 128 1");
  }

  #[test]
  fn test_parse_midi_msg_invalid_length_too_high() {
    assert_parse_err("C4 128 100 1");
  }

  #[test]
  fn test_parse_midi_msg_extra_text() {
    assert_parse_err("C4 64 100 1 extra");
  }

  #[test]
  fn test_parse_midi_msg_missing_fields() {
    assert_parse_err("C4 64 100");
  }

  #[test]
  fn test_parse_midi_msg_invalid_note() {
    assert_parse_err("X4 64 100 1");
  }

  #[test]
  fn test_parse_midi_msg_missing_octave() {
    assert_parse_err("C 64 100 1");
  }

  #[test]
  fn test_parse_midi_msg_empty_input() {
    assert_parse_err("");
  }

  #[test]
  fn test_parse_midi_msg_multiple_notes_low_velocity() {
    let (notes, len, vel, channel) = assert_parse_ok("A3,B4,C5 32 10 8");
    assert_eq!(
      notes,
      vec![
        ("A".to_string(), 3),
        ("B".to_string(), 4),
        ("C".to_string(), 5)
      ]
    );
    assert_eq!(len, vec![32]);
    assert_eq!(vel, vec![10]);
    assert_eq!(channel, 8);
  }

  #[test]
  fn test_parse_midi_msg_multiple_notes_high_velocity() {
    let (notes, len, vel, channel) = assert_parse_ok("F2,G3,A4,B5 96 120 12");
    assert_eq!(
      notes,
      vec![
        ("F".to_string(), 2),
        ("G".to_string(), 3),
        ("A".to_string(), 4),
        ("B".to_string(), 5)
      ]
    );
    assert_eq!(len, vec![96]);
    assert_eq!(vel, vec![120]);
    assert_eq!(channel, 12);
  }

  #[test]
  fn test_parse_midi_msg_multiple_notes_short_length() {
    let (notes, len, vel, channel) = assert_parse_ok("C4,E4,G4 1 64 3");
    assert_eq!(
      notes,
      vec![
        ("C".to_string(), 4),
        ("E".to_string(), 4),
        ("G".to_string(), 4)
      ]
    );
    assert_eq!(len, vec![1]);
    assert_eq!(vel, vec![64]);
    assert_eq!(channel, 3);
  }

  #[test]
  fn test_parse_midi_msg_multiple_notes_mixed_sharps() {
    let (notes, len, vel, channel) = assert_parse_ok("C#3,E3,G#3,B3 48 75 6");
    assert_eq!(
      notes,
      vec![
        ("C#".to_string(), 3),
        ("E".to_string(), 3),
        ("G#".to_string(), 3),
        ("B".to_string(), 3)
      ]
    );
    assert_eq!(len, vec![48]);
    assert_eq!(vel, vec![75]);
    assert_eq!(channel, 6);
  }

  #[test]
  fn test_parse_midi_msg_multiple_notes_zero_velocity() {
    let (notes, len, vel, channel) = assert_parse_ok("D4,F4,A4 64 0 2");
    assert_eq!(
      notes,
      vec![
        ("D".to_string(), 4),
        ("F".to_string(), 4),
        ("A".to_string(), 4)
      ]
    );
    assert_eq!(len, vec![64]);
    assert_eq!(vel, vec![0]);
    assert_eq!(channel, 2);
  }

  #[test]
  fn test_parse_midi_msg_five_notes_various_octaves() {
    let (notes, len, vel, channel) = assert_parse_ok("C1,D2,E3,F4,G5 80 90 10");
    assert_eq!(
      notes,
      vec![
        ("C".to_string(), 1),
        ("D".to_string(), 2),
        ("E".to_string(), 3),
        ("F".to_string(), 4),
        ("G".to_string(), 5)
      ]
    );
    assert_eq!(len, vec![80]);
    assert_eq!(vel, vec![90]);
    assert_eq!(channel, 10);
  }

  #[test]
  fn test_parse_midi_msg_multiple_lengths() {
    let (notes, len, vel, channel) = assert_parse_ok("C4 64,32,16 100 1");
    assert_eq!(notes, vec![("C".to_string(), 4)]);
    assert_eq!(len, vec![64, 32, 16]);
    assert_eq!(vel, vec![100]);
    assert_eq!(channel, 1);
  }

  #[test]
  fn test_parse_midi_msg_multiple_velocities() {
    let (notes, len, vel, channel) = assert_parse_ok("C4 64 100,80,60 1");
    assert_eq!(notes, vec![("C".to_string(), 4)]);
    assert_eq!(len, vec![64]);
    assert_eq!(vel, vec![100, 80, 60]);
    assert_eq!(channel, 1);
  }

  #[test]
  fn test_parse_midi_msg_multiple_notes_lengths_velocities() {
    let (notes, len, vel, channel) = assert_parse_ok("C4,D4,E4 64,32,16 100,80,60 5");
    assert_eq!(
      notes,
      vec![
        ("C".to_string(), 4),
        ("D".to_string(), 4),
        ("E".to_string(), 4)
      ]
    );
    assert_eq!(len, vec![64, 32, 16]);
    assert_eq!(vel, vec![100, 80, 60]);
    assert_eq!(channel, 5);
  }

  #[test]
  fn test_parse_midi_msg_all_arrays_different_sizes() {
    let (notes, len, vel, channel) = assert_parse_ok("C4,D4 127,64,32,16,8 100,80 10");
    assert_eq!(notes, vec![("C".to_string(), 4), ("D".to_string(), 4)]);
    assert_eq!(len, vec![127, 64, 32, 16, 8]);
    assert_eq!(vel, vec![100, 80]);
    assert_eq!(channel, 10);
  }
}
