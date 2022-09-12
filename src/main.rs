mod commands;

use std::{collections::{HashMap, HashSet}, sync::Arc};
use std::env;

use std::process::exit;

use serenity::{async_trait, framework::standard::{macros::help, Args, HelpOptions, CommandGroup, CommandResult, help_commands}, model::prelude::{Message, UserId}};
use serenity::framework::standard::macros::group;
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use Rusted_PEKKA::{CocClientContainer, ShardManagerContainer, UserMessageContainer, DiscordLinkAPIContainer, writes};

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
        Rusted_PEKKA::writes(format!("Resumed"));
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
    Rusted_PEKKA::writes(format!("Help command called"));
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[group]
#[commands(ping, quit, about, player)]
struct General;
#[tokio::main]
async fn main()  {
    if Rusted_PEKKA::SHOULD_LOG.to_owned() {
        simple_file_logger::init_logger("Rusted_P.E.K.K.A", simple_file_logger::LogLevel::Info).unwrap();
        Rusted_PEKKA::writes(format!("Logging enabled"));
    }
    println!("Rusted_P.E.K.K.A");
    // TODO: stop using unwrap everywhere and use proper error handling
    // and check for bad responces from rqwest
    let discord_link_user = env::var("discordlink_username").expect("Expected DISCORD_LINK_USER in environment");
    let discord_link_password = env::var("discordlink_password").expect("Expected DISCORD_LINK_PASSWORD in environment");
    let discord_link_token = match  Rusted_PEKKA::get_new_link_token(&discord_link_user, &discord_link_password).await {
        Ok(token) => token.0,
        Err(why) => {
            writes(format!("Error getting link token: {:?}", why));
            exit(1);
        }
    };
    writes(format!("Got link token: {}", discord_link_token));
    let discord_link_token = Arc::new(Mutex::new(discord_link_token.to_string()));
    Rusted_PEKKA::check_link_api_update(&discord_link_token, discord_link_user.to_string(), discord_link_password.to_string()).await;
    let coc_credentials = CocCredentials::builder()
        .add_credential(
            env::var("cocapi_username").expect("coc api email not found"),
            env::var("cocapi_password").expect("Password not found"),
        )
        .build();
    writes(format!("found credentials: {:?}", coc_credentials));
    let coc_client = match CocClient::new(coc_credentials).await {
        Ok(c) => c,
        Err(why) => {
            writes(format!("Error creating coc api client: {:?}", why));
            exit(1);
        }
    };
    Rusted_PEKKA::writes(format!("connected to coc api"));
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
        .group(&GENERAL_GROUP).help(&MY_HELP);

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
}
