use std::env;

use serenity::async_trait;
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::{Command, Interaction};
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::prelude::*;
use std::sync::Arc;

struct Handler;

use crate::commands;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            println!("Received command interaction: {command:#?}");

            let content = match command.data.name.as_str() {
                "ping" => Some(commands::ping::run(&command.data.options())),
                "test_command" => {
                    commands::test_command::run(&ctx, &command).await.unwrap();
                    None
                }
                "print_config" => {
                    commands::print_config::run(&ctx, &command).await.unwrap();
                    None
                }
                _ => Some("not implemented :(".to_string()),
            };

            if let Some(content) = content {
                let data = CreateInteractionResponseMessage::new().content(content);
                let builder = CreateInteractionResponse::Message(data);
                if let Err(why) = command.create_response(&ctx.http, builder).await {
                    println!("Cannot respond to slash command: {why}");
                }
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        for guild in ready.guilds {
            let guild_id = GuildId::new(guild.id.get());

            let commands = guild_id
                .set_commands(
                    &ctx.http,
                    vec![
                        commands::ping::register(),
                        commands::test_command::register(),
                        commands::print_config::register(),
                    ],
                )
                .await;

            // println!("I now have the following guild slash commands: {commands:#?}");
        }
    }
}

pub fn initialize() {
    let state = crate::get_server_state();
    let token = state.secrets.keys.discord_token.clone();

    tokio::task::spawn(async move {
        let mut client = Client::builder(token, GatewayIntents::empty())
            .event_handler(Handler)
            .await
            .expect("Error creating client");

        // Finally, start a single shard, and start listening to events.
        //
        // Shards will automatically attempt to reconnect, and will perform exponential backoff until
        // it reconnects.
        if let Err(why) = client.start().await {
            eprintln!("Client error: {why:?}");
        }
    });
}
