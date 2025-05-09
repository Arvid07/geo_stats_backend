use crate::entities::comp_team::ActiveModel as CompTeamModel;
use crate::entities::fun_team::ActiveModel as FunTeamModel;
use crate::entities::duels_game::ActiveModel as DuelsGameModel;
use crate::entities::duels_round::ActiveModel as DuelsRoundModel;
use crate::entities::guess::ActiveModel as GuessModel;
use crate::entities::location::ActiveModel as LocationModel;
use crate::entities::map::ActiveModel as MapModel;
use crate::entities::player::ActiveModel as PlayerModel;
use crate::entities::solo_game::ActiveModel as SoloGameModel;
use crate::entities::solo_round::ActiveModel as SoloRoundModel;
use crate::entities::prelude::{CompTeam, DuelsGame, DuelsRound, FunTeam, Guess, Location, Map, Player, SoloGame, SoloRound};
use crate::geo_guessr::GeoMode::{Moving, NoMove, NoMovingZooming, NoPanning, NoPanningMoving, NoPanningZooming, NoZooming, NMPZ};
use crate::geo_guessr::{TeamGameMode, GameModeRatings, PlayerRankedSystemProgress, RankedTeam, User, RankedTeamDuelsProgress};
use crate::geo_guessr::{GeoMode, MovementOption};
use actix_web::{post, web, Error, HttpResponse, Responder};
use actix_web::error::{ErrorBadRequest, ErrorInternalServerError, ErrorNotFound};
use chrono::{DateTime, FixedOffset};
use log::{error, info};
use reqwest::header::COOKIE;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, DbErr, EntityTrait, TransactionTrait};
use uuid::Uuid;
use crate::requests::geo_login;

pub async fn get_comp_team(
    team_id: &String,
    player_id1: &String,
    player_id2: &String,
    db: &DatabaseConnection,
    team_progress: &RankedTeamDuelsProgress,
    client: &reqwest::Client
) -> Result<Result<CompTeamModel, CompTeamModel>, Error> {
    let team_result = CompTeam::find_by_id(team_id)
        .one(db)
        .await;

    if let Ok(team_option) = team_result {
        let ranked_team_request_url = format!(
            "https://www.geoguessr.com/api/v4/ranked-team-duels/teams/?userId={}&userId={}",
            player_id1, player_id2
        );

        let team_response = client.get(ranked_team_request_url)
            .send()
            .await
            .map_err(|_| ErrorInternalServerError("Fetch Comp Team operation failed!"))?
            .json::<RankedTeam>()
            .await
            .map_err(|_| ErrorInternalServerError("Could not pass Json to Ranked Team"))?;

        let team = CompTeamModel {
            team_id: ActiveValue::Set(team_id.clone()),
            player_id1: ActiveValue::Set(player_id1.clone()),
            player_id2: ActiveValue::Set(player_id2.clone()),
            name: ActiveValue::Set(team_response.team_name),
            rating: ActiveValue::Set(team_progress.rating_after)
        };
        
        if team_option.is_some() {
            Ok(Ok(team))
        } else {
            Ok(Err(team))
        }
    } else {
        Err(ErrorInternalServerError("Database operation get_comp_team failed!"))
    }
}

async fn insert_fun_team(
    team_id: String,
    player_ids: Vec<String>,
    db: &DatabaseConnection,
) -> Result<Option<FunTeamModel>, Error> {
    let team_result = FunTeam::find_by_id(team_id.clone())
        .one(db)
        .await;

    if let Ok(team_option) = team_result {
        if team_option.is_some() {
            return Ok(None);
        }

        let team = FunTeamModel {
            team_id: ActiveValue::Set(team_id),
            player_ids: ActiveValue::Set(player_ids)
        };

        Ok(Some(team))
    } else {
        Err(ErrorInternalServerError("Database operation get_fun_team failed!"))
    }
}

async fn insert_fun_team_duels_game_model(
    game: &crate::geo_guessr::DuelsGame,
    game_mode: TeamGameMode,
    geo_mode: GeoMode,
    db: &DatabaseConnection,
) -> Result<(DuelsGameModel, Vec<FunTeamModel>), Error> {
    let team_id1 = game.teams[0].id.clone();
    let team_id2 = game.teams[1].id.clone();
    let mut teams = Vec::new();

    if let Some(team) = insert_fun_team(
        team_id1.clone(),
        game.teams[0].players.iter().map(|player| player.player_id.clone()).collect(),
        db
    ).await? {
        teams.push(team);
    }

    if let Some(team) = insert_fun_team(
        team_id2.clone(),
        game.teams[1].players.iter().map(|player| player.player_id.clone()).collect(),
        db
    ).await? {
        teams.push(team);
    }

    let game_model = get_team_duels_game_model(
        game,
        game_mode,
        geo_mode,
        team_id1,
        team_id2,
        None,
        None
    );

    Ok((game_model, teams))
}

