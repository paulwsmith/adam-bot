use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::fs;

use anyhow::Error;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use hound::{SampleFormat, WavSpec, WavWriter};
use log::{info, warn};
use reqwest::multipart::{Form, Part};
use serenity::all::GuildId;
use serenity::async_trait;
use serenity::client::Context;
use serenity::gateway::ActivityData;
use serenity::model::channel::Message;
use songbird::input::codecs::{CODEC_REGISTRY, PROBE};
use songbird::input::Input;
use songbird::model::id::UserId;
use songbird::model::payload::{ClientDisconnect, Speaking};
use songbird::packet::wrap::Wrap32;
use songbird::packet::rtcp;
use songbird::{CoreEvent, Event, EventContext as Ctx, EventHandler};

use crate::bot::Bot;
use crate::cfg::SYS_PROMPT;
use crate::music::find_song;
use crate::openai::{
    build_json_client, build_multipart_client, ChatMessage, ChatRequest, SpeechRequest,
    OPENAI_API_URL,
};

#[derive(Clone)]
struct Receiver {
    ctx: Context,
    guild_id: GuildId,
    chat_model: String,
    json_client: reqwest::Client,
    multipart_client: reqwest::Client,
    controller: Arc<VoiceController>,
}

struct VoiceReply {
    timestamp: DateTime<Utc>,
    duration: Duration,
}

struct VoiceController {
    last_tick_was_empty: AtomicBool,
    known_ssrcs: DashMap<u32, UserId>,
    accumulator: DashMap<u32, Slice>,
    last_reply: Mutex<Option<VoiceReply>>,
}

struct Slice {
    user_id: u64,
    bytes: Vec<i16>,
    timestamp: DateTime<Utc>,
    first_discord_timestamp: u32,
    last_discord_timestamp: u32,
}

fn compute_discord_timestamp(raw_discord_timestamp: Wrap32) -> u32 {
    let mod_value = (raw_discord_timestamp.0 % Wrap32::new(48).0);
    ((raw_discord_timestamp.0 - mod_value).0 / 48).into()
}


impl Receiver {
    pub fn new(ctx: Context, guild_id: GuildId) -> Self {
        // let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
        // let chat_model = env::var("MODEL").expect("MODEL not set");
        let openai_api_key = "";
        let chat_model = "".to_string();
        let json_client = build_json_client(&openai_api_key).unwrap();
        let multipart_client = build_multipart_client(&openai_api_key).unwrap();

        Self {
            ctx,
            guild_id,
            chat_model,
            json_client,
            multipart_client,
            controller: Arc::new(VoiceController {
                last_tick_was_empty: AtomicBool::default(),
                known_ssrcs: DashMap::new(),
                accumulator: DashMap::new(),
                last_reply: Mutex::new(None),
            }),
        }
    }

