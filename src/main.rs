mod proto;
mod utils;
mod ws_handlers;

pub(crate) use anyhow::{anyhow as anyerror, Result as AnyResult};

use crate::proto::{PartyId, ALL_CLIENT_ID};
use crate::ws_handlers::{ClientActor, GameRoomRouterActor, InterActorMessage, ServerActor};
use actix::clock::Duration;
use actix::{Actor, Addr as ActorAddress};
use actix_web::middleware::Logger as ActixLogger;
use actix_web::web::{
    get, resource, route, Bytes, Data as SharedData, Payload, PayloadConfig, Query as RequestQuery,
};
use actix_web::{
    get, main as actix_main, App, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_actors::ws::start_with_addr as ws_start;
use log::info;
use serde::Deserialize;
use serde_json::to_string_pretty as to_json_pretty;
use std::io::Result as IOResult;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use structopt::StructOpt;
use utils::init_logger;
use uuid::Uuid;

pub(crate) const CLIENT_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Deserialize)]
struct ServerQueryParams {
    client_id: Uuid,
}

#[derive(Deserialize)]
struct ClientQueryParams {
    client_id: Uuid,
    room_id: u8,
}

/// PoC - Game Room Router
#[derive(StructOpt, Debug)]
#[structopt(name = "game-room")]
pub(crate) struct GameRoomOptions {
    // Debug Mode to enable INFO message
    #[structopt(short, long)]
    pub(crate) debug_mode: bool,
    /// Set server UUID/GUID
    #[structopt(short, long, default_value = "00000000-0000-0000-0000-000000000000")]
    pub(crate) server_uuid: Uuid,
    /// Set listening port
    #[structopt(short, long, default_value = "7575")]
    pub(crate) listen_port: u16,
}

pub(crate) struct HttpSharedState {
    server_joined: Arc<AtomicBool>,
    client_counter: Mutex<Vec<u32>>,
    acceptable_server_uuid: Uuid,
    available_rooms: Arc<RwLock<Vec<u8>>>,
    router_address: ActorAddress<GameRoomRouterActor>,
}

#[get("/")]
async fn get_available_rooms(shared_state: SharedData<HttpSharedState>) -> impl Responder {
    match shared_state.available_rooms.read() {
        Err(_) => HttpResponse::InternalServerError().body("Memory poisoning detected!").await,
        Ok(read_guard) => {
            let available_rooms_clone = (*read_guard).clone();
            HttpResponse::Ok().body(to_json_pretty(&available_rooms_clone).unwrap()).await
        }
    }
}

async fn reject_unmapped_handler() -> impl Responder {
    HttpResponse::NotFound().body("Nothing to look here...").await
}

async fn ws_server_upgrade(
    query_params: RequestQuery<ServerQueryParams>,
    shared_state: SharedData<HttpSharedState>,
    request: HttpRequest,
    stream: Payload,
) -> impl Responder {
    let client_id = query_params.client_id;

    // Check if the request is server connection and deny if already a server in this instance
    if client_id == shared_state.acceptable_server_uuid
        && shared_state.server_joined.load(Ordering::Relaxed)
    {
        return HttpResponse::Forbidden().body("Server already joined in this instance!").await;
    }

    // Check if the request is server connection
    if client_id == shared_state.acceptable_server_uuid {
        shared_state.server_joined.store(true, Ordering::Relaxed);
        let server_party_id = PartyId::Server(0); // One server connection only
        let server_actor =
            ServerActor::new(server_party_id, client_id, shared_state.router_address.clone());

        match ws_start(server_actor, &request, stream) {
            Err(error) => {
                shared_state.server_joined.store(false, Ordering::Relaxed);

                HttpResponse::InternalServerError().body(error.to_string()).await
            }
            Ok((server_address, response)) => {
                shared_state
                    .router_address
                    .do_send(InterActorMessage::ServerConnect(server_party_id, server_address));
                info!("Server with client id {} just joined...", client_id);

                response.await
            }
        }
    } else {
        HttpResponse::Forbidden().body("Invalid server client_id!").await
    }
}

async fn ws_client_upgrade(
    query_params: RequestQuery<ClientQueryParams>,
    shared_state: SharedData<HttpSharedState>,
    request: HttpRequest,
    stream: Payload,
) -> impl Responder {
    let client_id = query_params.client_id;
    let room_id = query_params.room_id;
    let room_id_usize = room_id as usize;

    match shared_state.client_counter.lock() {
        Err(_) => HttpResponse::InternalServerError().body("Memory poisoning detected!").await,
        Ok(mut client_counter_guard) => {
            if client_counter_guard[room_id_usize] >= ALL_CLIENT_ID {
                return HttpResponse::InternalServerError()
                    .body(format!("Server needs to rejoin for room {} is exhausted!", room_id))
                    .await;
            }

            let party_id = PartyId::from_u32(client_counter_guard[room_id_usize]);
            client_counter_guard[room_id_usize] += 1;
            let client_actor =
                ClientActor::new(party_id, client_id, shared_state.router_address.clone());

            match ws_start(client_actor, &request, stream) {
                Err(error) => HttpResponse::InternalServerError().body(error.to_string()).await,
                Ok((client_address, response)) => {
                    shared_state.router_address.do_send(InterActorMessage::ClientConnect(
                        room_id,
                        party_id,
                        client_id,
                        client_address,
                    ));
                    info!("Client with client id {} just joined to room {}...", client_id, room_id);

                    response.await
                }
            }
        }
    }
}

#[actix_main]
async fn main() -> IOResult<()> {
    let options = GameRoomOptions::from_args();
    init_logger(options.debug_mode);
    let listen_socket = format!("0.0.0.0:{}", options.listen_port);

    let available_rooms = Arc::new(RwLock::new(Vec::new()));
    let server_joined = Arc::new(AtomicBool::new(false));
    let mut client_counter = Vec::new();

    for _ in 0..256 {
        client_counter.push(0);
    }

    let router_address =
        GameRoomRouterActor::new(available_rooms.clone(), server_joined.clone()).start();
    let shared_state = SharedData::new(HttpSharedState {
        available_rooms: available_rooms.clone(),
        client_counter: Mutex::new(client_counter),
        acceptable_server_uuid: options.server_uuid,
        router_address,
        server_joined,
    });

    HttpServer::new(move || {
        let shared_state_clone = shared_state.clone();
        App::new()
            .app_data(shared_state_clone)
            .app_data(PayloadConfig::new(8 * 1024 * 1024))
            .app_data(Bytes::configure(|cfg| cfg.limit(8 * 1024 * 1024)))
            .wrap(ActixLogger::default())
            .service(get_available_rooms)
            .service(resource("/server").route(get().to(ws_server_upgrade)))
            .service(resource("/client").route(get().to(ws_client_upgrade)))
            .default_service(route().to(reject_unmapped_handler))
    })
    .client_timeout(500)
    .client_shutdown(500)
    .shutdown_timeout(1)
    .bind(listen_socket)
    .unwrap()
    .run()
    .await
}