async fn insert_comp_team_duels_game_model(
    game: &crate::geo_guessr::DuelsGame,
    game_mode: TeamGameMode,
    geo_mode: GeoMode,
    db: &DatabaseConnection,
    client: &reqwest::Client
) -> Result<(DuelsGameModel, Vec<CompTeamModel>, Vec<CompTeamModel>), Error> {
    let team_id1 = game.teams[0].id.clone();
    let team_id2 = game.teams[1].id.clone();
    let mut update_teams = Vec::new();
    let mut insert_teams = Vec::new();
    
    match get_comp_team(
        &team_id1,
        &game.teams[0].players[0].player_id,
        &game.teams[0].players[1].player_id,
        db,
        game.teams[0].players[0].progress_change.as_ref().unwrap().ranked_team_duels_progress.as_ref().unwrap(),
        client
    ).await? {
        Ok(update_team) => update_teams.push(update_team),
        Err(insert_team) => insert_teams.push(insert_team)
    }

    match get_comp_team(
        &team_id2,
        &game.teams[1].players[0].player_id,
        &game.teams[1].players[1].player_id,
        db,
        game.teams[1].players[0].progress_change.as_ref().unwrap().ranked_team_duels_progress.as_ref().unwrap(),
        client
    ).await? {
        Ok(update_team) => update_teams.push(update_team),
        Err(insert_team) => insert_teams.push(insert_team)
    }
    
    let mut rating_before_team1 = None;
    let mut rating_before_team2 = None;
    
    if let (Some(progress1), Some(progress2)) = 
        (&game.teams[1].players[0].progress_change, &game.teams[0].players[0].progress_change) 
    {
        if let (Some(ranked_team_progress1), Some(ranked_team_progress2)) = 
            (&progress1.ranked_team_duels_progress, &progress2.ranked_team_duels_progress) {
            rating_before_team1 = ranked_team_progress1.rating_before;
            rating_before_team2 = ranked_team_progress2.rating_before;
        }
    }
    
    let game_model = get_team_duels_game_model(
        game,
        game_mode,
        geo_mode,
        team_id1,
        team_id2,
        rating_before_team1,
        rating_before_team2
    );

    Ok((game_model, update_teams, insert_teams))
}

fn get_team_duels_game_model(
    game: &crate::geo_guessr::DuelsGame,
    game_mode: TeamGameMode,
    geo_mode: GeoMode,
    team_id1: String,
    team_id2: String,
    rating_before_team1: Option<i32>,
    rating_before_team2: Option<i32>,
) -> DuelsGameModel {
    DuelsGameModel {
        id: ActiveValue::Set(game.game_id.clone()),
        team_id1: ActiveValue::Set(team_id1),
        team_id2: ActiveValue::Set(team_id2),
        health_team1: ActiveValue::Set(game.teams[0].health),
        health_team2: ActiveValue::Set(game.teams[1].health),
        team_game_mode: ActiveValue::Set(game_mode.to_string()),
        geo_mode: ActiveValue::Set(geo_mode.to_string()),
        start_time: ActiveValue::Set(game.rounds[0].start_time.clone().unwrap()),
        map_id: ActiveValue::Set(game.options.map.slug.clone()),
        rating_before_team1: ActiveValue::Set(rating_before_team1),
        rating_before_team2: ActiveValue::Set(rating_before_team2),
    }
}

