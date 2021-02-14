mod client_handler;
mod server_handler;

use crate::proto::{MessageCode, MessageStream, PartyId, PayloadKind};
use actix::clock::Duration;
use actix::{
    Actor as ActixActor, Addr as ActorAddress, Context, Handler as MessageHandler, Message, Running,
};
use log::error;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

pub(crate) const MAILBOX_CAPACITY: usize = 256;
pub(crate) const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) use client_handler::ClientActor;
pub(crate) use server_handler::ServerActor;

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub(crate) enum InterActorMessage {
    ServerConnect(PartyId, ActorAddress<ServerActor>),
    ClientConnect(u8, PartyId, Uuid, ActorAddress<ClientActor>),
    Disconnect(PartyId, Option<Uuid>),  // u32 -> Origin Party ID
    NewMessage(PartyId, MessageStream), // u32 -> Origin Party ID
}

#[derive(Debug)]
pub(crate) struct GameRoomRouterActor {
    pub(crate) available_rooms: Arc<RwLock<Vec<u8>>>,
    pub(crate) server_handle: Option<(u32, ActorAddress<ServerActor>)>,
    pub(crate) server_joined: Arc<AtomicBool>,
    pub(crate) game_rooms: BTreeMap<u8, BTreeMap<u32, (Uuid, ActorAddress<ClientActor>)>>,
}

impl GameRoomRouterActor {
    pub(crate) fn new(
        available_rooms: Arc<RwLock<Vec<u8>>>,
        server_joined: Arc<AtomicBool>,
    ) -> Self {
        Self { available_rooms, server_joined, server_handle: None, game_rooms: Default::default() }
    }
}

impl ActixActor for GameRoomRouterActor {
    type Context = Context<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        context.set_mailbox_capacity(MAILBOX_CAPACITY);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        Running::Stop
    }
}

impl MessageHandler<InterActorMessage> for GameRoomRouterActor {
    type Result = ();

    fn handle(&mut self, message: InterActorMessage, _: &mut Self::Context) {
        match message {
            InterActorMessage::ServerConnect(party_id, server_address) => {
                self.server_handle = Some((party_id.get_repr(), server_address));
            }
            InterActorMessage::ClientConnect(room_id, party_id, client_id, client_address) => {
                let mut room_entry = self.game_rooms.entry(room_id).or_default();
                let _ = room_entry.insert(party_id.get_repr(), (client_id, client_address));
                let mut hello_payload = [0; 17];
                hello_payload[0] = 0xF0;
                hello_payload[1..=16].copy_from_slice(&client_id.as_bytes()[..]);

                let join_info = MessageStream::new(
                    MessageCode::Special,
                    room_id as u32,
                    party_id,
                    PartyId::Server(0),
                    PayloadKind::Info,
                    Some(&hello_payload),
                );

                if let Some((_, server_actor)) = self.server_handle.as_ref() {
                    server_actor.do_send(InterActorMessage::NewMessage(party_id, join_info));
                }
            }
            InterActorMessage::Disconnect(party_id, _) => {
                if party_id == PartyId::Server(0) {
                    self.server_joined.store(false, Ordering::Relaxed);

                    if let Ok(mut write_guard) = self.available_rooms.write() {
                        write_guard.clear();
                    }

                    // This will be a recursive call to this branch
                    let game_room_iter = self.game_rooms.iter();

                    for (_, rooms) in game_room_iter {
                        let room_iter = rooms.iter();

                        for (party_id_raw, room_client) in room_iter {
                            room_client.1.do_send(InterActorMessage::Disconnect(
                                PartyId::from_u32(*party_id_raw),
                                Some(room_client.0),
                            ));
                        }
                    }

                    self.server_handle = None;
                } else {
                    let game_room_iter = self.game_rooms.iter_mut();

                    for (room_id, rooms) in game_room_iter {
                        let removed_client = rooms.remove(&party_id.get_repr());

                        if removed_client.is_some() && self.server_handle.is_some() {
                            let (client_id, _) = removed_client.unwrap();
                            let (_, server_handle) = self.server_handle.as_ref().unwrap();

                            let mut goodbye_payload = [0; 17];
                            goodbye_payload[0] = 0x0F;
                            goodbye_payload[1..=16].copy_from_slice(&client_id.as_bytes()[..]);

                            let exit_info = MessageStream::new(
                                MessageCode::Special,
                                *room_id as u32,
                                party_id,
                                PartyId::Server(0),
                                PayloadKind::Info,
                                Some(&goodbye_payload),
                            );

                            server_handle
                                .do_send(InterActorMessage::NewMessage(party_id, exit_info));
                        }
                    }
                }
            }
            InterActorMessage::NewMessage(party_id, message_stream) => {
                if party_id == PartyId::Server(0) {
                    match message_stream.message_code {
                        MessageCode::Normal => {
                            // TODO do proper message routing
                            self.server_handle
                                .as_ref()
                                .unwrap()
                                .1
                                .do_send(InterActorMessage::NewMessage(party_id, message_stream));
                        }
                        MessageCode::Special => {
                            let mut room_list = message_stream.payload;
                            room_list.sort();
                            room_list.dedup();
                            let mut room_count = room_list.len();

                            if room_count > 256 {
                                room_count = 256;
                            }

                            if let Ok(mut write_guard) = self.available_rooms.write() {
                                write_guard.resize(room_count, 0);

                                for i in 0..room_count {
                                    *write_guard.get_mut(i).unwrap() = room_list[i];
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
