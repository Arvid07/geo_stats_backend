use chrono::{DateTime, Duration, Utc};
use lazy_static::lazy_static;
use reqwest::Client;
use serde::Serialize;
use std::error::Error;
use std::fmt::Write as FmtWrite;
use std::ops::Add;
use tokio::sync::Mutex;

lazy_static! {
    static ref COOKIES: Mutex<String> = Mutex::new(String::new());
    static ref COOKIE_EXPIRE_DATE: Mutex<DateTime<Utc>> = Mutex::new(Utc::now());
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GuestRequest {
    nick: String
}

pub async fn get_cookies() -> String {
    let mut expire = COOKIE_EXPIRE_DATE.lock().await;

    if *expire < Utc::now() + Duration::seconds(15) {
        let new_expire = login().await.expect("Geo login failed!");
        *expire = new_expire;
    }
    
    drop(expire);

    let cookies = COOKIES.lock().await;
    cookies.clone()
}

async fn login() -> Result<DateTime<Utc>, Box<dyn Error>> {
    let client = Client::builder().cookie_store(true).build()?;

    let request_body = GuestRequest {
        nick: String::from("geo_stats")
    };

    let response = client
        .post("https://www.geoguessr.com/api/v4/guest-users")
        .json(&request_body)
        .send()
        .await?;

    let mut cookies = COOKIES.lock().await;
    *cookies = response.cookies().fold(String::new(), |mut acc, cookie| {
        write!(&mut acc, "{}={}; ", cookie.name(), cookie.value()).unwrap();
        acc
    });

    let mut first_expire_option = None;

    for cookie in response.cookies() {
        if let Some(time) = cookie.expires() {
            let cookie_expire = DateTime::<Utc>::from(time);

            if let Some(first_expire) = first_expire_option {
                if cookie_expire < first_expire {
                    first_expire_option = Some(cookie_expire);
                }
            } else {
                first_expire_option = Some(cookie_expire);
            }
        }
    }

    let expire = if let Some(first_expire) = first_expire_option {
        first_expire
    } else {
        Utc::now().add(Duration::new(50_000, 0).unwrap())
    };

    Ok(expire)
}
