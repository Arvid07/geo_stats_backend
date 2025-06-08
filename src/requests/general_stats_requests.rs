use crate::entities::guess::Model as GuessModel;
use crate::entities::player::Model as PlayerModel;
use crate::entities::prelude::{CompTeam, DuelsGame, Guess};
use crate::entities::{comp_team, duels_game, guess};
use crate::geo_guessr::TeamGameMode;
use crate::login::get_player_from_session;
use actix_web::error::{ErrorInternalServerError, ErrorUnauthorized};
use actix_web::{get, web, Error, HttpRequest, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
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
    duels: Vec<StatsGuess>,
    duels_ranked: Vec<StatsGuess>,
    team_duels: Vec<StatsGuess>,
    team_duels_ranked: Vec<StatsGuess>,
    team_fun: Vec<StatsGuess>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsGuess {
    time: i64,
    round_country_code: String,
    guess_country_code: Option<String>,
    points: usize
}

async fn get_processed_stats(guesses: Vec<GuessModel>, guess_id_to_game_mode: &HashMap<String, String>) -> Result<Stats, Error> {
    let mut duels = Vec::new();
    let mut duels_ranked = Vec::new();
    let mut team_duels = Vec::new();
    let mut team_duels_ranked = Vec::new();
    let mut team_fun = Vec::new();

    for guess in guesses {
        let date: DateTime<Utc> = guess.date.parse().unwrap();
        
        let stats_guess = StatsGuess {
            time: date.timestamp_millis(),
            round_country_code: guess.round_country_code,
            guess_country_code: guess.country_code,
            points: guess.score as usize
        };
        
        match TeamGameMode::from_str(guess_id_to_game_mode.get(&guess.id).unwrap()).unwrap() {
            TeamGameMode::Duels => {
                duels.push(stats_guess);
            }
            TeamGameMode::DuelsRanked => {
                duels_ranked.push(stats_guess);
            }
            TeamGameMode::TeamDuels => {
                team_duels.push(stats_guess);      
            }
            TeamGameMode::TeamDuelsRanked => {
                team_duels_ranked.push(stats_guess);
            }
            TeamGameMode::TeamFun => {
                team_fun.push(stats_guess);
            }
        }
    }
    
    duels.sort_unstable_by(|a, b| b.time.cmp(&a.time));
    duels_ranked.sort_unstable_by(|a, b| b.time.cmp(&a.time));
    team_duels.sort_unstable_by(|a, b| b.time.cmp(&a.time));
    team_duels_ranked.sort_unstable_by(|a, b| b.time.cmp(&a.time));
    team_fun.sort_unstable_by(|a, b| b.time.cmp(&a.time));
    
    let stats = Stats {
        duels,
        duels_ranked,
        team_duels,
        team_duels_ranked,
        team_fun
    };

    Ok(stats)
}

#[get("/stats")]
pub async fn get_general_stats(
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
    let player = get_player_from_session(&session_id, db).await?;
    let mut team_ids: HashSet<String> = [player.id.clone()].into_iter().collect();
    
    match CompTeam::find()
        .filter(comp_team::Column::PlayerId1.eq(&player.id).or(comp_team::Column::PlayerId2.eq(&player.id)))
        .all(db)
        .await 
    {
        Ok(teams) => team_ids.extend(teams.into_iter().map(|team| team.team_id)),
        Err(err) => return Err(ErrorInternalServerError(err))
    };

    let games_guesses = match DuelsGame::find()
        .filter(duels_game::Column::TeamId1.is_in(&team_ids).or(duels_game::Column::TeamId2.is_in(&team_ids)))
        .find_with_related(Guess)
        .filter(guess::Column::IsTeamsBest.eq(true))
        .all(db)
        .await 
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
    
    let mut player_guesses = Vec::new();
    let mut enemy_guesses = Vec::new();
    let mut guess_id_to_game_mode = HashMap::with_capacity(player_guesses.len());
    
    for (game, guesses) in games_guesses {
        for guess in guesses {
            if team_ids.contains(&guess.team_id) {
                guess_id_to_game_mode.insert(guess.id.clone(), game.team_game_mode.clone());
                player_guesses.push(guess);
            } else {
                guess_id_to_game_mode.insert(guess.id.clone(), game.team_game_mode.clone());
                enemy_guesses.push(guess);
            }
        }
    }
    
    let (stats, enemy_stats) = match tokio::try_join!(
        get_processed_stats(player_guesses, &guess_id_to_game_mode),
        get_processed_stats(enemy_guesses, &guess_id_to_game_mode)
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