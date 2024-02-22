use log::{error, info};
use reqwest::Error;
use serenity::builder::CreateMessage;
use serenity::model::channel::Message;
use serenity::prelude::*;

use crate::bot::Bot;
use crate::cfg::SYS_PROMPT;
use crate::openai::{ChatMessage, ChatRequest, OPENAI_API_URL};

impl Bot {
    pub async fn gen_msg(&self, ctx: &Context, msg: &Message) {
        info!("Message handling disabled...");
        // let typing = msg.channel_id.start_typing(&ctx.http);

        // if let Ok(text) = self.gen_with_prompt(&msg, SYS_PROMPT).await {
        //     self.send_msg(&ctx, &msg, &text).await;
        // }

        // typing.stop();
    }

    pub async fn gen_with_prompt(&self, msg: &Message, sys_prompt: &str) -> Result<String, Error> {
        let sys_prompt = format!(
            "{}\nConversation history:\n{}",
            sys_prompt,
            self.get_history_text(10)
        );
        let new_msg = format!("{}: {}", &msg.author.name, &msg.content);

        let res = self
            .client
            .post(format!("{OPENAI_API_URL}/chat/completions"))
            .json(&ChatRequest {
                model: self.model.clone(),
                messages: vec![
                    ChatMessage::new("system", &sys_prompt),
                    ChatMessage::new("user", &new_msg),
                ],
            })
            .send()
            .await?;

        let data = res.json::<serde_json::Value>().await?;
        let mut text = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("idk")
            .to_string();

        if text.contains(":") {
            let split = text.split(": ").collect::<Vec<&str>>();
            if split.len() > 1 {
                text = split[1].to_string();
            }
        }

        Ok(text)
    }

    pub async fn handle_msg(&self, msg: &Message, res: &str) {
        info!("{}: {}", msg.author.name, msg.content);
        info!("{}: {}", "tardbot", res);

        self.add_history(&msg.author.name, &msg.content);
        self.add_history("tardbot", &res);
    }

    pub async fn send_msg(&self, ctx: &Context, msg: &Message, res: &str) {
        self.handle_msg(&msg, &res).await;

        if let Err(e) = msg.channel_id.say(&ctx, res).await {
            error!("Failed to send message: {}", e);
        }
    }

    #[allow(dead_code)]
    pub async fn send_dm(&self, ctx: &Context, msg: &Message, res: &str) {
        self.handle_msg(&msg, &res).await;

        if let Err(e) = msg
            .author
            .direct_message(ctx, CreateMessage::new().content(res))
            .await
        {
            error!("Failed to send DM: {}", e);
        }
    }
}
