use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
#[help_available]
#[description = "Get a player's name"]
async fn player(ctx: &Context, msg: &Message) -> CommandResult {
    if let Some(player_tag) = Rusted_PEKKA::get_player_id(msg.author.id.0, ctx).await {
        println!("Player tag: {}", player_tag);
        let coc_client = Rusted_PEKKA::get_coc_client(ctx).await;
        let player = match coc_client.get_player(&player_tag).await {
            Ok(p) => p,
            Err(why) => {
                println!("Error getting player: {:?}", why);
                msg.reply(&ctx.http, "Error getting player")
                    .await?;
                return Ok(());
            }
        };
        msg.reply(&ctx.http, format!("Player: {}", player.name))
            .await?;
    } else {
        msg.reply(&ctx.http, "No player tag provided, use /link to link your account")
            .await?;

    }

    Ok(())
}
