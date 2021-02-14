use crate::{anyerror, AnyResult};
use actix::Message;
use num_enum::IntoPrimitive;
use serde::{Deserialize, Serialize};
use std::ops::Range;

pub(crate) const PREAMBLE: u32 = 0xFEED_BEEF;
pub(crate) const ALL_SERVER_ID_WITH_ECHO: u32 = 0xFFFF_FFFF;
pub(crate) const ALL_CLIENT_ID_WITH_ECHO: u32 = 0x7FFF_FFFF;
pub(crate) const ALL_SERVER_ID: u32 = 0xFFFF_FFFE;
pub(crate) const ALL_CLIENT_ID: u32 = 0x7FFF_FFFE;
pub(crate) const OFFSET_SERVER_ID: u32 = 0x8000_0000;
pub(crate) const LENGTH_MESSAGE_STREAM_HEADER: usize = 20;
pub(crate) const RANGE_PREAMBLE: Range<usize> = 0..4;
pub(crate) const RANGE_MESSAGE_CODE: Range<usize> = 4..5;
pub(crate) const RANGE_ROOM_ID: Range<usize> = 5..9;
pub(crate) const RANGE_ORIGIN_ID: Range<usize> = 9..13;
pub(crate) const RANGE_DESTINATION_ID: Range<usize> = 13..17;
pub(crate) const RANGE_PAYLOAD_TYPE: Range<usize> = 17..18;
pub(crate) const RANGE_PAYLOAD_LENGTH: Range<usize> = 18..20;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, IntoPrimitive, PartialEq, Serialize)]
pub(crate) enum MessageCode {
    Special = 0x5E,
    Normal = 0x00,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, IntoPrimitive, PartialEq, Serialize)]
pub(crate) enum PayloadKind {
    Command = 0xC0,
    Data = 0xDA,
    Info = 0x1F,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum PartyId {
    AllClients,
    AllServers,
    AllClientsWithEcho,
    AllServersWithEcho,
    Client(u32),
    Server(u32),
}

impl PartyId {
    pub(crate) fn from_u32(party_id_value: u32) -> Self {
        if party_id_value == ALL_CLIENT_ID {
            return Self::AllClients;
        }

        if party_id_value == ALL_SERVER_ID {
            return Self::AllServers;
        }

        if party_id_value == ALL_CLIENT_ID_WITH_ECHO {
            return Self::AllClientsWithEcho;
        }

        if party_id_value == ALL_SERVER_ID_WITH_ECHO {
            return Self::AllServersWithEcho;
        }

        if party_id_value >= OFFSET_SERVER_ID {
            return Self::Server(party_id_value - OFFSET_SERVER_ID);
        }

        Self::Client(party_id_value)
    }

    pub(crate) fn get_repr(&self) -> u32 {
        match self {
            Self::AllClientsWithEcho => ALL_CLIENT_ID_WITH_ECHO,
            Self::AllClients => ALL_CLIENT_ID,
            Self::AllServersWithEcho => ALL_SERVER_ID_WITH_ECHO,
            Self::AllServers => ALL_SERVER_ID,
            Self::Client(client_id) => *client_id,
            Self::Server(server_id) => OFFSET_SERVER_ID + server_id,
        }
    }

    pub(crate) fn to_le_bytes(self) -> [u8; 4] {
        self.get_repr().to_le_bytes()
    }

    pub(crate) fn is_single_client_id(&self) -> bool {
        match self {
            Self::Client(_) => true,
            _ => false,
        }
    }

    pub(crate) fn is_single_server_id(&self) -> bool {
        match self {
            Self::Server(_) => true,
            _ => false,
        }
    }
}

#[rtype(result = "()")]
#[derive(Clone, Debug, Message, PartialEq, Eq)]
pub(crate) struct MessageStream {
    pub(crate) message_code: MessageCode,
    pub(crate) room_id: u32,
    pub(crate) origin_id: PartyId,
    pub(crate) destination_id: PartyId,
    pub(crate) payload_kind: PayloadKind,
    pub(crate) payload: Vec<u8>,
}

impl MessageStream {
    pub(crate) fn new(
        message_code: MessageCode,
        room_id: u32,
        origin_id: PartyId,
        destination_id: PartyId,
        payload_kind: PayloadKind,
        payload: Option<&[u8]>,
    ) -> Self {
        let payload = if payload.is_none() { Vec::new() } else { payload.unwrap().into() };

        Self { message_code, room_id, origin_id, destination_id, payload_kind, payload }
    }

