use crate::entities::player::Model as PlayerModel;
use crate::entities::prelude::{DuelsGame, Guess, Player};
use crate::entities::{duels_game, guess};
use crate::geo_guessr::TeamGameMode;
use crate::login::get_player_id_from_session;
use actix_web::error::{ErrorConflict, ErrorInternalServerError, ErrorUnauthorized};
use actix_web::{get, web, Error, HttpRequest, HttpResponse, Responder};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HomePageResponse {
    stats: Option<HashMap<String, i32>>,
    enemy_stats: Option<HashMap<String, i32>>,
    player: PlayerModel
}

async fn get_avg_score(db: &DatabaseConnection, guess_ids: Vec<String>, error_message: String) -> Result<HashMap<String, i32>, Error> {
    let guesses = match Guess::find().filter(guess::Column::Id.is_in(guess_ids)).all(db).await {
        Ok(guesses_found) => {
            if guesses_found.is_empty() {
                return Err(ErrorInternalServerError(error_message));
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
    
    let games_guesses = match DuelsGame::find()
        .filter(duels_game::Column::TeamGameMode.eq(TeamGameMode::DuelsRanked.to_string())
                .and(duels_game::Column::TeamId1.eq(&player_id).or(duels_game::Column::TeamId2.eq(&player_id)))
        ).find_with_related(Guess)
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
    
    let mut guess_ids = Vec::new();
    let mut enemy_guess_ids = Vec::new();
    
    for (_, guesses) in games_guesses {
        for guess in guesses {
            if guess.team_id == player_id {
                guess_ids.push(guess.id);
            } else {
                enemy_guess_ids.push(guess.id);
            }
        }
    }

    let (avg_score, enemy_avg_score) = match tokio::try_join!(
        get_avg_score(db, guess_ids, format!("Could not find any guesses for player: {}", player_id)),
        get_avg_score(db, enemy_guess_ids, String::from("Could not find any enemy guesses"))
    ) {
        Ok((a, b)) => (a, b),
        Err(err) => return Err(ErrorInternalServerError(err))
    };
    
    let response = HomePageResponse {
        stats: Some(avg_score),
        enemy_stats: Some(enemy_avg_score),
        player,
    };
    
    Ok(HttpResponse::Ok().json(response))
}
