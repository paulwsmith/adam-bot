import datetime
from pydub import AudioSegment
import os
from collections import defaultdict
import re

# Directory containing your .wav files
directory = "/home/paul/dev/epakura/adam-bot/cache"

# Parse filenames to extract speaker IDs and timestamps
def parse_filename(filename):
    pattern = r"(\d+)_(\d+)_(\d+)_(\d+).wav"
    match = re.match(pattern, filename)
    if match:
        return match.groups()
    return None, None

earliest_timestamp = 0
# Group files by speaker ID
files_by_speaker = defaultdict(list)
for filename in os.listdir(directory):
    if filename.endswith(".wav"):
        speaker_id, timestamp_millis, discord_timestamp, now_timestamp = parse_filename(filename)
        timestamp_millis = int(timestamp_millis)
        discord_timestamp = int(discord_timestamp)
        now_timestamp = int(now_timestamp)

        if earliest_timestamp == 0 or timestamp_millis < earliest_timestamp:
            earliest_timestamp = timestamp_millis
        files_by_speaker[speaker_id].append((filename, timestamp_millis, discord_timestamp, now_timestamp))

# Sort files by timestamp_millis for each speaker
for speaker_id in files_by_speaker:
    files_by_speaker[speaker_id].sort(key=lambda x: x[1])

print("earliest_timestamp", earliest_timestamp)
print(files_by_speaker)
# exit(0)

# Combine files for each speaker, inserting silence as needed
for speaker_id, files in files_by_speaker.items():
    print("\nspeaker_id:", speaker_id)
    combined = AudioSegment.silent(duration=0, frame_rate=12000)  # Start with an empty segment
    last_end_time = earliest_timestamp

    initial_timestamp_millis = files[0][1]
    
    last_timestamp_millis = files[0][1]
    # last_discord_timestamp = files[0][2]
    last_discord_timestamp = 0
    last_now_timestamp = files[0][3]

    last_discord_offset = 0

    for filename, timestamp_millis, discord_timestamp, now_timestamp in files:
        # print("timestamp:", timestamp)
        audio = AudioSegment.from_wav(os.path.join(directory, filename))
        discord_offset = timestamp_millis - discord_timestamp
        print(f"\tOFFSET: [{discord_offset}]")
        print(f"\t\tdiff from previous offset: [{discord_offset - last_discord_offset}]")
        last_discord_offset = discord_offset

        segment_length = len(audio)

        # print(f"\tlast_end_time: [{last_end_time}] timestamp_millis: [{timestamp_millis}] discord_timestamp: [{discord_timestamp}] now_timestamp: [{now_timestamp}] length in ms: [{len(audio)}]")
        # print(f"\t\tdiffs: millis: [{timestamp_millis - last_timestamp_millis}] discord: [{discord_timestamp - last_discord_timestamp}] now: [{now_timestamp - last_now_timestamp}]")
        
        silence_duration = timestamp_millis - last_end_time
        # silence_duration = (timestamp_millis - initial_timestamp_millis) - (combined.duration_seconds * 1000)
        discord_silence_duration = discord_timestamp - last_discord_timestamp

        silence = AudioSegment.silent(duration=0, frame_rate=12000)

        if discord_silence_duration < 500:
            print(f"\t\tdiscord_silence_duration: [{discord_silence_duration}]")
            silence = AudioSegment.silent(duration=discord_silence_duration, frame_rate=12000)
        elif silence_duration > 0:
            print(f"\tsilence_duration: [{silence_duration}]")
            silence = AudioSegment.silent(duration=silence_duration, frame_rate=12000)

        combined += silence
        combined += audio

        last_end_time = timestamp_millis + len(audio)
        if discord_silence_duration < 250:
            last_end_time -= (silence_duration - discord_silence_duration)
            print(f"\t*** subtracted [{silence_duration - discord_silence_duration}] from last_end_time ***")

        last_timestamp_millis = timestamp_millis + segment_length
        last_discord_timestamp = discord_timestamp + segment_length
        last_now_timestamp = now_timestamp + segment_length

        print(f"\tdone with iteration. new last_end_time: [{last_end_time}] length of combined: [{len(combined)}]")
        # print("len(audio)", len(audio))
        # print("last_end_time", last_end_time)
    # Export combined audio for each speaker

    combined.export(f"{directory}/output/{datetime.datetime.now().strftime('%Y-%m-%dT%H-%M-%S')}_speaker_{speaker_id}.wav", format="wav")
    # combined.export(f"{directory}/output/combined_speaker_{speaker_id}.wav", format="wav")
