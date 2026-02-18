use nom::{
  bytes::complete::tag,
  character::{
    complete::{digit1, one_of},
    streaming::space1,
  },
  combinator::{map_res, opt},
  multi::separated_list1,
  sequence::tuple,
  IResult,
};

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

fn parse_midi_channel(input: &str) -> IResult<&str, u8> {
  let (input, channel) = map_res(digit1, |s: &str| s.parse::<u8>())(input)?;

  if channel <= 16 {
    Ok((input, channel))
  } else {
    Err(nom::Err::Error(nom::error::Error {
      input,
      code: nom::error::ErrorKind::Eof,
    }))
  }
}

fn parse_midi_length(input: &str) -> IResult<&str, u8> {
  let (input, channel) = map_res(digit1, |s: &str| s.parse::<u8>())(input)?;

  if channel <= 127 {
    Ok((input, channel))
  } else {
    Err(nom::Err::Error(nom::error::Error {
      input,
      code: nom::error::ErrorKind::Eof,
    }))
  }
}

fn parse_midi_velocity(input: &str) -> IResult<&str, u8> {
  let (input, channel) = map_res(digit1, |s: &str| s.parse::<u8>())(input)?;

  if channel <= 127 {
    Ok((input, channel))
  } else {
    Err(nom::Err::Error(nom::error::Error {
      input,
      code: nom::error::ErrorKind::Eof,
    }))
  }
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

  #[test]
  fn test_parse_midi_msg_single_note() {
    let input = "C4 64 100 1";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(notes, vec![("C".to_string(), 4)]);
    assert_eq!(len, vec![64]);
    assert_eq!(vel, vec![100]);
    assert_eq!(channel, 1);
  }

  #[test]
  fn test_parse_midi_msg_multiple_notes() {
    let input = "C4,D5,E6 127 127 16";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
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
    let input = "C#4,D#5 64 80 5";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(notes, vec![("C#".to_string(), 4), ("D#".to_string(), 5)]);
    assert_eq!(len, vec![64]);
    assert_eq!(vel, vec![80]);
    assert_eq!(channel, 5);
  }

  #[test]
  fn test_parse_midi_msg_min_values() {
    let input = "A0 0 0 1";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(notes, vec![("A".to_string(), 0)]);
    assert_eq!(len, vec![0]);
    assert_eq!(vel, vec![0]);
    assert_eq!(channel, 1);
  }

  #[test]
  fn test_parse_midi_msg_max_values() {
    let input = "G9 127 127 16";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(notes, vec![("G".to_string(), 9)]);
    assert_eq!(len, vec![127]);
    assert_eq!(vel, vec![127]);
    assert_eq!(channel, 16);
  }

  #[test]
  fn test_parse_midi_msg_invalid_channel_too_high() {
    let input = "C4 64 100 17";
    let result = parse_midi_msg(input);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_midi_msg_invalid_velocity_too_high() {
    let input = "C4 64 128 1";
    let result = parse_midi_msg(input);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_midi_msg_invalid_length_too_high() {
    let input = "C4 128 100 1";
    let result = parse_midi_msg(input);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_midi_msg_extra_text() {
    let input = "C4 64 100 1 extra";
    let result = parse_midi_msg(input);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_midi_msg_missing_fields() {
    let input = "C4 64 100";
    let result = parse_midi_msg(input);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_midi_msg_invalid_note() {
    let input = "X4 64 100 1";
    let result = parse_midi_msg(input);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_midi_msg_missing_octave() {
    let input = "C 64 100 1";
    let result = parse_midi_msg(input);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_midi_msg_empty_input() {
    let input = "";
    let result = parse_midi_msg(input);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_midi_msg_multiple_notes_low_velocity() {
    let input = "A3,B4,C5 32 10 8";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
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
    let input = "F2,G3,A4,B5 96 120 12";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
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
    let input = "C4,E4,G4 1 64 3";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
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
    let input = "C#3,E3,G#3,B3 48 75 6";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
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
    let input = "D4,F4,A4 64 0 2";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
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
    let input = "C1,D2,E3,F4,G5 80 90 10";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
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
    let input = "C4 64,32,16 100 1";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(notes, vec![("C".to_string(), 4)]);
    assert_eq!(len, vec![64, 32, 16]);
    assert_eq!(vel, vec![100]);
    assert_eq!(channel, 1);
  }

  #[test]
  fn test_parse_midi_msg_multiple_velocities() {
    let input = "C4 64 100,80,60 1";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(notes, vec![("C".to_string(), 4)]);
    assert_eq!(len, vec![64]);
    assert_eq!(vel, vec![100, 80, 60]);
    assert_eq!(channel, 1);
  }

  #[test]
  fn test_parse_midi_msg_multiple_notes_lengths_velocities() {
    let input = "C4,D4,E4 64,32,16 100,80,60 5";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
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
    let input = "C4,D4 127,64,32,16,8 100,80 10";
    let result = parse_midi_msg(input);
    assert!(result.is_ok());
    let (remaining, (notes, len, vel, channel)) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(notes, vec![("C".to_string(), 4), ("D".to_string(), 4)]);
    assert_eq!(len, vec![127, 64, 32, 16, 8]);
    assert_eq!(vel, vec![100, 80]);
    assert_eq!(channel, 10);
  }
}
