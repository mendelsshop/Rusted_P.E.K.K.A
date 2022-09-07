use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
async fn player(ctx: &Context, msg: &Message) -> CommandResult {
    match msg.content.split_whitespace().nth(1) {
        Some(player_tag) => {
            let coc_client = Rusted_PEKKA::get_coc_client(ctx).await;
            let player = match coc_client.get_player(player_tag).await {
                Ok(p) => p,
                Err(why) => {
                    println!("Error getting player: {:?}", why);
                    msg.channel_id
                        .say(&ctx.http, "Error getting player")
                        .await?;
                    return Ok(());
                }
            };
            msg.channel_id
                .say(&ctx.http, format!("Player: {}", player.name))
                .await?;
        }
        None => {
            msg.channel_id
                .say(&ctx.http, "No player tag provided")
                .await?;
        }
    }
    Ok(())
}
