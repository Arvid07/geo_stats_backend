use actix_web::{get, web, HttpResponse, Responder};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use serde::Deserialize;
use crate::entities::duels_game::Model;
use crate::entities::duels_round;
use crate::entities::prelude::{DuelsGame, DuelsRound};

#[derive(Deserialize)]
struct DuelsGameRequest {
    game_id: String
}

#[get("/duels-game")]
pub async fn get_duels_game(
    db: web::Data<DatabaseConnection>,
    request: web::Json<DuelsGameRequest>,
) -> impl Responder {
    let db = db.get_ref();
    
    let game = match DuelsGame::find_by_id(request.game_id.clone()).one(db).await {
        Ok(response) => {
            if let Some(game_object) = response {
                game_object
            } else {
                return HttpResponse::NotFound().body(format!("Could not find Game with id {}", request.game_id));
            }
        }
        Err(error) => return HttpResponse::InternalServerError().body("Fetching Game operation failed!")
    };
    
    let rounds = match DuelsRound::find().filter(duels_round::Column::GameId.eq(request.game_id.clone())).all(db).await {
        Ok(rounds) => {
            rounds
        }
        Err(_) => return HttpResponse::InternalServerError().body("Fetching Rounds operation failed!")
    };
    
    HttpResponse::Ok().body("")
}
