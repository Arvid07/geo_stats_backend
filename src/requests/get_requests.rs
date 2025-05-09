use crate::entities::player::Model as PlayerModel;
use crate::entities::prelude::{DuelsGame, DuelsRound, Guess, Player};
use crate::entities::{duels_game, guess};
use crate::geo_guessr::{GeoMode, TeamGameMode};
use crate::login::get_player_id_from_session;
use actix_web::error::{ErrorConflict, ErrorInternalServerError, ErrorNotFound, ErrorUnauthorized};
use actix_web::{get, web, Error, HttpRequest, HttpResponse, Responder};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DuelsGameRequest {
    game_id: String
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DuelsGameResponse {
    game_id: String,
    team_id1: String,
    team_id2: String,
    health_team1: usize,
    health_team2: usize,
    game_mode: TeamGameMode,
    geo_mode: GeoMode,
    start_time: String,
    map_id: String
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HomePageResponse {
    stats: Option<HashMap<String, i32>>,
    enemy_stats: Option<HashMap<String, i32>>,
    player: PlayerModel
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

#[get("/home-page")]
pub async fn get_home_page(
    db: web::Data<DatabaseConnection>,
    http_request: HttpRequest
) -> Result<impl Responder, Error> {
    let session_id = match http_request.cookie("sessionId") {
        Some(cookie) => {
            String::from(cookie.value())
        },
        None => return Err(ErrorUnauthorized("Missing `sessionId` cookie!"))
    };
    
    let db = db.get_ref();
    let player_id = get_player_id_from_session(&session_id, db).await?;
    
    let player = match Player::find_by_id(&player_id).one(db).await {
        Ok(player_option) => {
            match player_option {
                Some(player) => player,
                None => return Err(ErrorConflict("Account is not linked correctly!"))
            }
        },
        Err(err) => return Err(ErrorInternalServerError(err.to_string()))
    };
    
    let games_rounds = match DuelsGame::find()
        .filter(duels_game::Column::TeamGameMode.eq(TeamGameMode::DuelsRanked.to_string())
                .and(duels_game::Column::TeamId1.eq(&player_id).or(duels_game::Column::TeamId2.eq(&player_id)))
        ).find_with_related(DuelsRound)
        .all(db).await 
    {
        Ok(games_found) => {
            if games_found.is_empty() {
                let response = HomePageResponse {
                    stats: None,
                    enemy_stats: None,
                    player,
                };
                
                return Ok(HttpResponse::PartialContent().json(response));
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

    let (avg_score, enemy_avg_score) = match tokio::try_join!(
        get_avg_score(db, guess_ids, format!("Could not find any guesses for player: {}", player_id)),
        get_avg_score(db, enemy_guess_ids, String::from("Could not find any enemy guesses"))
    ) {
        Ok((a, b)) => (a, b),
        Err(err) => return Err(ErrorNotFound(err))
    };
    
    let response = HomePageResponse {
        stats: Some(avg_score),
        enemy_stats: Some(enemy_avg_score),
        player,
    };
    
    Ok(HttpResponse::Ok().json(response))
}