async fn get_duels_game_model(
    game: &crate::geo_guessr::DuelsGame,
    game_mode: TeamGameMode,
    geo_mode: GeoMode
) -> DuelsGameModel {
    let players_team1 = &game.teams[0].players;
    let players_team2 = &game.teams[1].players;

    let mut rating_before_team1 = None;
    let mut rating_before_team2 = None;

    if let (Some(progress1), Some(progress2)) =
        (&game.teams[1].players[0].progress_change, &game.teams[0].players[0].progress_change)
    {
        if let (Some(ranked_team_progress1), Some(ranked_team_progress2)) =
            (&progress1.ranked_system_progress, &progress2.ranked_system_progress) {
            rating_before_team1 = ranked_team_progress1.rating_before;
            rating_before_team2 = ranked_team_progress2.rating_before;
        }
    }

    DuelsGameModel {
        id: ActiveValue::Set(game.game_id.clone()),
        team_id1: ActiveValue::Set(players_team1[0].player_id.clone()),
        team_id2: ActiveValue::Set(players_team2[0].player_id.clone()),
        health_team1: ActiveValue::Set(game.teams[0].health),
        health_team2: ActiveValue::Set(game.teams[1].health),
        team_game_mode: ActiveValue::Set(game_mode.to_string()),
        geo_mode: ActiveValue::Set(geo_mode.to_string()),
        start_time: ActiveValue::Set(game.rounds[0].start_time.clone().unwrap()),
        map_id: ActiveValue::Set(game.options.map.slug.clone()),
        rating_before_team1: ActiveValue::Set(rating_before_team1),
        rating_before_team2: ActiveValue::Set(rating_before_team2),
    }
}

fn get_game_mode(team1_size: usize, team2_size: usize, is_rated: bool) -> TeamGameMode {
    if team1_size == 1 && team2_size == 1 {
        if is_rated {
            TeamGameMode::DuelsRanked
        } else {
            TeamGameMode::Duels
        }
    } else if team1_size == 2 && team2_size == 2 {
        if is_rated {
            TeamGameMode::TeamDuelsRanked
        } else {
            TeamGameMode::TeamDuels
        }
    } else {
        TeamGameMode::TeamFun
    }
}

fn get_geo_mode(movement_options: &MovementOption) -> GeoMode {
    match movement_options {
        MovementOption {
            forbid_moving: false,
            forbid_zooming: false,
            forbid_rotating: false,
        } => Moving,
        MovementOption {
            forbid_moving: false,
            forbid_zooming: false,
            forbid_rotating: true,
        } => NoPanning,
        MovementOption {
            forbid_moving: false,
            forbid_zooming: true,
            forbid_rotating: false,
        } => NoZooming,
        MovementOption {
            forbid_moving: false,
            forbid_zooming: true,
            forbid_rotating: true,
        } => NoPanningZooming,
        MovementOption {
            forbid_moving: true,
            forbid_zooming: false,
            forbid_rotating: false,
        } => NoMove,
        MovementOption {
            forbid_moving: true,
            forbid_zooming: false,
            forbid_rotating: true,
        } => NoPanningMoving,
        MovementOption {
            forbid_moving: true,
            forbid_zooming: true,
            forbid_rotating: false,
        } => NoMovingZooming,
        MovementOption {
            forbid_moving: true,
            forbid_zooming: true,
            forbid_rotating: true
        } => NMPZ
    }
}

async fn get_player_model(player_id: &str, db: &DatabaseConnection, client: &reqwest::Client) -> Result<Result<PlayerModel, PlayerModel>, Error> {
    if let Ok(player_option) = Player::find_by_id(player_id).one(db).await {
        let player_response = client.get(format!("https://www.geoguessr.com/api/v3/users/{}", player_id))
            .send()
            .await
            .map_err(|_| ErrorInternalServerError("Fetch User operation failed!"))?
            .json::<User>()
            .await
            .map_err(|_| ErrorNotFound(format!("User with id {} could not be found!", player_id)))?;

        let player_ratings_option = client.get(format!("https://www.geoguessr.com/api/v4/ranked-system/progress/{}", player_id))
            .send()
            .await
            .map_err(|_| ErrorInternalServerError(format!("Fetch Player Ratings operation failed for player {}!", player_id)))?
            .json::<PlayerRankedSystemProgress>()
            .await
            .ok();

        let player_rating;
        let game_mode_ratings;

        if player_ratings_option.is_none() {
            player_rating = None;

            game_mode_ratings = GameModeRatings {
                standard_duels: None,
                no_move_duels: None,
                nmpz_duels: None
            }
        } else {
            let player_ratings = player_ratings_option.unwrap();
            player_rating = player_ratings.rating;

            if let Some(game_mode_ratings_result) = player_ratings.game_mode_ratings {
                game_mode_ratings = game_mode_ratings_result;
            } else {
                game_mode_ratings = GameModeRatings {
                    standard_duels: None,
                    no_move_duels: None,
                    nmpz_duels: None
                }
            }
        }

        let player = PlayerModel {
            id: ActiveValue::Set(player_response.id),
            name: ActiveValue::Set(player_response.nick),
            country_code: ActiveValue::Set(player_response.country_code),
            rating: ActiveValue::Set(player_rating),
            moving_rating: ActiveValue::Set(game_mode_ratings.standard_duels),
            no_move_rating: ActiveValue::Set(game_mode_ratings.no_move_duels),
            nmpz_rating: ActiveValue::Set(game_mode_ratings.nmpz_duels),
            avatar_pin: ActiveValue::Set(player_response.pin.url),
            level: ActiveValue::Set(player_response.br.level),
            is_pro_user: ActiveValue::Set(player_response.is_pro_user),
            is_creator: ActiveValue::Set(player_response.is_creator)
        };

        if player_option.is_some() {
            Ok(Ok(player))
        } else {
            Ok(Err(player))
        }
    } else {
        Err(ErrorInternalServerError("Database operation get_player failed!"))
    }
}

