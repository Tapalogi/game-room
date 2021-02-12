use structopt::StructOpt;
use uuid::Uuid;

/// PoC - Game Room Router
#[derive(StructOpt, Debug)]
#[structopt(name = "game-room")]
struct GameRoomOptions {
    /// Set server UUID/GUID
    #[structopt(short, long, default_value = "00000000-0000-0000-0000-000000000000")]
    server_uuid: Uuid,
    /// Set listening port
    #[structopt(short, long, default_value = "7575")]
    listen_port: u16,
}

fn main() {
    let options = GameRoomOptions::from_args();
    println!("{:#?}", options);
}
