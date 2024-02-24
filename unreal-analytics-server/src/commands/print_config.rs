use futures::TryFutureExt;
use serde::Serialize;
use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    let state = crate::SERVER_STATE
        .get()
        .ok_or(serenity::Error::Other("Failed to load server state"))?;

    let config = state.read_config().ok_or(serenity::Error::Other(
        "Failed to acquire read lock on server state",
    ))?;

    let config_string = toml::to_string(&config).or(Err(serenity::Error::Other(
        "Unable to serialize config to toml",
    )))?;

    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!("```toml\n{}\n```", config_string)),
            ),
        )
        .await?;

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("print_config").description("Prints the config file values")
}
