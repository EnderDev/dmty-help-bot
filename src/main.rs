use std::collections::HashMap;
use std::env;
use std::time::Duration;

use dotenv::dotenv;

use serenity::async_trait;
use serenity::builder::{CreateActionRow, CreateSelectMenu, CreateSelectMenuOption};
use serenity::collector::MessageCollectorBuilder;
use serenity::futures::StreamExt;
use serenity::json::json;
use serenity::model::channel::{GuildChannel, ReactionType};
use serenity::model::gateway::Ready;
use serenity::model::interactions::message_component::{ButtonStyle};
use serenity::model::interactions::{Interaction, InteractionResponseType};
use serenity::prelude::*;
use serenity::utils::Colour;

fn send_help_msg<'a>(
    ctx: Context,
    channel: &'a GuildChannel,
    faq_channel: &'a GuildChannel,
) -> impl std::future::Future<
    Output = std::result::Result<serenity::model::channel::Message, serenity::Error>,
> + 'a {
    channel.send_message(ctx.http, |m| {
        m.embed(|e| {
            e.colour(Colour::from_rgb(255, 213, 97))
            .title("🤝 Welcome to Help").description(format!(
                "Before you create a help thread, you should read <#{}> first.",
                faq_channel.id.0
            ))
        })
        .components(|c| {
            c.create_action_row(|r| {
                r.create_button(|b| {
                    b.custom_id("assist-help")
                        .emoji(ReactionType::Unicode("🤝".to_string()))
                        .label("I need help")
                })
                .create_button(|b| {
                    b.custom_id("assist-question")
                        .emoji(ReactionType::Unicode("🙋".to_string()))
                        .label("I have a question")
                })
                .create_button(|b| {
                    b.custom_id("assist-bug")
                        .emoji(ReactionType::Unicode("🐞".to_string()))
                        .label("I've found a bug or problem")
                })
                .create_button(|b| {
                    b.custom_id("assist-feature")
                        .emoji(ReactionType::Unicode("💡".to_string()))
                        .label("I have a feature suggestion")
                })
                .create_button(|b| {
                    b.style(ButtonStyle::Link).label("FAQ").url(format!(
                        "https://discord.com/channels/525056817399726102/{}",
                        faq_channel.id.0
                    ))
                })
            })
        })
    })
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let component = interaction.clone().message_component().unwrap();
        let mut msg = component.clone().message;
        let button_id = component.data.custom_id.as_str();

        if button_id.starts_with("assist-") {
            if button_id.starts_with("assist-close-thread") {
                msg.edit(&ctx.http, |m| {
                    m.content("Closing thread...").suppress_embeds(true)
                }).await.unwrap();

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                msg.channel(&ctx.http).await.unwrap().delete(&ctx.http).await.unwrap();
                return;
            }

            let options = vec![
                "Windows", 
                "Linux", 
                "macOS", 
                "BSD", 
                "Unix-like", 
                "Product: Dot Browser for Desktop", 
                "Product: Dot Browser for Android", 
                "Product: Dot One", 
                "Product: Dot Shield", 
                "Product: Other"
            ];

            if button_id.starts_with("assist-tags-select-dismiss") {
                msg.delete(&ctx.http).await.unwrap();

                return;
            }

            if button_id.starts_with("assist-add-tags") {
                let mut select_menu = CreateSelectMenu::default();

                select_menu.custom_id("assist-tags-select");
                select_menu.placeholder("No tags selected");
                select_menu.min_values(0);
                select_menu.max_values(options.len().try_into().unwrap());

                let mut select_menu_options: Vec<CreateSelectMenuOption> = vec![];

                let mut existing_options = if msg.embeds[0].fields[0].value == "*<no tags>*" {
                    vec![]
                } else {
                    msg.embeds[0].fields[0].value.split(", ").collect::<Vec<_>>()
                };

                existing_options.sort_by_key(|name| name.to_lowercase());

                for o in options {
                    let mut opt = CreateSelectMenuOption::default();

                    opt.value(o);
                    opt.label(o);
                    opt.default_selection(existing_options.contains(&o));
                    
                    select_menu_options.push(opt);
                }

                select_menu.options(|o| o.set_options(select_menu_options));

                let tags_msg = msg.channel(&ctx.http).await.unwrap().guild().unwrap().send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e
                            .colour(msg.embeds[0].colour.unwrap())
                            .title("🏷️ Add thread tags")
                            .description("Tags help your thread get solved quicker, select a couple from the list below.")
                            .footer(|f| {
                                f.text("Tapping outside the selection menu will save your selections.")
                            })
                            
                    }).components(|c| {
                        c
                            .create_action_row(|r| {
                                r.add_select_menu(select_menu)
                            })
                            .create_action_row(|r| {
                                r.create_button(|b| {
                                    b.custom_id("assist-tags-select-dismiss").label("Dismiss").style(ButtonStyle::Secondary)
                                })
                            })
                    })
                }).await.unwrap();

                let mut cib = tags_msg.await_component_interactions(&ctx).build();

                while let Some(mci) = cib.next().await {
                    mci.create_interaction_response(&ctx, |r| {
                        r.kind(InteractionResponseType::ChannelMessageWithSource).interaction_response_data(
                            |d| {
                                return d.ephemeral(true).content("Updated tags.");
                            }
                        )
                    }).await.unwrap();

                    let tags = if mci.data.values.is_empty() {
                        "*<no tags>*".to_string()
                    } else {
                        mci.data.values.join(", ").to_string()
                    };
        
                    tags_msg.delete(&ctx.http).await.unwrap();

                    let cloned_msg = msg.clone();

                    msg.edit(&ctx.http, |m| {
                        m.content("").embed(|e| {
                            e
                                .colour(cloned_msg.embeds[0].colour.unwrap())
                                .title(cloned_msg.embeds[0].clone().title.unwrap())
                                .description(cloned_msg.embeds[0].clone().description.unwrap())
                                .field("🏷️ Tags", tags, false)
                                .footer(|f| {
                                    f
                                        .text(cloned_msg.embeds[0].footer.clone().unwrap().text)
                                        .icon_url(cloned_msg.embeds[0].footer.clone().unwrap().icon_url.unwrap())
                                })
                        }).components(|c| c.set_action_row(create_thread_action_row(false)) )
                    }).await.unwrap();
                }

                return;
            }

            let thread_prefixes = HashMap::from([
                ("help", "Help"),
                ("question", "Question"),
                ("bug", "Bug"),
                ("feature", "Feature"),
            ]);
    
            let thread_emojis = HashMap::from([
                ("help", "🤝"),
                ("question", "🙋"),
                ("bug", "🐞"),
                ("feature", "💡"),
            ]);
    
            let thread_colours = HashMap::from([
                ("help", Colour::from_rgb(255, 213, 97)),
                ("question", Colour::from_rgb(253, 103, 63)),
                ("bug", Colour::from_rgb(220, 40, 63)),
                ("feature", Colour::from_rgb(254, 194, 83)),
            ]);
            
            let thread_type = button_id.split("assist-").collect::<Vec<_>>()[1];
            let thread_emoji = thread_emojis.get(thread_type).unwrap();
            let thread_prefix = thread_prefixes.get(thread_type).unwrap();
            let thread_colour = thread_colours.get(thread_type).unwrap();
    
            let thread_title_description_intent = match thread_type {
                "question" => "question",
                "feature" => "feature suggestion",
                _ => "problem"
            };

            let guild_channel = msg.channel(&ctx.http).await.unwrap().guild().unwrap();

            let member = component.clone().member.unwrap();
            let username = member.display_name();

            let thread_name = format!(
                "{} {}{} thread",
                thread_emoji,
                username,
                if username[username.len() - 1..].to_lowercase() == "s" {
                    "'"
                } else {
                    "'s"
                }
            );

            let thread_data = json!({
                "name": thread_name,
                "type": 11u32
            })
            .as_object()
            .unwrap()
            .clone();

            let thread = ctx
                .http
                .create_private_thread(guild_channel.id.0, &thread_data)
                .await
                .unwrap();

            msg.delete(&ctx.http).await.unwrap();

            fn create_thread_action_row(add_tags_disabled: bool) -> serenity::builder::CreateActionRow {
                let mut thread_action_row = CreateActionRow::default();

                thread_action_row.create_button(|b| {
                    b.label("Add tags").style(ButtonStyle::Secondary).custom_id("assist-add-tags").disabled(add_tags_disabled)
                });
                
                thread_action_row.create_button(|b| {
                    b.label("Close thread").style(ButtonStyle::Secondary).custom_id("assist-close-thread")
                });

                thread_action_row
            }

            let mut title_msg = thread.send_message(&ctx.http, |m| {
                m
                    .content(format!("<@{}>", member.user.id.0))
                    .embed(|e| {
                        e
                            .colour(*thread_colour)
                            .title("🏷️ Set the thread title")
                            .description(format!("Provide a **short description** of your {} by typing into the thread.", thread_title_description_intent))
                            .footer(|f| {
                                f.text("The thread will automatically close if there is no activity within 10 minutes.")
                            })
                    }).components(|c| {
                        c.set_action_row(create_thread_action_row(true)) 
                    })
            }).await.unwrap();

            title_msg.pin(&ctx.http).await.unwrap();

            let collector = MessageCollectorBuilder::new(&ctx)
                .author_id(member.user.id.0)
                .channel_id(thread.id.0)
                .collect_limit(1u32)
                .timeout(Duration::from_secs(600))
                .build();

            let collected = &collector.collect::<Vec<_>>().await;
        
            if collected.is_empty() {
                title_msg.edit(&ctx.http, |m| {
                    m.content("Closing thread...").suppress_embeds(true)
                }).await.unwrap();

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                thread.id.delete(&ctx.http).await.unwrap();
            } else {
                let thread_title = &collected[0].content;

                thread.id.edit_thread(&ctx.http, |t| {
                    t.name(format!("{} {}", thread_emoji, thread_title))
                }).await.unwrap();

                let user_avatar = member.user.avatar_url().or_else(|| Some(member.user.default_avatar_url())).unwrap();

                title_msg.edit(&ctx.http, |m| {
                    m.content("").embed(|e| {
                        e
                            .colour(*thread_colour)
                            .title(format!("{} {}", thread_emoji, thread_prefix))
                            .description(thread_title)
                            .field("🏷️ Tags", "*<no tags>*", false)
                            .footer(|f| {
                                f
                                    .text(format!("Opened by {}#{}", member.user.name, member.user.discriminator))
                                    .icon_url(user_avatar)
                            })
                    }).components(|c| c.set_action_row(create_thread_action_row(false)) )
                }).await.unwrap();
            }
        }
    }

    async fn thread_create(&self, ctx: Context, thread: GuildChannel) {
        let guild_id = env::var("GUILD_ID")
            .expect("Expected a guild ID in the environment")
            .parse::<u64>()
            .unwrap();

        let guild_channels = ctx.http.get_channels(guild_id).await.unwrap();

        let help_channel = guild_channels
            .iter()
            .find(|ch| ch.name() == "help")
            .expect("No #help channel found");

        let faq_channel = guild_channels
            .iter()
            .find(|ch| ch.name() == "faq")
            .expect("No #faq channel found");

        if thread.parent_id.unwrap() == help_channel.id {
            send_help_msg(ctx, help_channel, faq_channel).await.unwrap();
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let token = env::var("TOKEN").expect("Expected a token in the environment");
    env::var("GUILD_ID").expect("Expected a guild ID in the environment");

    let intents = GatewayIntents::GUILDS 
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
