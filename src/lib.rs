use std::sync::Arc;

use serenity::{prelude::*, client::bridge::gateway::ShardManager};

use coc_rs::{api::Client as CocClient};

pub async fn get_coc_client(ctx: &Context) -> CocClient {
    let data = ctx.data.read().await;
    data.get::<CocClientContainer>().expect("Expected coc client in TypeMap").clone()
}

pub struct CocClientContainer;

impl TypeMapKey for CocClientContainer {
    type Value = CocClient;
}

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}