    pub(crate) fn from_raw(source: &[u8]) -> AnyResult<Self> {
        // Length check
        if source.len() < LENGTH_MESSAGE_STREAM_HEADER {
            return Err(anyerror!(
                "Source raw bytes length is less than the header length {}",
                LENGTH_MESSAGE_STREAM_HEADER
            ));
        }

        // Preamble
        let mut u32_bytes = [0u8; 4];
        u32_bytes.copy_from_slice(&source[RANGE_PREAMBLE]);

        if u32::from_le_bytes(u32_bytes) != PREAMBLE {
            return Err(anyerror!("Corrupted PREAMBLE, should be {}", PREAMBLE));
        }

        // MessageCode
        let message_code;

        match source[RANGE_MESSAGE_CODE] {
            [0x00] => message_code = MessageCode::Normal,
            [0x5E] => message_code = MessageCode::Special,
            _ => {
                return Err(anyerror!(
                    "Invalid MessageCode {:#?}",
                    source[RANGE_MESSAGE_CODE].to_vec()
                ))
            }
        }

        // Room ID
        u32_bytes.copy_from_slice(&source[RANGE_ROOM_ID]);
        let room_id = u32::from_le_bytes(u32_bytes);

        // Origin ID
        u32_bytes.copy_from_slice(&source[RANGE_ORIGIN_ID]);
        let origin_id = PartyId::from_u32(u32::from_le_bytes(u32_bytes));

        // Destination ID
        u32_bytes.copy_from_slice(&source[RANGE_DESTINATION_ID]);
        let destination_id = PartyId::from_u32(u32::from_le_bytes(u32_bytes));

        // Payload Type
        let payload_kind;

        match source[RANGE_PAYLOAD_TYPE] {
            [0xC0] => payload_kind = PayloadKind::Command,
            [0xDA] => payload_kind = PayloadKind::Data,
            [0x1F] => payload_kind = PayloadKind::Info,
            _ => {
                return Err(anyerror!(
                    "Invalid PayloadKind {:#?}",
                    source[RANGE_PAYLOAD_TYPE].to_vec()
                ))
            }
        }

        // Payload
        let mut u16_bytes = [0u8; 2];
        u16_bytes.copy_from_slice(&source[RANGE_PAYLOAD_LENGTH]);
        let payload_length = u16::from_le_bytes(u16_bytes);

        if payload_length as usize + LENGTH_MESSAGE_STREAM_HEADER != source.len() {
            return Err(anyerror!(
                "Source raw bytes length is less than the length {}",
                payload_length as usize + LENGTH_MESSAGE_STREAM_HEADER
            ));
        }

        let payload = if payload_length == 0 {
            None
        } else {
            let range_payload = LENGTH_MESSAGE_STREAM_HEADER
                ..(LENGTH_MESSAGE_STREAM_HEADER + payload_length as usize);
            Some(&source[range_payload])
        };

        Ok(Self::new(message_code, room_id, origin_id, destination_id, payload_kind, payload))
    }

    pub(crate) fn into_raw(self) -> Vec<u8> {
        let payload_length = self.payload.len() as u16;
        let mut result = vec![0u8; LENGTH_MESSAGE_STREAM_HEADER + payload_length as usize];

        // Unique Code => Offset 0, Length 4
        result[RANGE_PREAMBLE].copy_from_slice(&PREAMBLE.to_le_bytes());

        // Message Code => Offset 4, Length 1
        result[RANGE_MESSAGE_CODE].copy_from_slice(&[self.message_code.into()]);

        // Room ID => Offset 5, Length 4
        result[RANGE_ROOM_ID].copy_from_slice(&self.room_id.to_le_bytes());

        // Origin ID => Offset 9, Length 4
        result[RANGE_ORIGIN_ID].copy_from_slice(&self.origin_id.to_le_bytes());

        // Destination ID => Offset 13, Length 4
        result[RANGE_DESTINATION_ID].copy_from_slice(&self.destination_id.to_le_bytes());

        // Payload Type => Offset 17, Length 1
        result[RANGE_PAYLOAD_TYPE].copy_from_slice(&[self.payload_kind.into()]);

        // Payload Length => Offset 18, Length 2
        result[RANGE_PAYLOAD_LENGTH].copy_from_slice(&payload_length.to_le_bytes());

        if payload_length > 0 {
            // Payload => Offset 20, Length n
            let range_payload = LENGTH_MESSAGE_STREAM_HEADER
                ..(LENGTH_MESSAGE_STREAM_HEADER + payload_length as usize);
            result[range_payload].copy_from_slice(&self.payload[..]);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_stream_into_bytes_is_as_expected() {
        let expected_result = vec![
            0xEF, 0xBE, 0xED, 0xFE, 0x5E, 0x0A, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x80, 0x0C,
            0x00, 0x00, 0x00, 0xDA, 0x02, 0x00, 0xFF, 0xAA,
        ];
        let message_stream = MessageStream::new(
            MessageCode::Special,
            10,
            PartyId::Server(15),
            PartyId::Client(12),
            PayloadKind::Data,
            Some(&[0xFF, 0xAA]),
        );
        let message_stream_raw = message_stream.into_raw();

        assert_eq!(message_stream_raw, expected_result);
    }

    #[test]
    fn test_bytes_into_message_stream_is_as_expected() {
        let message_stream_raw = vec![
            0xEF, 0xBE, 0xED, 0xFE, 0x5E, 0x0A, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x80, 0x0C,
            0x00, 0x00, 0x00, 0xDA, 0x02, 0x00, 0xFF, 0xAA,
        ];
        let expected_result = MessageStream::new(
            MessageCode::Special,
            10,
            PartyId::Server(15),
            PartyId::Client(12),
            PayloadKind::Data,
            Some(&[0xFF, 0xAA]),
        );
        let message_stream = MessageStream::from_raw(&message_stream_raw).unwrap();

        assert_eq!(message_stream, expected_result);
    }
}