    async fn process(&self, slice: &mut Slice) -> Result<(), Error> {
        if let Ok(mut last_reply) = self.controller.last_reply.lock() {
            info!("LAST_REPLY thing is TRUE!!!!");
            if let Some(reply) = last_reply.take() {
                info!("\t NEXT THING is TRUE!!!!");
                let elapsed = Utc::now() - reply.timestamp;
                let remaining = reply.duration - elapsed;

                if remaining > Duration::milliseconds(0) || slice.bytes.len() < 48000 {
                    info!("\t\t FINAL THING TRUE! It's clearing shit");
                    slice.timestamp = Utc::now();
                    slice.first_discord_timestamp = 0;
                    slice.last_discord_timestamp = 0;
                    slice.bytes.clear();

                    return Ok(());
                }
            }
        }

        // let filename = format!("cache/{}_{}.wav", slice.user_id, slice.timestamp.timestamp_millis());
        info!("Saving file. slice.timestamp: [{}], slice.discord_timestamp: [{:?}], Utc.now: [{}]", slice.timestamp.timestamp_millis(), slice.first_discord_timestamp, Utc::now().timestamp_millis());
        let filename = format!("cache/{}_{}_{}_{}.wav", slice.user_id, slice.timestamp.timestamp_millis(), slice.first_discord_timestamp, Utc::now().timestamp_millis());

        self.save(&slice.bytes, &filename);

        slice.timestamp = Utc::now();
        slice.first_discord_timestamp = 0;
        slice.bytes.clear();

        // if let Ok(text) = self.transcribe(&filename).await {
        //     let text = text.to_lowercase();
        //     // let mentioned = ["adam", "add", "i don't know"]
        //     //     .iter()
        //     //     .any(|s| text.contains(s));
        //     let mentioned = false;

        //     match text
        //         .replace("adam", "")
        //         .trim()
        //         .chars()
        //         .filter(|&c| c != ',' && c != '.' && c != '!')
        //         .collect::<String>()
        //         .as_str()
        //     {
        //         t if t.starts_with("play") || t.starts_with("clay") || t.starts_with("lay") => {
        //             let search = t.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");

        //             info!("Searching for {}", search);

        //             let manager = songbird::get(&self.ctx).await.unwrap().clone();

        //             if let Some(handler_lock) = manager.get(self.guild_id) {
        //                 let mut handler = handler_lock.lock().await;

        //                 let (youtube_dl, url) = find_song(&self.ctx, &search).await?;

        //                 info!("Queueing {}", url);

        //                 let (input, _) =
        //                     self.gen_audio(&format!("Queueing up, {}", &search)).await?;
        //                 let _ = handler.play_input(input).set_volume(0.5);

        //                 let handle = handler.enqueue_input(youtube_dl.into()).await;
        //                 let _ = handle.set_volume(0.05);
        //             }
        //         }
        //         t if t.starts_with("stop") => {
        //             let manager = songbird::get(&self.ctx).await.unwrap().clone();

        //             if let Some(handler_lock) = manager.get(self.guild_id) {
        //                 let mut handler = handler_lock.lock().await;
        //                 let _ = handler.stop();

        //                 let queue = handler.queue();
        //                 queue.stop();

        //                 let (input, _) = self
        //                     .gen_audio("Just say the word and I'll be back to play some tunes")
        //                     .await?;
        //                 let _ = handler.play_input(input).set_volume(0.5);
        //             }
        //         }
        //         t if mentioned => {
        //             let res = self.gen_response(&t).await?;
        //             let (input, duration) = self.gen_audio(&res).await?;
        //             self.play_audio(input, duration).await?;
        //         }
        //         _ => {}
        //     }
        // }

        Ok(())
    }

    fn save(&self, pcm_samples: &[i16], filename: &str) {
        let spec = WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        let _ = fs::create_dir_all("cache");
        let mut writer = WavWriter::create(filename, spec).unwrap();

        for &sample in pcm_samples {
            let _ = writer.write_sample(sample);
        }

        let _ = writer.finalize();
    }

    async fn transcribe(&self, filename: &str) -> Result<String, Error> {
        let file = fs::read(&filename)?;
        let form = Form::new()
            .part(
                "file",
                Part::bytes(file)
                    .file_name(filename.to_string())
                    .mime_str("audio/wav")
                    .unwrap(),
            )
            .part("model", Part::text("whisper-1"));

        let res = self
            .multipart_client
            .post(format!("{OPENAI_API_URL}/audio/transcriptions"))
            .multipart(form)
            .send()
            .await?;

        let data = res.json::<serde_json::Value>().await?;
        if let Some(text) = data["text"].as_str() {
            info!("Transcription: {:?}", text);
            return Ok(text.to_string());
        }

        Err(Error::msg("Failed to transcribe audio"))
    }

    async fn gen_response(&self, text: &str) -> Result<String, Error> {
        let data = self
            .json_client
            .post(format!("{OPENAI_API_URL}/chat/completions"))
            .json(&ChatRequest {
                model: self.chat_model.clone(),
                messages: vec![
                    ChatMessage::new("system", &SYS_PROMPT),
                    ChatMessage::new("user", &text),
                ],
            })
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let res = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("idk")
            .to_string();

        info!("Response: {:?}", res);

        Ok(res)
    }

