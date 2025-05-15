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
use std::str::FromStr;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HomePageResponse {
    stats: Option<Stats>,
    enemy_stats: Option<Stats>,
    player: PlayerModel
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Stats {
    duels: HashMap<String, CountryStats>,
    duels_ranked: HashMap<String, CountryStats>,
    team_duels: HashMap<String, CountryStats>,
    team_duels_ranked: HashMap<String, CountryStats>,
    team_fun: HashMap<String, CountryStats>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CountryStats {
    points: usize,
    count: usize
}

struct Score {
    guess_id: String,
    country_code: String,
    score: usize
}

async fn get_stats(db: &DatabaseConnection, guess_ids: Vec<String>, guess_id_to_game_mode: &HashMap<String, String>, error_message: String) -> Result<Stats, Error> {
    let guesses = match Guess::find().filter(guess::Column::Id.is_in(guess_ids)).all(db).await {
        Ok(guesses_found) => {
            if guesses_found.is_empty() {
                return Err(ErrorInternalServerError(error_message));
            }
            guesses_found
        }
        Err(err) => return Err(ErrorInternalServerError(err.to_string()))
    };

    let mut scores = Vec::with_capacity(guesses.len());

    for guess in guesses {
        scores.push(
            Score {
                guess_id: guess.id, 
                country_code: 
                guess.round_country_code,
                score: guess.score as usize
            });
    }

    let mut duels_score = HashMap::new();
    let mut duels_ranked = HashMap::new();
    let mut team_duels = HashMap::new();
    let mut team_duels_ranked = HashMap::new();
    let mut team_fun = HashMap::new();

    for score in scores {
        match TeamGameMode::from_str(guess_id_to_game_mode.get(&score.guess_id).unwrap()).unwrap() {
            TeamGameMode::Duels => {
                duels_score.entry(score.country_code).and_modify(|entry: &mut (usize, usize)| {
                    entry.0 += score.score;
                    entry.1 += 1;
                }).or_insert((score.score, 1));
            }
            TeamGameMode::DuelsRanked => {
                duels_ranked.entry(score.country_code).and_modify(|entry: &mut (usize, usize)| {
                    entry.0 += score.score;
                    entry.1 += 1;
                }).or_insert((score.score, 1));
            }
            TeamGameMode::TeamDuels => {
                team_duels.entry(score.country_code).and_modify(|entry: &mut (usize, usize)| {
                    entry.0 += score.score;
                    entry.1 += 1;
                }).or_insert((score.score, 1));            
            }
            TeamGameMode::TeamDuelsRanked => {
                team_duels_ranked.entry(score.country_code).and_modify(|entry: &mut (usize, usize)| {
                    entry.0 += score.score;
                    entry.1 += 1;
                }).or_insert((score.score, 1));
            }
            TeamGameMode::TeamFun => {
                team_fun.entry(score.country_code).and_modify(|entry: &mut (usize, usize)| {
                    entry.0 += score.score;
                    entry.1 += 1;
                }).or_insert((score.score, 1));
            }
        }
    }
    
    let stats = Stats {
        duels: duels_score
            .into_iter()
            .map(|(country_code, (points, amount))| (country_code, CountryStats { points, count: amount }))
            .collect(),
        duels_ranked: duels_ranked
            .into_iter()
            .map(|(country_code, (points, amount))| (country_code, CountryStats { points, count: amount }))
            .collect(),
        team_duels: team_duels
            .into_iter()
            .map(|(country_code, (points, amount))| (country_code, CountryStats { points, count: amount }))
            .collect(),
        team_duels_ranked: team_duels_ranked
            .into_iter()
            .map(|(country_code, (points, amount))| (country_code, CountryStats { points, count: amount }))
            .collect(),
        team_fun: team_fun
            .into_iter()
            .map(|(country_code, (points, amount))| (country_code, CountryStats { points, count: amount }))
            .collect()
    };

    Ok(stats)
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
        .filter(duels_game::Column::TeamId1.eq(&player_id).or(duels_game::Column::TeamId2.eq(&player_id)))
        .find_with_related(Guess)
        .all(db).await 
    {
        Ok(games_found) => {
            if games_found.is_empty() {
                let response = HomePageResponse {
                    stats: None,
                    enemy_stats: None,
                    player
                };
                
                return Ok(HttpResponse::PartialContent().json(response));
            }
            games_found
        }
        Err(err) => return Err(ErrorInternalServerError(err.to_string()))
    };
    
    let mut guess_ids = Vec::new();
    let mut enemy_guess_ids = Vec::new();
    let mut guess_id_to_game_mode = HashMap::with_capacity(guess_ids.len());
    
    for (game, guesses) in games_guesses {
        for guess in guesses {
            if guess.team_id == player_id {
                guess_ids.push(guess.id.clone());
                guess_id_to_game_mode.insert(guess.id, game.team_game_mode.clone());
            } else {
                enemy_guess_ids.push(guess.id.clone());
                guess_id_to_game_mode.insert(guess.id, game.team_game_mode.clone());
            }
        }
    }
    
    let (stats, enemy_stats) = match tokio::try_join!(
        get_stats(db, guess_ids, &guess_id_to_game_mode, format!("Could not find any guesses for player: {}", player_id)),
        get_stats(db, enemy_guess_ids, &guess_id_to_game_mode, String::from("Could not find any enemy guesses"))
    ) {
        Ok((a, b)) => (a, b),
        Err(err) => return Err(ErrorInternalServerError(err))
    };
    
    let response = HomePageResponse {
        stats: Some(stats),
        enemy_stats: Some(enemy_stats),
        player
    };
    
    Ok(HttpResponse::Ok().json(response))
}
