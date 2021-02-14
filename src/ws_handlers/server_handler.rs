use crate::proto::{MessageStream, PartyId};
use crate::ws_handlers::{
    GameRoomRouterActor, InterActorMessage, HEARTBEAT_INTERVAL, MAILBOX_CAPACITY,
};
use crate::CLIENT_TIMEOUT;
use actix::clock::Instant;
use actix::{
    Actor as ActixActor, ActorContext, Addr as ActorAddress, AsyncContext, Handler as SendHandler,
    Running, StreamHandler as ReceiveHandler,
};
use actix_web_actors::ws::{
    CloseReason, Message as WsMessage, ProtocolError as WsProtocolError, WebsocketContext,
};
use log::warn;

#[derive(Debug)]
pub(crate) struct ServerActor {
    party_id: PartyId,
    last_known_activity: Instant,
    router_actor: ActorAddress<GameRoomRouterActor>,
}

impl ServerActor {
    pub(crate) fn new(party_id: PartyId, router_actor: ActorAddress<GameRoomRouterActor>) -> Self {
        Self { party_id, last_known_activity: Instant::now(), router_actor }
    }

    pub(crate) fn heartbeat(&self, context: &mut WebsocketContext<Self>) {
        context.run_interval(HEARTBEAT_INTERVAL, |actor, context| {
            if Instant::now().duration_since(actor.last_known_activity) > CLIENT_TIMEOUT {
                Self::close_and_disconnect(context, None);
            } else {
                context.ping(b"");
            }
        });
    }

    pub(crate) fn update_last_known_activity(&mut self) {
        self.last_known_activity = Instant::now();
    }

    pub(crate) fn close_and_disconnect(
        context: &mut WebsocketContext<Self>,
        reason: Option<CloseReason>,
    ) {
        context.close(reason);
        context.stop();
    }
}

impl ActixActor for ServerActor {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        context.set_mailbox_capacity(MAILBOX_CAPACITY);
        self.heartbeat(context);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.router_actor.do_send(InterActorMessage::Disconnect(self.party_id));
        Running::Stop
    }
}

impl SendHandler<MessageStream> for ServerActor {
    type Result = ();

    fn handle(&mut self, message: MessageStream, context: &mut Self::Context) {
        context.binary(message.into_raw());
    }
}

impl ReceiveHandler<Result<WsMessage, WsProtocolError>> for ServerActor {
    fn handle(
        &mut self,
        stream_result: Result<WsMessage, WsProtocolError>,
        context: &mut Self::Context,
    ) {
        if let Ok(payload) = stream_result {
            match payload {
                WsMessage::Close(reason) => {
                    Self::close_and_disconnect(context, reason);
                }
                WsMessage::Pong(_) => self.update_last_known_activity(),
                WsMessage::Ping(ping_payload) => {
                    self.update_last_known_activity();
                    context.pong(&ping_payload);
                }
                WsMessage::Binary(binary_payload) => {
                    self.update_last_known_activity();

                    if let Ok(message_stream) = MessageStream::from_raw(&binary_payload) {
                        self.router_actor
                            .do_send(InterActorMessage::NewMessage(self.party_id, message_stream));
                    }
                }
                WsMessage::Text(text_payload) => {
                    let text_payload = text_payload.trim();
                    warn!("Server is not supposed to send TEXT to the server. The server said: \"{}\"", text_payload);
                    context.text(format!(
                        "You're not supposed to send TEXT to the server. You said: \"{}\"",
                        text_payload
                    ));
                }
                _ => (),
            }
        } else {
            Self::close_and_disconnect(context, None);
        }
    }
}
