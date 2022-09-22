use std::{
    collections::HashMap,
    convert::TryInto,
    error::Error,
    fmt::{self, Display},
    process::exit,
    sync::Arc,
};

use coc_rs::api::Client as CocClient;
use serde_json::Value;
use serenity::{
    client::bridge::gateway::ShardManager, framework::standard::CommandResult,
    model::prelude::Message, prelude::*,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task::JoinHandle;
lazy_static::lazy_static! {
    pub static ref SHOULD_LOG: bool = parse_args().log;
}

pub async fn get_coc_client(ctx: &Context) -> CocClient {
    let data = ctx.data.read().await;
    data.get::<CocClientContainer>()
        .expect("Expected coc client in TypeMap")
        .clone()
}

pub async fn get_user_message(ctx: &Context, id: u64) -> Option<(u8, String)> {
    let data = ctx.data.read().await;
    let user_messages = data
        .get::<UserMessageContainer>()
        .expect("Expected user messages in TypeMap");
    user_messages.get(&id).cloned()
}

pub async fn set_user_message(ctx: &Context, id: u64, message: (u8, String)) {
    let mut data = ctx.data.write().await;
    let user_messages = data
        .get_mut::<UserMessageContainer>()
        .expect("Expected user messages in TypeMap");
    user_messages.insert(id, message);
}

pub async fn get_discord_link_api(ctx: &Context) -> Arc<Mutex<String>> {
    let data = ctx.data.read().await;
    data.get::<DiscordLinkAPIContainer>()
        .expect("Expected discord link api in TypeMap")
        .clone()
}

pub async fn get_player_id(discord_id: u64, ctx: &Context) -> Result<String, Box<dyn Error + Send + Sync>> {
    let discord_link_api = get_discord_link_api(ctx).await;
    let discord_link_api: String = discord_link_api.lock().await.clone();
    writes("got link api token".to_string());
    let client = reqwest::Client::new();
    let mut player_id = client.get(format!("https://cocdiscord.link/links/{discord_id}"));
    player_id = player_id.bearer_auth(discord_link_api);
    let player_id = match player_id.send().await {
        Ok(player_id) => player_id,
        Err(e) => {
            writes(format!("Error getting player id: {}", e));
            match e.to_string() {
                e if e.contains("dns error: failed to lookup address information: Name or service not known") => {
                    Err("retry")?
                }
                _ => {
                    Err("non recoverable error")?
                }
            }
        }
    };
    writes("got player id".to_string());
    match player_id.status() {
        reqwest::StatusCode::OK => {
            let player_id = match player_id.text().await {
                Ok(player_id) => player_id,
                Err(e) => {
                    writes(format!("Error getting player id: {}", e));
                    return Err("non recoverable error")?;
                }
            };
            let player_id: Value = match serde_json::from_str(&player_id) {
                Ok(player_id) => player_id,
                Err(e) => {
                    writes(format!("Error parsing player id json: {}", e));
                    return Err("non recoverable error")?;
                }
            };
            let player_id = player_id.as_array().unwrap_to_err("non recoverable error")?[0]["playerTag"].as_str().unwrap_to_err("non recoverable error")?;
            Ok(player_id.to_string())
        }
        _ => {
            writes(format!("Error getting player id: {}", player_id.status()));
            Err("non recoverable error")?
        }
    }
}

pub struct CocClientContainer;

impl TypeMapKey for CocClientContainer {
    type Value = CocClient;
}

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

pub struct UserMessageContainer;

impl TypeMapKey for UserMessageContainer {
    type Value = HashMap<u64, (u8, String)>;
}

pub struct DiscordLinkAPIContainer;
impl TypeMapKey for DiscordLinkAPIContainer {
    type Value = Arc<Mutex<String>>;
}

pub async fn too_many_tries(msg: String, ctx: &Context, id: u64) -> bool {
    let data = get_user_message(ctx, id).await;
    let prev: (u8, String) = match data {
        Some(p) => p,
        None => {
            set_user_message(ctx, id, (0, msg)).await;
            return false;
        }
    };
    if *prev.1 == msg {
        set_user_message(ctx, id, (prev.0 + 1, msg)).await;
    } else {
        set_user_message(ctx, id, (0, msg)).await;
    }
    get_user_message(ctx, id).await.unwrap().0 > 3
}

pub async fn check_to_many_times(ctx: &Context, msg: &Message, cmd: String) -> CommandResult {
    if too_many_tries(cmd.clone(), ctx, msg.author.id.0).await {
        writes(format!(
            "Too many tries author:{}, command:{}",
            msg.author, cmd
        ));
        let times = get_user_message(ctx, msg.author.id.0).await.unwrap().0;
        msg.reply(
            &ctx.http,
            format!("Calm down you've done /{} {} times!", cmd, times),
        )
        .await?;
        return Err("Too many tries".into());
    };
    writes(format!("Not too many tries {}", cmd));
    Ok(())
}

pub fn decode_jwt_for_time_left(token: &str) -> Result<u64, Box<dyn Error + Send + Sync>> {
    let mut split_token = token.split('.').collect::<Vec<&str>>();
    split_token.pop();
    let split_token: [&str; 2] = match split_token.try_into() {
        Ok(token) => token,
        Err(t) => return Err(format!("invalid token cold not be parsed {:?}", t).as_str())?,
    };
    let mut split_token_string: [String; 2] = ["".to_string(), "".to_string()];
    for (i, token) in split_token.into_iter().enumerate() {
        let t = base64::decode_config(token, base64::URL_SAFE_NO_PAD).unwrap();
        split_token_string[i] = String::from_utf8(t).unwrap();
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let t = serde_json::from_str::<Value>(&split_token_string[1])?;
    let t: u64 = t.get("exp").unwrap().as_f64().unwrap() as u64;

    if t < now {
        return Err("Token expired")?;
    }

    Ok(t - now)
}

pub async fn check_link_api_update(
    key: &Arc<Mutex<String>>,
    username: String,
    password: String,
) -> JoinHandle<i32> {
    let keys = Arc::clone(key);
    tokio::spawn(async move {
        loop {
            let time_left = match decode_jwt_for_time_left(keys.lock().await.as_str()) {
                Ok(t) => t,
                Err(ref e) => {
                    let e = e.to_string();
                    match e.as_str() {
                        "Token expired" => {
                            writes("Token expired, getting new token".to_string());
                            0
                        }
                        _ => {
                            writes(format!("Error decoding jwt {}", e));
                            0
                        }
                    }
                }
            };
            writes(format!("Time left in seconds: {}", time_left));
            tokio::time::sleep(tokio::time::Duration::from_millis(time_left * 1000)).await;
            writes("Updating token".to_string());
            let temp = match get_new_link_token(&username, &password).await {
                Ok(t) => t,
                Err(e) => {
                    writes(format!("Error getting new token {:?}", e));
                    continue;
                }
            };
            writes(format!("New token {}", temp.0));
            *keys.lock().await = temp.0;
        }
    })
}

pub async fn get_new_link_token(
    username: &str,
    password: &str,
) -> Result<(String, u64), Box<dyn Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let mut map = HashMap::new();
    map.insert("username", &username);
    map.insert("password", &password);
    writes("Getting new token".to_string());
    let discord_link_token = serde_json::from_str::<Value>(
        &client
            .post("https://cocdiscord.link/login")
            .json(&map)
            .send()
            .await?
            .text()
            .await?,
    )?;
    writes(format!("got json: {:?}", discord_link_token));
    let discord_link_token = discord_link_token["token"]
        .as_str()
        .unwrap_to_err("could not get token from json")?;
    writes(format!("got token: {}", discord_link_token));
    Ok((
        discord_link_token.to_string(),
        decode_jwt_for_time_left(discord_link_token)?,
    ))
}

fn parse_args() -> Config {
    let args = std::env::args();
    let mut config = Config::new();
    for arg in args {
        match arg.as_str() {
            "--log" => {
                config.log = true;
            }
            "--help" => {
                println!("--log to log to file");
                exit(0);
            }
            _ => {}
        }
    }
    config
}

#[derive(Debug, Clone)]
pub struct Config {
    pub log: bool,
}

impl Config {
    pub fn new() -> Self {
        Self { log: false }
    }
}

pub fn writes<T: Display>(msg: T) {
    match SHOULD_LOG.to_owned() {
        true => log::info!("{}", msg),
        false => println!("{}", msg),
    }
}

pub trait UnwrapToErr<T, D: fmt::Display> {
    fn unwrap_to_err(self, msg: D) -> Result<T, String>;
}

impl<T, D: fmt::Display> UnwrapToErr<T, D> for Option<T> {
    fn unwrap_to_err(self, msg: D) -> Result<T, String> {
        match self {
            Some(t) => Ok(t),
            None => Err(msg.to_string()),
        }
    }
}

impl<T, D: fmt::Display> UnwrapToErr<T, D> for Result<T, Box<dyn Error>> {
    fn unwrap_to_err(self, msg: D) -> Result<T, String> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(format!("{}: {}", msg, e)),
        }
    }
}