    async fn gen_audio(&self, text: &str) -> Result<(Input, u64), Error> {
        let res = self
            .json_client
            .post(format!("{OPENAI_API_URL}/audio/speech"))
            .json(&SpeechRequest {
                model: "tts-1".to_string(),
                input: text.to_string(),
                voice: "onyx".to_string(),
            })
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(Error::msg("Failed to generate audio"));
        }

        let bytes = res.bytes().await?;

        let mut input: Input = bytes.clone().into();
        input = input.make_playable_async(&CODEC_REGISTRY, &PROBE).await?;

        let duration = (bytes.len() / 48) as u64;

        if !input.is_playable() {
            return Err(Error::msg("Generated audio is not playable"));
        }

        Ok((input, duration))
    }

    async fn play_audio(&self, input: Input, duration: u64) -> Result<(), Error> {
        let manager = songbird::get(&self.ctx).await.unwrap();

        if let Some(handler_lock) = manager.get(self.guild_id.clone()) {
            let mut handler = handler_lock.lock().await;
            let _ = handler.play_input(input).set_volume(0.5);

            if let Ok(mut last_reply) = self.controller.last_reply.lock() {
                *last_reply = Some(VoiceReply {
                    timestamp: Utc::now(),
                    duration: Duration::milliseconds(duration as i64),
                });
            }
        }

        Ok(())
    }
}

