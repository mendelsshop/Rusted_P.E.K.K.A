use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    match Rusted_PEKKA::check_to_many_times(ctx, msg, "ping".to_string()).await {
        Ok(_) => {
            msg.reply(ctx,  "Pong!, Took `Some amount of time`!").await?;
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
    Ok(())
}
#[command]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {

    match Rusted_PEKKA::check_to_many_times(ctx, msg, "about".to_string()).await {
        Ok(_) => {
            msg.reply(&ctx.http, "I am made in rust.\nI am a P.E.K.K.A no longer in service,\nbecause I rusted near the spell factory,\nhere to help all the clash chiefs. ").await?;
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }

    Ok(())
}
