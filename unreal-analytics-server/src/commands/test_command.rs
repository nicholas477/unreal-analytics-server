use futures::TryFutureExt;
use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    let state = crate::SERVER_STATE
        .get()
        .ok_or(serenity::Error::Other("Failed to load server state"))?;

    let player_stats = state
        .db
        .get_players_stats()
        .map_err(|err| {
            eprintln!("Database error! {err:?}");
            serenity::Error::Other("Database error")
        })
        .await?;

    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!("Player stats: ```{}```", player_stats)),
            ),
        )
        .await?;

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("test_command").description("Asks some details about you")
}
