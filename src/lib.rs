use std::{sync::Arc, collections::HashMap};

use serenity::{prelude::*, client::bridge::gateway::ShardManager, model::prelude::Message, framework::standard::CommandResult};

use coc_rs::{api::Client as CocClient};

pub async fn get_coc_client(ctx: &Context) -> CocClient {
    let data = ctx.data.read().await;
    data.get::<CocClientContainer>().expect("Expected coc client in TypeMap").clone()
}

pub async fn get_user_message(ctx: &Context, id: u64) -> Option<(u8, String)> {
    let data = ctx.data.read().await;
    let user_messages = data.get::<UserMessageContainer>().expect("Expected user messages in TypeMap");
    user_messages.get(&id).cloned()
}

pub async fn set_user_message(ctx: &Context, id: u64, message: (u8, String)) {
    let mut data = ctx.data.write().await;
    let user_messages = data.get_mut::<UserMessageContainer>().expect("Expected user messages in TypeMap");
    user_messages.insert(id, message);
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

pub async fn too_many_tries(msg: String, ctx: &Context, id: u64) -> bool {
    let data = get_user_message(ctx, id).await;
    let prev: (u8, String) = match data {
        Some(p) => p,
        None => {
            set_user_message(ctx, id, (1, msg)).await;
            return false;}
    };
    if *prev.1 == msg {
        set_user_message(ctx, id, (prev.0 + 1, msg)).await;
    } else {
        set_user_message(ctx, id, (0, msg)).await;
    }
    prev.0 >= 3
}

pub async fn check_to_many_times(ctx: &Context, msg: &Message, cmd: String) -> CommandResult {
    if too_many_tries(cmd, &ctx, msg.author.id.0).await {
        let times = get_user_message(&ctx, msg.author.id.0).await.unwrap().0;
        msg.channel_id.say(&ctx.http, format!("Calm down you've done the same command {} times!", times)).await?;
        return Err("Too many tries".into());
    };
    Ok(())
}