use crate::entities::prelude::{DuelsGame, DuelsRound, Guess};
use crate::entities::{duels_game, guess};
use crate::geo_guessr::{GeoMode, TeamGameMode};
use actix_web::{get, web, Error, HttpResponse, Responder};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use actix_web::error::{ErrorInternalServerError, ErrorNotFound};

#[derive(Deserialize)]
struct DuelsGameRequest {
    game_id: String
}

#[derive(Serialize)]
struct DuelsGameResponse {
    game_id: String,
    team_id1: String,
    team_id2: String,
    health_team1: usize,
    health_team2: usize,
    game_mode: TeamGameMode,
    geo_mode: GeoMode,
    start_time: String,
    map_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompStatsResponse {
    stats: HashMap<String, i32>,
    enemy_stats: HashMap<String, i32>
}

#[get("/duels-game")]
pub async fn get_duels_game(
    db: web::Data<DatabaseConnection>,
    request: web::Json<DuelsGameRequest>
) -> impl Responder {
    let db = db.get_ref();
    
    let (game, rounds) = match DuelsGame::find_by_id(request.game_id.clone()).find_with_related(DuelsRound).all(db).await {
        Ok(response) => {
            if response.is_empty() {
                return HttpResponse::NotFound().body(format!("Could not find Game with id {}", request.game_id));
            }
            
            response.into_iter().next().unwrap()
        }
        Err(_) => return HttpResponse::InternalServerError().body("Fetching Game operation failed!")
    };
    
    let guess_ids: Vec<(&str, &str)> = rounds.iter().map(|round| (round.guess_id_team1.as_str(), round.guess_id_team2.as_str())).collect();
    
    HttpResponse::Ok().body("")
}

async fn get_avg_score(db: &DatabaseConnection, guess_ids: Vec<String>, error_message: String) -> Result<HashMap<String, i32>, Error> {
    let guesses = match Guess::find().filter(guess::Column::Id.is_in(guess_ids)).all(db).await {
        Ok(guesses_found) => {
            if guesses_found.is_empty() {
                return Err(ErrorNotFound(error_message));
            }
            guesses_found
        }
        Err(err) => return Err(ErrorInternalServerError(err.to_string()))
    };

    let mut score = HashMap::new();

    for guess in guesses {
        score.entry(guess.round_country_code).and_modify(|entry: &mut (i32, i32)| {
            entry.0 += guess.score;
            entry.1 += 1;
        }).or_insert((guess.score, 1));
    }

    Ok(score.into_iter().map(|(country_code, (points, amount))| (country_code, points / amount)).collect())
}

#[get("/games/comp-duels-avg/{player_id}")]
pub async fn get_comp_duels_avg(
    db: web::Data<DatabaseConnection>,
    path: web::Path<String>
) -> Result<impl Responder, Error> {
    let db = db.get_ref();
    let player_id = path.into_inner();
    
    let games_rounds = match DuelsGame::find()
        .filter(duels_game::Column::TeamGameMode.eq(TeamGameMode::DuelsRanked.to_string())
                .and(duels_game::Column::TeamId1.eq(&player_id).or(duels_game::Column::TeamId2.eq(&player_id)))
        ).find_with_related(DuelsRound)
        .all(db).await 
    {
        Ok(games_found) => {
            if games_found.is_empty() {
                return Err(ErrorNotFound(format!("Could not find any comp duel games for player {}", player_id)));
            }
            games_found
        }
        Err(err) => return Err(ErrorInternalServerError(err.to_string()))
    };

    let mut location_ids = Vec::new();
    let mut guess_ids = Vec::new();
    let mut enemy_guess_ids = Vec::new();

    for (game, rounds) in games_rounds {
        for round in rounds {
            if game.team_id1 == player_id {
                guess_ids.push(round.guess_id_team1);
                enemy_guess_ids.push(round.guess_id_team2);
            } else {
                guess_ids.push(round.guess_id_team2);
                enemy_guess_ids.push(round.guess_id_team1);
            }
            
            location_ids.push(round.location_id);
        }
    }
    
    let avg_score = get_avg_score(db, guess_ids, format!("Could not find any guesses for player: {}", player_id)).await?;
    let enemy_avg_score = get_avg_score(db, enemy_guess_ids, String::from("Could not find any enemy guesses")).await?;
    
    let response = CompStatsResponse {
        stats: avg_score,
        enemy_stats: enemy_avg_score
    };
    
    Ok(HttpResponse::Ok().json(response))
}
