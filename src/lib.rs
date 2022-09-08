use std::{collections::HashMap, sync::Arc, convert::TryInto, error::Error};

use serde_json::Value;
use serenity::{
    client::bridge::gateway::ShardManager, framework::standard::CommandResult,
    model::prelude::Message, prelude::*,
};
use std::time::{SystemTime, UNIX_EPOCH};
use coc_rs::api::Client as CocClient;

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

pub async fn get_player_id(discord_id: u64, ctx: &Context) -> Option<String> {
    let discord_link_api = get_discord_link_api(ctx).await;
    let discord_link_api: String = discord_link_api.lock().await.clone();
    let client = reqwest::Client::new();
    let mut player_id = client.get(format!("https://cocdiscord.link/links/{discord_id}"));
    player_id = player_id.bearer_auth(discord_link_api);
    let player_id = player_id.send().await.unwrap();
    match player_id.status() {
        reqwest::StatusCode::OK => {
            let player_id = player_id.text().await.unwrap();
            let player_id: Value = serde_json::from_str(&player_id).unwrap();
            let player_id = player_id.as_array().unwrap()[0]["playerTag"].as_str().unwrap();
            Some(player_id.to_string())

        }
        _ => None,
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
        println!("Too many tries {}", cmd);
        let times = get_user_message(ctx, msg.author.id.0).await.unwrap().0;
        msg.reply(
                &ctx.http,
                format!("Calm down you've done /{} {} times!",cmd, times),
            )
            .await?;
        return Err("Too many tries".into());
    };
    println!("Not too many tries {}", cmd);
    Ok(())
}

pub fn decode_jwt_for_time_left(token: &str) -> Result<bool, Box<dyn Error>> {
    let mut split_token = token.split('.').collect::<Vec<&str>>();
    split_token.pop();
    let split_token: [&str; 2] = match split_token.try_into() {
        Ok(token) => token,
        Err(t) => return Err(format!("invalid token cold not be parsed {:?}", t).as_str())?,
    };
    let mut split_token_string: [String; 2] = ["".to_string(), "".to_string()];
    for (i, token) in split_token.into_iter().enumerate() {
        let t = base64::decode_config(token, base64::URL_SAFE_NO_PAD).unwrap();
        split_token_string[i]=String::from_utf8(t).unwrap();
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let t = serde_json::from_str::<Value>(&split_token_string[1])?;
    let t: u64 = t.get("exp").unwrap().as_f64().unwrap() as u64;
    
    if t < now {
        return Ok(true);
    }

    Ok(false)
}

pub async fn check_link_api_update(key: &Arc<Mutex<String>>, username: String, password: String)  {
    let keys = Arc::clone(key);
    tokio::spawn(async move {
        loop {
            if decode_jwt_for_time_left(keys.lock().await.as_str()).unwrap() {
                println!("Updating link api key");
                let client = reqwest::Client::new();
                    let mut map = HashMap::new();
                    map.insert("username", &username);
                    map.insert("password", &password);
                    let discord_link_token = serde_json::from_str::<Value>(&client.post("https://cocdiscord.link/login").json(&map).send().await.unwrap().text().await.unwrap()).unwrap();
                    let discord_link_token = discord_link_token["token"].as_str().unwrap();
                    *keys.lock().await = discord_link_token.to_string();
            }
            std::thread::sleep(std::time::Duration::from_secs(600));
        }
    });
}