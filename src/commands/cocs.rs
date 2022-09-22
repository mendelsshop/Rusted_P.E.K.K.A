use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
#[help_available]
#[description = "Get a player's name"]
async fn player(ctx: &Context, msg: &Message) -> CommandResult {
    Rusted_PEKKA::writes(format!("player name requested for {}", msg.author.name));
match Rusted_PEKKA::get_player_id(msg.author.id.0, ctx).await {
    Ok(player_tag) => {
        let coc_client = Rusted_PEKKA::get_coc_client(ctx).await;
        Rusted_PEKKA::writes(format!("Player tag: {}", player_tag));
        let player = match coc_client.get_player(&player_tag).await {
            Ok(p) => p,
            Err(why) => {
                Rusted_PEKKA::writes(format!("Error getting player: {:?}", why));
                msg.reply(&ctx.http, "Error getting player").await?;
                return Ok(());
            }
        };
        msg.reply(&ctx.http, format!("Player: {}", player.name))
            .await?;
    }
    Err(why) => { match why.to_string().as_str() {
        "non recoverable error" => {
            Rusted_PEKKA::writes(format!("Error getting player id: {}", why));
            msg.reply(&ctx.http, "Error getting player id").await?;
        }
        _ => {
            Rusted_PEKKA::writes(format!("Error getting player id: {}", why));
            msg.reply(&ctx.http, "please try again network connection was interupted").await?;
        }
    
    }
        }        

    } 
    Ok(())
}
