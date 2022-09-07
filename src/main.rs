mod commands;

use std::collections::{HashMap, HashSet};
use std::env;

use std::process::exit;

use serenity::async_trait;
use serenity::framework::standard::macros::group;
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use Rusted_PEKKA::{CocClientContainer, ShardManagerContainer, UserMessageContainer};

use coc_rs::{api::Client as CocClient, credentials::Credentials as CocCredentials};

use crate::commands::cocs::*;
use crate::commands::meta::*;
use crate::commands::owner::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        println!("Resumed");
    }
}

#[group]
#[commands(ping, quit, about, player)]
struct General;

#[tokio::main]
async fn main() {
    let credentials = CocCredentials::builder()
        .add_credential(
            env::var("username").expect("coc api email not found"),
            env::var("password").expect("Password not found"),
        )
        .build();
    println!("found credentials: {:?}", credentials);
    let coc_client = match CocClient::new(credentials).await {
        Ok(c) => c,
        Err(why) => {
            println!("Error creating coc api client: {:?}", why);
            exit(1);
        }
    };
    println!("connected to coc api");
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new(&token);

    // We will fetch your bot's owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("/"))
        .group(&GENERAL_GROUP);

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<CocClientContainer>(coc_client.clone());
        data.insert::<UserMessageContainer>(HashMap::new());
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        log::error!("Client error: {:?}", why);
    }
}