#[post("/duels-game/{game_id}")]
async fn insert_duels_game(
    path: web::Path<String>,
    db: web::Data<DatabaseConnection>,
) -> Result<impl Responder, Error> {
    let client = reqwest::Client::new();
    let cookies = geo_login::get_cookies().await;
    let game_id = path.into_inner();
    
    let game = client.get(format!("https://game-server.geoguessr.com/api/duels/{}", game_id))
        .header(COOKIE, cookies)
        .send()
        .await
        .map_err(|_| ErrorInternalServerError("Fetch Game operation failed!"))?
        .json::<crate::geo_guessr::DuelsGame>()
        .await
        .map_err(|_| ErrorBadRequest(format!("Could not find Game with id: {}!", game_id)))?;

    if game.status.as_str() != "Finished" {
        return Err(ErrorBadRequest("Game has not finished yet!"));
    }

    let db = db.get_ref();
    let game_mode = get_game_mode(game.teams[0].players.len(), game.teams[1].players.len(), game.options.is_rated);
    let geo_mode = get_geo_mode(&game.options.movement_options);

    let mut rounds = Vec::with_capacity(game.current_round_number as usize);
    let mut guesses = Vec::new();
    let mut locations = Vec::new();
    let mut insert_players = Vec::new();
    let mut update_players = Vec::new();
    let mut insert_comp_teams = Vec::new();
    let mut update_comp_teams = Vec::new();
    let mut insert_fun_teams = Vec::new();

    for team in game.teams.iter() {
        for player in team.players.iter() {
            match get_player_model(&player.player_id, db, &client).await {
                Ok(player_option) => {
                    match player_option {
                        Ok(update_player) => update_players.push(update_player),
                        Err(insert_player) => insert_players.push(insert_player)
                    }
                },
                Err(internal_server_error) => return Err(internal_server_error)
            }
        }
    }

    for (round_number, round) in game
        .rounds
        .iter()
        .filter(|round| round.round_number <= game.current_round_number)
        .enumerate()
    {
        let panorama = &round.panorama;

        let location = LocationModel {
            id: ActiveValue::Set(panorama.pano_id.clone()),
            lat: ActiveValue::Set(panorama.lat),
            lng: ActiveValue::Set(panorama.lng),
            heading: ActiveValue::Set(panorama.heading),
            pitch: ActiveValue::Set(panorama.pitch),
            zoom: ActiveValue::Set(panorama.zoom),
            country_code: ActiveValue::Set(panorama.country_code.clone()),
        };

        locations.push(location);

        let guess_ids: Vec<String> = std::iter::repeat_with(|| Uuid::new_v4().to_string())
            .take(game.teams.len())
            .collect();

        for (index, team) in game.teams.iter().enumerate() {
            let round_results_option = team.round_results.iter().find(|round| {
                round
                    .best_guess
                    .as_ref()
                    .is_some_and(|guess| guess.round_number - 1 == round_number as i32)
            });

            if let Some(round_result) = round_results_option {
                if let Some(best_guess) = &round_result.best_guess {
                    let start_time_option = &round.start_time;
                    let round_starting_date: DateTime<FixedOffset> =
                        start_time_option.clone().unwrap().parse().unwrap();
                    let guess_date: DateTime<FixedOffset> = best_guess.created.parse().unwrap();

                    let guess = GuessModel {
                        id: ActiveValue::Set(guess_ids[index].clone()),
                        lat: ActiveValue::Set(best_guess.lat),
                        lng: ActiveValue::Set(best_guess.lng),
                        score: ActiveValue::Set(best_guess.score.unwrap()),
                        time: ActiveValue::Set(Some((guess_date - round_starting_date).num_seconds() as i32)),
                        distance: ActiveValue::Set(best_guess.distance),
                        round_country_code: ActiveValue::Set(round.panorama.country_code.clone()),
                    };

                    guesses.push(guess);
                }
            }
        }

        let round = DuelsRoundModel {
            id: ActiveValue::Set(Uuid::new_v4().to_string()),
            game_id: ActiveValue::Set(game.game_id.clone()),
            location_id: ActiveValue::Set(panorama.pano_id.clone()),
            guess_id_team1: ActiveValue::Set(guess_ids[0].clone()),
            guess_id_team2: ActiveValue::Set(guess_ids[1].clone()),
            round_number: ActiveValue::Set(round_number as i32),
            damage_multiplier: ActiveValue::Set(round.damage_multiplier),
        };

        rounds.push(round);
    }

    let map = MapModel {
        id: ActiveValue::Set(game.options.map.slug.clone()),
        name: ActiveValue::Set(game.options.map.name.clone()),
        lat1: ActiveValue::Set(game.options.map.bounds.min.lat),
        lng1: ActiveValue::Set(game.options.map.bounds.min.lng),
        lat2: ActiveValue::Set(game.options.map.bounds.max.lat),
        lng2: ActiveValue::Set(game.options.map.bounds.max.lng)
    };

    let duels_game;
    
    match game_mode {
        TeamGameMode::Duels | TeamGameMode::DuelsRanked => {
            duels_game = get_duels_game_model(&game, game_mode, geo_mode).await;
        },
        TeamGameMode::TeamDuelsRanked => {
            match insert_comp_team_duels_game_model(&game, game_mode, geo_mode, db, &client).await {
                Ok((duels_game_model, update_team_models, insert_team_models)) => {
                    duels_game = duels_game_model;
                    update_comp_teams = update_team_models;
                    insert_comp_teams = insert_team_models;
                },
                Err(error) => return Err(error)
            };
        },
        TeamGameMode::TeamDuels | TeamGameMode::TeamFun => {
            match insert_fun_team_duels_game_model(&game, game_mode, geo_mode, db).await {
                Ok((duels_game_model, team_models)) => {
                    duels_game = duels_game_model;
                    insert_fun_teams = team_models;
                },
                Err(error) => return Err(error)
            }
        }
    }

    match db.transaction::<_, _, DbErr>(|txn| {
        Box::pin(async move {
            DuelsGame::insert(duels_game).exec(txn).await?;
            Guess::insert_many(guesses).exec(txn).await?;
            DuelsRound::insert_many(rounds).exec(txn).await?;

            for location in locations {
                Location::insert(location).exec(txn).await?;
            }
            if !insert_players.is_empty() {
                Player::insert_many(insert_players).exec(txn).await?;
            }
            for player in update_players {
                player.update(txn).await?;
            }
            if !insert_comp_teams.is_empty() {
                CompTeam::insert_many(insert_comp_teams).exec(txn).await?;
            }
            for team in update_comp_teams {
                team.update(txn).await?;
            }
            if !insert_fun_teams.is_empty() {
                FunTeam::insert_many(insert_fun_teams).exec(txn).await?;
            }

            Ok(())
        })
    }).await {
        Ok(()) => info!("All inserts succeeded"),
        Err(err) => {
            error!("Insertion failed, Rolling back: {}", err);

            return if err.to_string().contains("duplicate key value violates unique constraint") {
                Err(ErrorBadRequest(format!("Game with id {} does already exist!", game.game_id)))
            } else {
                Err(ErrorInternalServerError(err.to_string()))
            }
        }
    }
    
    let _ = Map::insert(map).exec(db).await;
    
    Ok(HttpResponse::Created().body(""))
}

