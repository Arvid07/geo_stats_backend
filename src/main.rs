mod entities;
mod geo_guessr;
mod migrator;
mod requests;

use actix_cors::Cors;
use actix_web::web::Data;
use actix_web::{App, HttpServer};
use sea_orm::{Database, DbErr};
use crate::requests::get_requests::{get_comp_duels_avg, get_duels_game};
use crate::requests::insertion_requests::{insert_duels_game, insert_solo_game};

const DATABASE_URL: &str = "***REMOVED***";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();
    
    let db = Database::connect(DATABASE_URL)
        .await
        .unwrap_or_else(|db_err: DbErr| {
            eprintln!("Failed connecting to db: {}", db_err);
            std::process::exit(1);
        });

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .app_data(Data::new(db.clone()))
            .service(insert_duels_game)
            .service(insert_solo_game)
            .service(get_duels_game)
            .service(get_comp_duels_avg)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
