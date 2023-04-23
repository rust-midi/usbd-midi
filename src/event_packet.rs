use crate::{
    cable_number::CableNumber,
    code_index_number::CodeIndexNumber,
    midi_convert::{MidiParseError, MidiRenderSlice, MidiTryParseSlice},
    midi_types::MidiMessage,
};
use core::convert::TryFrom;

/// A packet that communicates with the host
/// Currently supported is sending the specified normal midi
/// message over the supplied cable number
#[derive(Debug, PartialEq)]
pub struct UsbMidiEventPacket {
    pub cable_number: CableNumber,
    pub message: MidiMessage,
}

impl From<UsbMidiEventPacket> for [u8; 4] {
    fn from(value: UsbMidiEventPacket) -> [u8; 4] {
        let message = value.message;
        let cable_number: u8 = value.cable_number.into();
        let index_number: u8 = CodeIndexNumber::find_from_message(&message).into();
        let header = cable_number << 4 | index_number;
        let mut data: [u8; 4] = [header, 0, 0, 0];
        message.render_slice(&mut data[1..]);

        data
    }
}

impl From<(CableNumber, MidiMessage)> for UsbMidiEventPacket {
    fn from(value: (CableNumber, MidiMessage)) -> Self {
        let (cable_number, message) = value;
        Self {
            cable_number,
            message,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum MidiPacketParsingError {
    MissingHeader,
    MissingDataPacket,
    ParseError(MidiParseError),
}

impl TryFrom<&[u8]> for UsbMidiEventPacket {
    type Error = MidiPacketParsingError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let raw_cable_number = match value.first() {
            Some(byte) => *byte >> 4,
            None => return Err(MidiPacketParsingError::MissingHeader),
        };

        let cable_number =
            CableNumber::try_from(raw_cable_number).expect("(u8 >> 4) < 16");
        let message_body = match value.get(1..) {
            Some(bytes) => bytes,
            None => return Err(MidiPacketParsingError::MissingDataPacket),
        };

        let message = MidiMessage::try_parse_slice(message_body)
            .map_err(MidiPacketParsingError::ParseError)?;

        Ok(UsbMidiEventPacket {
            cable_number,
            message,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        cable_number::CableNumber::{Cable0, Cable1},
        event_packet::UsbMidiEventPacket,
        midi_types::{Channel, Control, MidiMessage, Note, Program, Value14, Value7},
    };
    use core::convert::TryFrom;

    macro_rules! decode_message_test {
        ($($id:ident:$value:expr,)*) => {
            $(
                #[test]
                fn $id() {
                    let (usb_midi_data_packet, expected) = $value;
                    let message = UsbMidiEventPacket::try_from(&usb_midi_data_packet[..]).unwrap();
                    assert_eq!(expected, message);
                }
            )*
        }
    }

    const CHANNEL1: Channel = Channel::new(0);
    const CHANNEL2: Channel = Channel::new(1);

    decode_message_test! {
        note_on: ([9, 144, 36, 127], UsbMidiEventPacket {
            cable_number: Cable0,
            message: MidiMessage::NoteOn(CHANNEL1, Note::C1, Value7::new(127))
        }),
        note_off: ([8, 128, 36, 0], UsbMidiEventPacket {
            cable_number: Cable0,
            message: MidiMessage::NoteOff(CHANNEL1, Note::C1, Value7::new(0))
        }),
        polyphonic_aftertouch: ([10, 160, 36, 64], UsbMidiEventPacket {
            cable_number: Cable0,
            message: MidiMessage::KeyPressure(CHANNEL1, Note::C1, Value7::new(64))
        }),
        program_change: ([28, 192, 127, 0], UsbMidiEventPacket {
            cable_number: Cable1,
            message: MidiMessage::ProgramChange(CHANNEL1, Program::new(127))
        }),
        channel_aftertouch: ([13, 208, 127, 0], UsbMidiEventPacket {
            cable_number: Cable0,
            message: MidiMessage::ChannelPressure(CHANNEL1, Value7::new(127))
        }),
        pitch_wheel: ([14, 224, 64, 32], UsbMidiEventPacket {
            cable_number: Cable0,
            message: MidiMessage::PitchBendChange(CHANNEL1, Value14::from((64u8, 32u8)))
        }),
        control_change: ([11, 177, 1, 32], UsbMidiEventPacket {
            cable_number: Cable0,
            message: MidiMessage::ControlChange(CHANNEL2, Control::new(1), Value7::new(32))
        }),
    }
}
