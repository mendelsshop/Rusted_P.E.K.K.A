use coc_rs::{clan::Clan, player::Player};
use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;
#[command]
#[help_available]
#[description = "Get a player's name"]
async fn player(ctx: &Context, msg: &Message) -> CommandResult {
    if let Some(player) = get_player(ctx, msg).await {
        msg.reply(&ctx.http, format!("Player: {}", player.name))
            .await?;
    }
    Ok(())
}

#[command]
#[help_available]
#[description = "Get a player's clan"]
async fn clan(ctx: &Context, msg: &Message) -> CommandResult {
    if let Some(clan) = get_clan(ctx, msg).await {
        msg.reply(&ctx.http, format!("Player: {}", clan.name))
            .await?;
    }
    Ok(())
}

async fn get_player(ctx: &Context, msg: &Message) -> Option<Player> {
    Rusted_PEKKA::writes(format!("retrieving player for {}", msg.author.name));
    match Rusted_PEKKA::get_player_id(msg.author.id.0, ctx).await {
        Ok(player_tag) => {
            let coc_client = Rusted_PEKKA::get_coc_client(ctx).await;
            Rusted_PEKKA::writes(format!("Player tag: {}", player_tag));
            match coc_client.get_player(&player_tag).await {
                Ok(p) => Some(p),
                Err(why) => {
                    Rusted_PEKKA::writes(format!("Error getting player: {:?}", why));
                    msg.reply(&ctx.http, "Error getting player").await.ok()?;
                    None
                }
            }
        }
        Err(why) => match why.to_string().as_str() {
            "non recoverable error" => {
                Rusted_PEKKA::writes(format!("Error getting player id: {}", why));
                msg.reply(&ctx.http, "Error getting player id").await.ok()?;
                None
            }
            _ => {
                Rusted_PEKKA::writes(format!("Error getting player id: {}", why));
                msg.reply(
                    &ctx.http,
                    "please try again network connection was interupted",
                )
                .await
                .ok()?;
                None
            }
        },
    }
}

async fn get_clan(ctx: &Context, msg: &Message) -> Option<Clan> {
    Rusted_PEKKA::writes(format!("clan name requested for {}", msg.author.name));
    let player = get_player(ctx, msg).await?;
    let pc = player.clan?;
    let coc_client = Rusted_PEKKA::get_coc_client(ctx).await;
    match coc_client.get_clan(&pc.tag).await {
        Ok(clan) => Some(clan),
        Err(_) => {
            msg.reply(&ctx.http, "could not retrieve clan").await.ok()?;
            None
        }
    }
}
