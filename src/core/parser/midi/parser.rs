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

type MidiParser = (Vec<(String, u8)>, u8, u8, u8);

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

pub fn parse_midi_msg(input: &str) -> IResult<&str, MidiParser> {
  let (input, (note_octave, _, len, _, vel, _, channel)) = tuple((
    parse_note_octave_array,
    space1,
    parse_midi_length,
    space1,
    parse_midi_velocity,
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