#[post("/solo-game/{game_id}")]
pub async fn insert_solo_game(
    path: web::Path<String>,
    db: web::Data<DatabaseConnection>,
) -> Result<impl Responder, Error> {
    let client = reqwest::Client::new();
    let game_id = path.into_inner();

    let game = client.get(format!("https://www.geoguessr.com/api/v3/games/{}", game_id))
        .send()
        .await
        .map_err(|_| ErrorInternalServerError("Fetch Game operation failed!"))?
        .json::<crate::geo_guessr::SoloGame>()
        .await
        .map_err(|_| ErrorBadRequest(format!("Could not find Game with id: {}!", game_id)))?;
    
    if game.state.as_str() != "finished" {
        return Err(ErrorBadRequest("Game has not finished yet!"));
    }
    
    let db = db.get_ref();
    let geo_mode = get_geo_mode(&MovementOption {
        forbid_moving: game.forbid_moving,
        forbid_zooming: game.forbid_zooming,
        forbid_rotating: game.forbid_rotating,
    });
    
    let mut insert_player = None;
    let mut rounds = Vec::with_capacity(game.round as usize);
    let mut guesses = Vec::with_capacity(game.round as usize);
    let mut locations = Vec::with_capacity(game.round as usize);
    
    match get_player_model(&game.player.id, db, &client).await {
        Ok(player_option) => {
            if let Err(player_model) = player_option {
                insert_player = Some(player_model);
            }
        },
        Err(internal_server_error) => return Err(internal_server_error)
    }
    
    for (round_number, round) in game.rounds.iter().enumerate() {
        let location = LocationModel {
            id: ActiveValue::Set(round.pano_id.clone()),
            lat: ActiveValue::Set(round.lat),
            lng: ActiveValue::Set(round.lng),
            heading: ActiveValue::Set(round.heading),
            pitch: ActiveValue::Set(round.pitch),
            zoom: ActiveValue::Set(round.zoom),
            country_code: ActiveValue::Set(round.streak_location_code.clone())
        };

        locations.push(location);
        
        let guess_id = Uuid::new_v4().to_string();
        
        let guess = GuessModel {
            id: ActiveValue::Set(guess_id.clone()),
            lat: ActiveValue::Set(game.player.guesses[round_number].lat),
            lng: ActiveValue::Set(game.player.guesses[round_number].lng),
            score: ActiveValue::Set(game.player.guesses[round_number].round_score_in_points),
            time: ActiveValue::NotSet,
            distance: ActiveValue::Set(game.player.guesses[round_number].distance_in_meters),
            round_country_code: ActiveValue::Set(round.streak_location_code.clone()),
        };
        
        guesses.push(guess);
        
        let round = SoloRoundModel {
            id: ActiveValue::Set(Uuid::new_v4().to_string()),
            game_id: ActiveValue::Set(game.token.clone()),
            guess_id: ActiveValue::Set(guess_id),
            location_id: ActiveValue::Set(round.pano_id.clone()),
            round_number: ActiveValue::Set(round_number as i32),
        };
        
        rounds.push(round);
    }
    
    let solo_game = SoloGameModel {
        id: ActiveValue::Set(game.token.clone()),
        player_id: ActiveValue::Set(game.player.id.clone()),
        geo_mode: ActiveValue::Set(geo_mode.to_string()),
        start_time: ActiveValue::Set(game.rounds[0].start_time.clone().unwrap()),
        map_id: ActiveValue::Set(game.map.clone())
    };

    let map = MapModel {
        id: ActiveValue::Set(game.map.clone()),
        name: ActiveValue::Set(game.map_name.clone()),
        lat1: ActiveValue::Set(game.bounds.min.lat),
        lng1: ActiveValue::Set(game.bounds.min.lng),
        lat2: ActiveValue::Set(game.bounds.max.lat),
        lng2: ActiveValue::Set(game.bounds.max.lng)
    };
    
    match db.transaction::<_, _, DbErr>(|txn| {
        Box::pin(async move {
            SoloGame::insert(solo_game).exec(txn).await?;
            SoloRound::insert_many(rounds).exec(txn).await?;
            Guess::insert_many(guesses).exec(txn).await?;
            
            for location in locations {
                Location::insert(location).exec(txn).await?;
            }
            if let Some(player) = insert_player {
                Player::insert(player).exec(txn).await?;
            }
            
            Ok(())
        })
    }).await {
        Ok(()) => info!("All inserts succeeded"),
        Err(err) => {
            error!("Insertion failed, Rolling back: {}", err);

            return if err.to_string().contains("duplicate key value violates unique constraint") {
                Err(ErrorBadRequest(format!("Game with id {} does already exist!", game_id)))
            } else {
                Err(ErrorInternalServerError(err.to_string()))
            }
        }
    }

    let _ = Map::insert(map).exec(db).await;

    Ok(HttpResponse::Created().body(""))
}