#[async_trait]
impl EventHandler for Receiver {
    async fn act(&self, ctx: &Ctx<'_>) -> Option<Event> {
        match ctx {
            Ctx::SpeakingStateUpdate(Speaking {
                delay,
                speaking: _,
                ssrc,
                user_id: Some(user_id),
                ..
            }) => {
                info!("{:?} speaking; delay: {:#?}", ssrc, delay);

                self.controller.known_ssrcs.insert(*ssrc, *user_id);

                self.controller.accumulator.entry(*ssrc).or_insert(Slice {
                    user_id: user_id.0,
                    bytes: Vec::new(),
                    timestamp: Utc::now(),
                    first_discord_timestamp: 0,
                    last_discord_timestamp: 0,
                });
            }
            Ctx::VoiceTick(tick) => {
                let speaking = tick.speaking.len();
                // info!("VoiceTick: speaking: {}", speaking);
                let last_tick_was_empty =
                    self.controller.last_tick_was_empty.load(Ordering::SeqCst);
                // info!("\tlast_tick_was_empty: {}", last_tick_was_empty);

                if speaking == 0 && !last_tick_was_empty {
                    info!("VoiceTick: store/process section...");
                    self.controller
                        .last_tick_was_empty
                        .store(true, Ordering::SeqCst);

                    for mut slice in self.controller.accumulator.iter_mut() {
                        if slice.bytes.is_empty() {
                            continue;
                        }
                        if let Err(e) = self.process(&mut slice).await {
                            info!("Processing error: {:?}", e);
                        }
                    }
                } else if speaking != 0 {
                    // info!("VoiceTick: speaking... length: {}", speaking);
                    self.controller
                        .last_tick_was_empty
                        .store(false, Ordering::SeqCst);

                    for (ssrc, data) in &tick.speaking {
                        // data.packet.
                        if let Some(decoded_voice) = data.decoded_voice.as_ref() {
                            let mut bytes = decoded_voice.to_owned();

                            let discord_timestamp = compute_discord_timestamp(data.packet.as_ref().unwrap().rtp().get_timestamp());

                            // if let Some(packet) = &data.packet {
                            //     let rtp = packet.rtp();
                            //     // let rtcp
                            //     println!(
                            //         "\t{ssrc}: packet seq {} ts {}",
                            //         rtp.get_sequence().0,
                            //         rtp.get_timestamp().0
                            //     );
                            // } else {
                            //     println!("\t{ssrc}: Missed packet");
                            // }

                            if let Some(mut slice) = self.controller.accumulator.get_mut(&ssrc) {
                                info!("VoiceTick: appending bytes [{ssrc}]...");

                                let diff_from_last_timestamp = discord_timestamp - slice.last_discord_timestamp;
                                warn!("diff_from_last_timestamp: [{:?}]", diff_from_last_timestamp);

                                slice.bytes.append(&mut bytes);
                                if slice.first_discord_timestamp == 0 {
                                    info!("\tdiscord_timestamp 0; setting timestamps [{ssrc}]");
                                    slice.first_discord_timestamp = discord_timestamp;
                                    slice.timestamp = Utc::now();
                                }
                                slice.last_discord_timestamp = discord_timestamp;
                            } else if let Some(user_id) = self.controller.known_ssrcs.get(ssrc) {
                                info!("VoiceTick: creating new slice [{ssrc}]...");
                                // let discord_timestamp = data.packet.as_ref().unwrap().rtp().get_timestamp().0.into();
                                self.controller.accumulator.insert(
                                    *ssrc,
                                    Slice {
                                        user_id: user_id.0,
                                        bytes,
                                        timestamp: Utc::now(),
                                        first_discord_timestamp: discord_timestamp,
                                        last_discord_timestamp: discord_timestamp,
                                    },
                                );
                            }
                        }
                    }
                }
            }
            Ctx::ClientDisconnect(ClientDisconnect { user_id, .. }) => {
                info!("{:?} disconnected", user_id);
            }
            Ctx::RtcpPacket(data) => {
                // An event which fires for every received rtcp packet,
                // containing the call statistics and reporting information.
                // info!("RTCP packet received: {:?} offset: [{:?}] end_pad: [{:?}]", data.rtcp(), data.payload_offset, data.payload_end_pad);
                let rtcp = data.rtcp();
                match rtcp {
                    rtcp::RtcpPacket::SenderReport(s) => {
                        info!("RTCP packet received: {:?} offset: [{:?}] end_pad: [{:?}]", data.rtcp(), data.payload_offset, data.payload_end_pad);
                        info!("SenderReport: {:?}", s);
                    }
                    rtcp::RtcpPacket::ReceiverReport(s) => {
                        // info!("RTCP packet received: {:?} offset: [{:?}] end_pad: [{:?}]", data.rtcp(), data.payload_offset, data.payload_end_pad);
                        // info!("ReceiverReport: {:?}", s);
                    }
                    rtcp::RtcpPacket::KnownType(_) => {
                        info!("RTCP packet received: {:?} offset: [{:?}] end_pad: [{:?}]", data.rtcp(), data.payload_offset, data.payload_end_pad);
                        info!("KnownType: {:?}", rtcp);
                    }
                    _ => {
                        info!("RTCP packet received: {:?} offset: [{:?}] end_pad: [{:?}]", data.rtcp(), data.payload_offset, data.payload_end_pad);
                        info!("Unknown RTCP packet: {:?}", rtcp);
                    }
                }
                // data.rtcp().
            },
            // Ctx::RtpPacket(packet) => {
            //     // An event which fires for every received audio packet,
            //     // containing the decoded data.
            //     let rtp = packet.rtp();
            //     let pt = rtp.get_payload_type();
            //     let pt_as_string = match pt {
            //         songbird::packet::rtp::RtpType::Pcmu => "pcmu",
            //             songbird::packet::rtp::RtpType::Gsm => "gsm",
            //             songbird::packet::rtp::RtpType::G723 => "g723",
            //             songbird::packet::rtp::RtpType::Dvi4(u) => "dvi4({})",
            //             songbird::packet::rtp::RtpType::Lpc => "lpc",
            //             songbird::packet::rtp::RtpType::Pcma => "pcma",
            //             songbird::packet::rtp::RtpType::G722 => "g722",
            //             songbird::packet::rtp::RtpType::L16Stereo => "l16_stereo",
            //             songbird::packet::rtp::RtpType::L16Mono => "l16_mono",
            //             songbird::packet::rtp::RtpType::Qcelp => "qcelp",
            //             songbird::packet::rtp::RtpType::Cn => "cn",
            //             songbird::packet::rtp::RtpType::Mpa => "mpa",
            //             songbird::packet::rtp::RtpType::G728 => "g728",
            //             songbird::packet::rtp::RtpType::G729 => "g729",
            //             songbird::packet::rtp::RtpType::CelB => "celb",
            //             songbird::packet::rtp::RtpType::Jpeg => "jpeg",
            //             songbird::packet::rtp::RtpType::Nv => "nv",
            //             songbird::packet::rtp::RtpType::H261 => "h261",
            //             songbird::packet::rtp::RtpType::Mpv => "mpv",
            //             songbird::packet::rtp::RtpType::Mp2t => "mp2t",
            //             songbird::packet::rtp::RtpType::H263 => "h263",
            //             // songbird::packet::rtp::RtpType::Dynamic(u) => format!("dynamic({})", u),
            //             // songbird::packet::rtp::RtpType::Reserved(u) => format!("reserved({})", u),
            //             // songbird::packet::rtp::RtpType::Unassigned(u) => format!("unassigned({})", u),
            //             // songbird::packet::rtp::RtpType::Illegal(u) => format!("illegal({})", u),
            //             _ => "unknown",
            //         };
                    
            //     info!(
            //         "Packet from SSRC [{}], sequence [{}], timestamp [{}] -- [{}]B long, CSRC count: [{}] CSRCs: [{:?}], PT: [{}], ext: [{}], marker: [{}]",
            //         rtp.get_ssrc(),
            //         rtp.get_sequence().0,
            //         rtp.get_timestamp().0,
            //         rtp.payload().len(),
            //         rtp.get_csrc_count(),
            //         rtp.get_csrc_list(),
            //         pt_as_string,
            //         rtp.get_extension(),
            //         rtp.get_marker(),
            //     );
            // },
            _ => {}
        }

        None
    }
}

