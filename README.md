# PS: issues / brainstorming
- Missing voice recordings...
  - What
    - Music bot
    - Justin's audio part of the time... why?
  - What to do
    - Review voice.rs changes - diff_from_last_timestamp was crashing if < 0; I did a quick fix during game to stop crashing but needs review. May have led to inaccurate timestamps or missed recordings?
    - Potentially need to change "tick that looks like continuous voice from before" flushing
    - known_ssrcs - voice data that comes in from a NOT known ssrc is skipped. Instead, let's still record it - just change the filename if necessary
      - Log data about this
- Logs too noisy / not useful enough
  - Separate log for levels - i.e. verbose.log for VoiceTick that's repeated thousands of time; regular log for more important data
- Keep track of who's talking - real time?
  - Would be awesome to have a little websockets page with realtime data about who's talking. We already get packet updates on server every 20ms - perfect amount of time to not deal w/ timing myself. Update state with list of all current speakers and display in real time preview
  - Could add other useful data on this page too
  - Also add inputs to bot - join channel, send message...
- Have bot join voice channel without me needing to send public message
  - Need a way to trigger this - from a REST or websocket listener
  - And/or auto join specific channel on startup
    - CLI parameter - channel id?
    

# Adam (Discord Bot)

Models (OpenAI): gpt-3.5-turbo, whisper-1

## Features

- Messaging
  - Reply detection
  - Rate limiting
- Comprehensive logging
- Music
  - YouTube search
  - Queue controls
- Voice
  - Live transcriptions
  - Transcription-based replies
  - Text to speech
  - Music controls

## Development

#### Requirements

For local development:

- rust
- ffmpeg
- opus
- yt-dlp

OR

For Dockerized development:

- docker
- docker-compose

Create a `.env` file from `.env.example`, then tweak `src/cfg.rs` to your needs.

Running:

```sh
# Locally
cargo run

# Using Docker
docker-compose up
```

### Fine-tuning

#### Requirements

- `python@^3.11`
- `poetry`

Create a new file `model/<name>.jsonl` and update the path in `model/tune.py`.
Alternatively, update `model/train.jsonl` directly.

To queue up a fine-tuning job on OpenAI:

```sh
cd model
poetry shell
poetry install
poetry run python tune.py
```

---

[License](https://github.com/drewxs/adam-bot/blob/main/LICENSE)
