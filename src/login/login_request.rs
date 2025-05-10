use crate::entities::prelude::{Player, Session, User};
use crate::entities::player::{ActiveModel as PlayerModel};
use crate::entities::session::ActiveModel as SessionModel;
use crate::entities::user::ActiveModel as UserModel;
use crate::entities::{user};
use crate::login::email::send_verify_email;
use actix_web::error::{ErrorBadRequest, ErrorConflict, ErrorGone, ErrorInternalServerError, ErrorNotFound, ErrorUnauthorized};
use actix_web::{post, web, Error, HttpRequest, HttpResponse, Responder};
use chrono::{DateTime, Duration, TimeDelta, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use ring::rand::{SecureRandom, SystemRandom};
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use actix_web::cookie::{time, Cookie, SameSite};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sea_orm::prelude::Expr;
use tokio::sync::Mutex;
use uuid::Uuid;
use crate::geo_guessr::{GameModeRatings, PlayerRankedSystemProgress};

const SESSION_EXPIRE: TimeDelta = Duration::days(30);
const SESSION_EXPIRE_DURATION: time::Duration = time::Duration::days(30);
const VERIFICATION_CODE_EXPIRE: TimeDelta = Duration::minutes(5);

lazy_static! {
    static ref UNREGISTERED_USERS: Mutex<HashMap<String, UnregisteredUser>> = Mutex::new(HashMap::new());
}

struct UnregisteredUser {
    user_id: String,
    salt: String,
    salted_password_hash: String,
    verification_code: String,
    verification_code_expire: DateTime<Utc>
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserLoginRequest {
    email: String,
    password: String
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserVerifyEmailRequest {
    email: String,
    verification_code: String
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserVerifyEmailResponse {
    verification_code_expire: DateTime<Utc>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserLoginResponse {
    session_expire: String
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserLinkAccountRequest {
    player_id: String
}

async fn create_new_session(user_id: String, db: &DatabaseConnection) -> Result<String, Error> {
    let session_id = Uuid::new_v4().to_string();
    
    let session = SessionModel {
        id: ActiveValue::Set(session_id.clone()),
        user_id: ActiveValue::Set(user_id),
        expire_date: ActiveValue::Set((Utc::now() + SESSION_EXPIRE).to_string())
    };
    
    match Session::insert(session).exec(db).await {
        Ok(_) => Ok(session_id),
        Err(err) => Err(ErrorInternalServerError(err))
    }
}

fn generate_salt() -> String {
    let rng = SystemRandom::new();
    let mut salt = vec![0u8, 32];
    rng.fill(&mut salt).expect("Randomness failed!");
    
    STANDARD.encode(&salt)
}

fn get_salted_password_hash(password: &String, salt: &String) -> String {
    let mut hasher = DefaultHasher::new();
    format!("{}{}", password, salt).hash(&mut hasher);
    
    hasher.finish().to_string()
}

fn generate_6_digit_code() -> String {
    let rng = SystemRandom::new();
    let mut buf = [0u8; 4];
    rng.fill(&mut buf).expect("Randomness failed!");
    let v = u32::from_be_bytes(buf) % 1_000_000;
    format!("{:06}", v)
}

fn is_valid_email(email: &str) -> bool {
    let re = Regex::new(r"^[^@]+@[^@.]+\..+$").unwrap();
    re.is_match(email)
}

async fn insert_player_model(player_id: &str, db: &DatabaseConnection, client: &reqwest::Client) -> Result<(), Error> {
    if let Ok(player_option) = Player::find_by_id(player_id).one(db).await {
        if player_option.is_some() {
            return Ok(());
        }
        
        let player_response = client.get(format!("https://www.geoguessr.com/api/v3/users/{}", player_id))
            .send()
            .await
            .map_err(|_| ErrorInternalServerError("Fetch User operation failed!"))?
            .json::<crate::geo_guessr::User>()
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
        
        match player_ratings_option {
            Some(player_ratings) => {
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
            },
            None => {
                player_rating = None;

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
        
        if let Err(err) = Player::insert(player).exec(db).await {
            return Err(ErrorInternalServerError(err));
        }
        
        Ok(())
    } else {
        Err(ErrorInternalServerError("Database operation get_player failed!"))
    }
}

#[post("/login")]
async fn user_login(
    db: web::Data<DatabaseConnection>,
    request: web::Json<UserLoginRequest>,
    http_request: HttpRequest
) -> Result<impl Responder, Error> {
    let db = db.get_ref();
    
    let user = match User::find().filter(user::Column::Email.eq(&request.email)).one(db).await {
        Ok(user_option) => {
            match user_option {
                Some(user) => user,
                None => return Err(ErrorNotFound("User not found!"))
            }
        },
        Err(err) => return Err(ErrorInternalServerError(err.to_string()))
    };
    
    let salted_password_hash = get_salted_password_hash(&request.password, &user.salt);
    
    if salted_password_hash != user.salted_password_hash {
        return Err(ErrorBadRequest("Incorrect password!"));
    }

    if let Some(session_cookie) = http_request.cookie("sessionId") {
        let _ = Session::delete_by_id(session_cookie.value()).exec(db).await;
    };

    let session_id = create_new_session(user.id, db).await?;
    
    let cookie = Cookie::build("sessionId", session_id)
        .path("/")
        .http_only(true)
        .secure(cfg!(not(debug_assertions)))
        .same_site(SameSite::Strict)
        .max_age(SESSION_EXPIRE_DURATION)
        .finish();

    Ok(HttpResponse::Ok().cookie(cookie).json(UserLoginResponse { session_expire: SESSION_EXPIRE.to_string() }))
}

#[post("/signup")]
async fn user_signup(
    db: web::Data<DatabaseConnection>,
    request: web::Json<UserLoginRequest>,
    http_request: HttpRequest
) -> Result<impl Responder, Error> {
    let db = db.get_ref();
    
    if !is_valid_email(&request.email) {
        return Err(ErrorBadRequest("Invalid Email!"));
    }

    match User::find().filter(user::Column::Email.eq(&request.email)).one(db).await {
        Ok(user_option) => {
            if user_option.is_some() {
                return Err(ErrorConflict(format!("The email {} is already registered!", request.email)));
            }
        },
        Err(err) => return Err(ErrorInternalServerError(err.to_string()))
    };

    let user_id = Uuid::new_v4().to_string();
    let salt = generate_salt();
    let salted_password_hash = get_salted_password_hash(&request.password, &salt);
    let verification_code = generate_6_digit_code();
    let verification_code_expire = Utc::now() + VERIFICATION_CODE_EXPIRE;

    if let Err(err) = send_verify_email(&verification_code, &request.email).await {
        return Err(ErrorInternalServerError(err));
    }

    if let Some(session_cookie) = http_request.cookie("sessionId") {
        let _ = Session::delete_by_id(session_cookie.value()).exec(db).await;
    };

    let unregistered_user = UnregisteredUser {
        user_id,
        salt,
        salted_password_hash,
        verification_code,
        verification_code_expire
    };
    
    let mut unregistered_users = UNREGISTERED_USERS.lock().await;
    unregistered_users.insert(request.email.clone(), unregistered_user);
    
    let response = UserVerifyEmailResponse {
        verification_code_expire
    };
    
    Ok(HttpResponse::Ok().json(response))
}

#[post("/verify-email")]
async fn verify_email(
    db: web::Data<DatabaseConnection>,
    request: web::Json<UserVerifyEmailRequest>
) -> Result<impl Responder, Error> {
    let mut unregistered_users = UNREGISTERED_USERS.lock().await;

    let user = match unregistered_users.remove(&request.email) {
        Some(unregistered_user) => {
            if Utc::now() > unregistered_user.verification_code_expire {
                return Err(ErrorGone("Verification code expired. You have to sign up again!"))
            }
            
            if unregistered_user.verification_code != request.verification_code {
                return Err(ErrorBadRequest("Incorrect verification code. You have to sign up again!"))
            } 
            unregistered_user
        }
        None => return Err(ErrorNotFound("Email not found!"))
    };
    
    drop(unregistered_users);
    
    let db = db.get_ref();
    let user_id = user.user_id;
    
    let user = UserModel {
        id: ActiveValue::Set(user_id.clone()),
        email: ActiveValue::Set(request.email.clone()),
        salt: ActiveValue::Set(user.salt),
        salted_password_hash: ActiveValue::Set(user.salted_password_hash),
        player_id: ActiveValue::NotSet,
    };
    
    match User::insert(user).exec(db).await {
        Ok(_) => {
            let session_id = create_new_session(user_id, db).await?;
            let cookie = Cookie::build("sessionId", session_id)
                .path("/")
                .http_only(true)
                .secure(cfg!(not(debug_assertions)))
                .same_site(SameSite::Strict)
                .max_age(SESSION_EXPIRE_DURATION)
                .finish();

            Ok(HttpResponse::Created().cookie(cookie).json(UserLoginResponse { session_expire: SESSION_EXPIRE.to_string() }))
        },
        Err(err) => Err(ErrorInternalServerError(err.to_string()))
    }
}

#[post("/link-account")]
async fn link_account(
    db: web::Data<DatabaseConnection>,
    request: web::Json<UserLinkAccountRequest>,
    http_request: HttpRequest
) -> Result<impl Responder, Error> {
    let session_id = match http_request.cookie("sessionId") {
        Some(cookie) => {
            String::from(cookie.value())
        },
        None => return Err(ErrorUnauthorized("Missing `sessionId` cookie!"))
    };

    let db = db.get_ref();

    let user_id = match Session::find_by_id(session_id).one(db).await {
        Ok(session_option) => {
            match session_option {
                Some(session) => {
                    if Utc::now() > session.expire_date.parse::<DateTime<Utc>>().unwrap() {
                        let _ = session.delete(db).await;
                        return Err(ErrorGone("Session expired!"));
                    }

                    session.user_id
                },
                None => return Err(ErrorBadRequest("Session does not exist!"))
            }
        },
        Err(err) => return Err(ErrorInternalServerError(err.to_string()))
    };
    
    insert_player_model(&request.player_id, db, &reqwest::Client::new()).await?;
    
    User::update_many()
        .col_expr(
            user::Column::PlayerId,
            Expr::value(&request.player_id)
        )
        .filter(user::Column::Id.eq(user_id))
        .exec(db)
        .await
        .map_err(ErrorInternalServerError)?;
    
    Ok(HttpResponse::Ok())    
}

#[post("/logout")]
async fn log_out(
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

    match Session::delete_by_id(&session_id).exec(db).await {
        Ok(response) => {
            if response.rows_affected == 0 {
                return Err(ErrorNotFound("sessionId not found!"))
            }
            Ok(HttpResponse::Ok())
        },
        Err(err) => Err(ErrorInternalServerError(err))
    }
}