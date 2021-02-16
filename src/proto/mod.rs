mod message_stream;

pub(crate) use message_stream::MessageStream;

use num_enum::IntoPrimitive;
use serde::{Deserialize, Serialize};

pub(crate) const ALL_SERVER_ID_WITH_ECHO: u32 = 0xFFFF_FFFF;
pub(crate) const ALL_CLIENT_ID_WITH_ECHO: u32 = 0x7FFF_FFFF;
pub(crate) const ALL_SERVER_ID: u32 = 0xFFFF_FFFE;
pub(crate) const ALL_CLIENT_ID: u32 = 0x7FFF_FFFE;
pub(crate) const OFFSET_SERVER_ID: u32 = 0x8000_0000;

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
        matches!(self, Self::Client(_))
    }

    pub(crate) fn is_single_server_id(&self) -> bool {
        matches!(self, Self::Server(_))
    }
}
