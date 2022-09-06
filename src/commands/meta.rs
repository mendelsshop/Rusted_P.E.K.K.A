use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!, Took `Some amount of time`!").await?;

    Ok(())
}
#[command]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "I am made in rust.\nI am a P.E.K.K.A no longer in service,\nbecause I rusted near the spell factory,\nhere to help all the clash chiefs. ").await?;

    Ok(())
}