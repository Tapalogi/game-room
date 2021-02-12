mod utils;

use actix_web::middleware::Logger as ActixLogger;
use actix_web::web::{get, resource, route, Bytes, Data as SharedData, Payload, PayloadConfig};
use actix_web::{
    get, main as actix_main, App, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder,
};
use serde_json::to_string_pretty as to_json_pretty;
use std::io::Result as IOResult;
use std::sync::RwLock;
use structopt::StructOpt;
use utils::init_logger;
use uuid::Uuid;

/// PoC - Game Room Router
#[derive(StructOpt, Debug)]
#[structopt(name = "game-room")]
pub(crate) struct GameRoomOptions {
    /// Set server UUID/GUID
    #[structopt(short, long, default_value = "00000000-0000-0000-0000-000000000000")]
    pub(crate) server_uuid: Uuid,
    /// Set listening port
    #[structopt(short, long, default_value = "7575")]
    pub(crate) listen_port: u16,
}

pub(crate) struct GameRoomStates {
    acceptable_server_uuid: Uuid,
    available_rooms: RwLock<Vec<u8>>,
}

#[get("/")]
async fn get_available_rooms(shared_state: SharedData<GameRoomStates>) -> impl Responder {
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

#[actix_main]
async fn main() -> IOResult<()> {
    init_logger();
    let options = GameRoomOptions::from_args();
    let listen_socket = format!("0.0.0.0:{}", options.listen_port);
    let state = GameRoomStates {
        available_rooms: RwLock::new(Vec::new()),
        acceptable_server_uuid: options.server_uuid,
    };
    let shared_state = SharedData::new(state);
    HttpServer::new(move || {
        let shared_state_clone = shared_state.clone();
        App::new()
            .app_data(shared_state_clone)
            .app_data(PayloadConfig::new(8 * 1024 * 1024))
            .app_data(Bytes::configure(|cfg| cfg.limit(8 * 1024 * 1024)))
            .wrap(ActixLogger::default())
            .service(get_available_rooms)
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