impl Bot {
    pub async fn join_channel(&self, ctx: &Context, msg: &Message) {
        if msg.guild_id.is_none() {
            self.send_msg(&ctx, &msg, "no").await;
            return;
        }

        let (guild_id, channel_id) = {
            let guild = msg.guild(&ctx.cache).unwrap();
            let channel_id = guild
                .voice_states
                .get(&msg.author.id)
                .and_then(|voice_state| voice_state.channel_id);
            (guild.id, channel_id)
        };

        if let Some(channel_id) = channel_id {
            info!("Joining voice channel");

            ctx.set_activity(Some(ActivityData::listening("youtube music")));

            let manager = songbird::get(&ctx).await.unwrap().clone();

            if let Ok(handler_lock) = manager.join(guild_id, channel_id).await {
                let mut handler = handler_lock.lock().await;

                let receiver = Receiver::new(ctx.to_owned(), guild_id.into());

                handler.add_global_event(CoreEvent::SpeakingStateUpdate.into(), receiver.clone());
                handler.add_global_event(CoreEvent::VoiceTick.into(), receiver.clone());
                handler.add_global_event(CoreEvent::RtpPacket.into(), receiver.clone());
                handler.add_global_event(CoreEvent::RtcpPacket.into(), receiver.clone());
                handler.add_global_event(CoreEvent::ClientDisconnect.into(), receiver);
            }
        }
    }

    pub async fn leave_channel(&self, ctx: &Context, msg: &Message) {
        if msg.guild_id.is_none() {
            self.send_msg(&ctx, &msg, "no").await;
            return;
        }

        ctx.set_activity(None);

        let guild_id = msg.guild_id.unwrap();
        let manager = songbird::get(&ctx).await.unwrap().clone();

        if manager.get(guild_id).is_some() {
            info!("Leaving voice channel");
            let _ = manager.remove(guild_id).await;
        }

        // let _ = fs::remove_dir_all("cache");
    }
}
