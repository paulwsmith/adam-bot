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
    
    last_timestamp_millis = files[0][1]
    last_discord_timestamp = files[0][2]
    last_now_timestamp = files[0][3]


    first_timestamp_millis = files[0][1]
    first_discord_timestamp = files[0][2]
    first_now_timestamp = files[0][3]
    
    # last_end_time = earliest_timestamp
    initial_silence_offset = last_timestamp_millis - earliest_timestamp
    combined = AudioSegment.silent(duration=initial_silence_offset, frame_rate=12000)  # Start with an empty segment
    
    # last_end_time = last_discord_timestamp - initial_silence_offset


    for filename, timestamp_millis, discord_timestamp, now_timestamp in files:
        # print("timestamp:", timestamp)
        audio = AudioSegment.from_wav(os.path.join(directory, filename))
        # print(f"audio frame_rate: [{audio.frame_rate}]")

        previous_combined_length = len(combined)
        segment_length = len(audio)


        offset_timestamp_millis = timestamp_millis - first_timestamp_millis
        offset_discord_timestamp = discord_timestamp - first_discord_timestamp
        offset_now_timestamp = now_timestamp - first_now_timestamp


        print(f"\ttimestamp_millis: [{timestamp_millis}] discord_timestamp: [{discord_timestamp}] now_timestamp: [{now_timestamp}] length in ms: [{len(audio)}]")
        print(f"\t\tdiffs: millis: [{offset_timestamp_millis - previous_combined_length}] discord: [{offset_discord_timestamp - previous_combined_length}] now: [{offset_now_timestamp - previous_combined_length}]")
        

        # millis_silence_duration = offset_timestamp_millis - previous_combined_length
        # discord_silence_duration = offset_discord_timestamp - previous_combined_length
        # now_silence_duration = offset_now_timestamp - previous_combined_length

        millis_silence_duration = timestamp_millis - last_timestamp_millis
        discord_silence_duration = discord_timestamp - last_discord_timestamp
        now_silence_duration = now_timestamp - last_now_timestamp

        print(f"\t\tdiffs2: millis: [{millis_silence_duration}] discord: [{discord_silence_duration}] now: [{now_silence_duration}]")

        # silence_duration = offset_discord_timestamp - previous_combined_length
        silence_duration = discord_silence_duration

        # if millis_silence_duration >= 0 and millis_silence_duration < silence_duration and now_silence_duration > 0 and now_silence_duration < silence_duration:
        if millis_silence_duration >= 0 and millis_silence_duration < silence_duration and millis_silence_duration / silence_duration < 0.5:
            print(f"!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!")
            print(f"\t\tweird silence! going with back up from millis: [{millis_silence_duration}]")
            print(f"!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!")
            silence_duration = millis_silence_duration

        # silence_duration = last_end_time - timestamp
        print(f"\tsilence_duration: [{silence_duration}]")
        if silence_duration > 0:
            silence = AudioSegment.silent(duration=silence_duration, frame_rate=int(audio.frame_rate/4))
            # silence = AudioSegment.silent(duration=silence_duration)
            combined += silence
            # print(f"\t\tmeasured silence length: [{len(silence)}] frame_rate: [{silence.frame_rate}] duration: [{silence.duration_seconds}]")
        combined += audio
        # last_end_time = discord_timestamp + len(audio)

        last_timestamp_millis = timestamp_millis + segment_length
        last_discord_timestamp = discord_timestamp + segment_length
        last_now_timestamp = now_timestamp + segment_length

        print(f"\tdone with iteration. length of combined: [{len(combined)}]")
        # print("len(audio)", len(audio))
        # print("last_end_time", last_end_time)
    # Export combined audio for each speaker
    combined.export(f"{directory}/output/combined_speaker_{speaker_id}.wav", format="wav")
