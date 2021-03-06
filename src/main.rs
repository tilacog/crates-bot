extern crate telebot;
extern crate tokio_core;
extern crate futures;
extern crate erased_serde;
extern crate crates_api;

use telebot::RcBot;
use tokio_core::reactor::Core;
use futures::stream::Stream;
use std::env;
use futures::IntoFuture;

use erased_serde::Serialize;

use telebot::functions::*;
use telebot::objects::*;

fn inline_result(crates: Vec<crates_api::Crate>) -> Vec<Box<Serialize>> {
    crates
        .into_iter()
        .map(|each_crate| {
            let crate_name = each_crate.name;
            let crate_desc = each_crate.description.unwrap_or("".to_owned());

            let msg_text =
                format!(
                "<strong>Crate</strong>: {}\n<strong>Description</strong>: {}\n\n<strong>Total downloads</strong>: {}. <strong>Recent downloads</strong>: {}",
                &crate_name,
                &crate_desc,
                each_crate.downloads,
                each_crate.recent_downloads
                );
            let input_message_content = InputMessageContent::Text::new(msg_text).parse_mode("html").disable_web_page_preview(true);

            let mut inline_keyboard_buttons = Vec::new();
            if let Some(crate_repo) = each_crate.repository {
                inline_keyboard_buttons.push(InlineKeyboardButton::new("Repository".to_owned()).url(crate_repo));
            }

            if let Some(crate_doc) = each_crate.documentation {
                inline_keyboard_buttons.push(InlineKeyboardButton::new("Documentation".to_owned()).url(crate_doc));
            }

            let inline_keyboard_markup = InlineKeyboardMarkup { inline_keyboard: vec![inline_keyboard_buttons] };

            let inline_resp = InlineQueryResultArticle::new(
                crate_name.clone().into(),
                Box::new(input_message_content),
            ).reply_markup(inline_keyboard_markup);

            Box::new(inline_resp.description(crate_desc)) as Box<Serialize>
        })
        .collect()
}

fn main() {
    // Create a new tokio core
    let mut lp = Core::new().unwrap();

    // Create the bot
    let bot = RcBot::new(lp.handle(), &env::var("TELEGRAM_BOT_KEY").unwrap()).update_interval(200);

    let stream = bot.get_stream()
        .filter_map(|(bot, msg)| {
            println!("{:?}", msg);
            msg.inline_query.map(|query| (bot, query))
        })
        .and_then(|(bot, query)| {
            let crates = crates_api::query(query.query);
            let result: Vec<Box<Serialize>> = if crates.is_ok() {
                inline_result(crates.unwrap().crates)
            } else {
                println!("Error: {:?}", crates);
                vec![
                    Box::new(InlineQueryResultArticle::new(
                        "Error fetching results".into(),
                        Box::new(InputMessageContent::Text::new(
                            "There was an error querying crates api".into(),
                        )),
                    )),
                ]
            };

            bot.answer_inline_query(query.id, result).send()
        });

    // enter the main loop
    lp.run(stream.for_each(|_| Ok(())).into_future()).unwrap();
}
