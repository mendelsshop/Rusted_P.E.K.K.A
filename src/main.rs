mod commands;

use std::env;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use std::process::exit;

use serenity::framework::standard::macros::group;
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::{
    async_trait,
    framework::standard::{
        help_commands, macros::help, Args, CommandGroup, CommandResult, HelpOptions,
    },
    model::prelude::{Message, UserId},
};
use Rusted_PEKKA::{
    CocClientContainer, DiscordLinkAPIContainer, ShardManagerContainer, UserMessageContainer,
};

use coc_rs::{api::Client as CocClient, credentials::Credentials as CocCredentials};

use crate::commands::cocs::*;
use crate::commands::meta::*;
use crate::commands::owner::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        Rusted_PEKKA::writes(format!("Connected as {}", ready.user.name));
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        Rusted_PEKKA::writes("Resumed".to_string());
    }
}

#[help]
#[individual_command_tip = "Rusted_P.E.K.K.A Commands:\nIf you want more information about a specific command, just pass the command as argument."]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    Rusted_PEKKA::writes("Help command called".to_string());
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[group]
#[commands(ping, quit, about, player)]
struct General;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if Rusted_PEKKA::SHOULD_LOG.to_owned() {
        simple_file_logger::init_logger("Rusted_P.E.K.K.A", simple_file_logger::LogLevel::Info)
            .unwrap();
        Rusted_PEKKA::writes("Logging enabled".to_string());
    }

    // TODO: stop using unwrap everywhere and use proper error handling
    // and check for bad responces from rqwest
    let discord_link_user =
        env::var("discordlink_username").expect("Expected DISCORD_LINK_USER in environment");
    let discord_link_password =
        env::var("discordlink_password").expect("Expected DISCORD_LINK_PASSWORD in environment");
    let discord_link_token =
        Rusted_PEKKA::get_new_link_token(&discord_link_user, &discord_link_password)
            .await?
            .0;
    let discord_link_token = Arc::new(Mutex::new(discord_link_token.to_string()));
    Rusted_PEKKA::check_link_api_update(
        &discord_link_token,
        discord_link_user.to_string(),
        discord_link_password.to_string(),
    )
    .await;
    let coc_credentials = CocCredentials::builder()
        .add_credential(
            env::var("cocapi_username").expect("coc api email not found"),
            env::var("cocapi_password").expect("Password not found"),
        )
        .build();
    Rusted_PEKKA::writes(format!("found credentials: {:?}", coc_credentials));
    let coc_client = match CocClient::new(coc_credentials).await {
        Ok(c) => c,
        Err(why) => {
            Rusted_PEKKA::writes(format!("Error creating coc api client: {:?}", why));
            exit(1);
        }
    };
    Rusted_PEKKA::writes("connected to coc api".to_string());
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
        .group(&GENERAL_GROUP)
        .help(&MY_HELP);

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
        data.insert::<DiscordLinkAPIContainer>(discord_link_token);
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
    Ok(())
}
