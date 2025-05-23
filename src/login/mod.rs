use crate::entities::prelude::{Session, User, Player};
use crate::entities::user::Model as UserModel;
use crate::entities::player::Model as PlayerModel;
use actix_web::error::{ErrorBadRequest, ErrorConflict, ErrorGone, ErrorInternalServerError};
use actix_web::Error;
use chrono::{DateTime, Utc};
use sea_orm::{DatabaseConnection, EntityTrait, ModelTrait};

pub mod login_request;
mod email;

pub async fn get_player_id_from_session(session_id: &str, db: &DatabaseConnection) -> Result<String, Error> {
    match get_user_from_session(session_id, db).await?.player_id {
        Some(player_id) => Ok(player_id),
        None => Err(ErrorConflict("Account is not linked yet!"))
    }
}

pub async fn get_user_from_session(session_id: &str, db: &DatabaseConnection) -> Result<UserModel, Error> {
    match Session::find_by_id(session_id).find_also_related(User).one(db).await {
        Ok(session_option) => {
            match session_option {
                Some((session, user_option)) => {
                    if Utc::now() > session.expire_date.parse::<DateTime<Utc>>().unwrap() {
                        let _ = session.delete(db).await;
                        return Err(ErrorGone("Session expired!"));
                    }

                    match user_option {
                        Some(user) => {
                            Ok(user)
                        },
                        None => Err(ErrorInternalServerError("Can not find user from sessionId"))
                    }
                },
                None => Err(ErrorBadRequest("Session does not exist!"))
            }
        },
        Err(err) => Err(ErrorInternalServerError(err.to_string()))
    }
}

pub async fn get_player_from_session(session_id: &str, db: &DatabaseConnection) -> Result<PlayerModel, Error> {
    match Session::find_by_id(session_id).find_also_related(User).and_also_related(Player).one(db).await {
        Ok(session_option) => {
            match session_option {
                Some((session, _, player_option)) => {
                    if Utc::now() > session.expire_date.parse::<DateTime<Utc>>().unwrap() {
                        let _ = session.delete(db).await;
                        return Err(ErrorGone("Session expired!"));
                    }

                    match player_option {
                        Some(player) => {
                            Ok(player)
                        },
                        None => Err(ErrorConflict("Account is not linked!"))
                    }
                },
                None => Err(ErrorBadRequest("Session does not exist!"))
            }
        },
        Err(err) => Err(ErrorInternalServerError(err.to_string()))
    }
}