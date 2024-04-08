import datetime
import wave
from pydub import AudioSegment
import os
from collections import defaultdict
import re

# Directory containing your .wav files
directory = "/Users/paul/dev/dnd/adam-bot/cache"
ssrc_userid_map_file = directory + "/ssrc_userid_map.txt"
file_date = datetime.datetime.now().strftime('%Y-%m-%dT%H-%M-%S') # for output filename

def parse_ssrc_user_map():
    ssrc_userid_map = {}
    userid_username_map = {}

    try:
        with open(ssrc_userid_map_file, "r") as file:
            for line in file:
                fields = line.strip().split(":")
                if len(fields) >= 2:
                    ssrc = int(fields[0])
                    user_id = int(fields[1])
                    if len(fields) == 3:
                        username = fields[2]
                        userid_username_map[user_id] = username
                    ssrc_userid_map[ssrc] = user_id
    except FileNotFoundError:
        print(f"File '{ssrc_userid_map_file}' not found.")
    except IOError:
        print(f"Error reading file '{ssrc_userid_map_file}'.")

    # Update the SSRC to user ID mappings with usernames
    final_map = {}
    
    for ssrc, user_id in ssrc_userid_map.items():
        if user_id in userid_username_map:
            final_map[ssrc] = userid_username_map[user_id]
            final_map[user_id] = userid_username_map[user_id]
        else:
            final_map[ssrc] = user_id

    return final_map


def get_user_identifier(ssrc, ssrc_userid_map):
    return ssrc_userid_map.get(ssrc, ssrc) or ssrc

# Parse filenames to extract speaker IDs and timestamps
def parse_filename(filename):
    pattern = r"(\d+)_(\d+)_(\d+).wav"
    match = re.match(pattern, filename)
    if match:
        return match.groups()
    return None, None

ssrc_user_map = parse_ssrc_user_map()
earliest_timestamp = 0
# Group files by speaker ID
files_by_speaker = defaultdict(list)
for filename in os.listdir(directory):
    if filename.endswith(".wav"):
        speaker_id, timestamp_millis, discord_timestamp = parse_filename(filename)
        speaker_id = get_user_identifier(int(speaker_id), ssrc_user_map)
        timestamp_millis = int(timestamp_millis)
        discord_timestamp = int(discord_timestamp)
        # now_timestamp = int(now_timestamp)

        if earliest_timestamp == 0 or timestamp_millis < earliest_timestamp:
            earliest_timestamp = timestamp_millis
            print(f"new earliest_timestamp: [{earliest_timestamp}]")
        files_by_speaker[speaker_id].append((filename, timestamp_millis, discord_timestamp))

# exit(0)
# Sort files by timestamp_millis for each speaker
for speaker_id in files_by_speaker:
    files_by_speaker[speaker_id].sort(key=lambda x: x[1])

print("earliest_timestamp", earliest_timestamp)
# print(files_by_speaker)
# exit(0)

# Combine files for each speaker, inserting silence as needed
for speaker_id, files in files_by_speaker.items():
    print("\nspeaker_id:", speaker_id)
    # if speaker_id != "689875413102755882":
        # continue

    
    output_file = f"{directory}/output/{file_date}_speaker_{speaker_id}.wav"

    
    combined = AudioSegment.silent(duration=0, frame_rate=48000)  # Start with an empty segment
    combined = combined.set_channels(2).set_frame_rate(48000).set_sample_width(2)



    wave_data = wave.open(output_file, 'wb')
    wave_data.setnchannels(combined.channels)
    wave_data.setsampwidth(combined.sample_width)
    wave_data.setframerate(combined.frame_rate)
    # For some reason packing the wave header struct with
    # a float in python 2 doesn't throw an exception
    # wave_data.setnframes(int(combined.frame_count()))
    wave_data.writeframesraw(combined._data)



    print(f"combined frame_rate: {combined.frame_rate}, channels: {combined.channels}, sample_width: {combined.sample_width}")


    last_end_time = earliest_timestamp

    initial_timestamp_millis = files[0][1]
    
    last_timestamp_millis = files[0][1]
    # last_discord_timestamp = files[0][2]
    last_discord_timestamp = 0
    # last_now_timestamp = files[0][3]

    last_discord_offset = 0

    for filename, timestamp_millis, discord_timestamp in files:
        print("filename:", filename)
        audio = AudioSegment.from_wav(os.path.join(directory, filename)).set_frame_rate(48000).set_channels(2)
        print(f"audio frame_rate: {audio.frame_rate}, channels: {audio.channels}, sample_width: {audio.sample_width}")

        # discord_offset = timestamp_millis - discord_timestamp
        # print(f"\tOFFSET: [{discord_offset}]")
        # print(f"\t\tdiff from previous offset: [{discord_offset - last_discord_offset}]")
        # last_discord_offset = discord_offset

        segment_length = len(audio)


        

        # print(f"\tlast_end_time: [{last_end_time}] timestamp_millis: [{timestamp_millis}] discord_timestamp: [{discord_timestamp}] now_timestamp: [{now_timestamp}] length in ms: [{len(audio)}]")
        # print(f"\t\tdiffs: millis: [{timestamp_millis - last_timestamp_millis}] discord: [{discord_timestamp - last_discord_timestamp}] now: [{now_timestamp - last_now_timestamp}]")
        
        silence_duration = timestamp_millis - last_end_time
        # silence_duration = (timestamp_millis - initial_timestamp_millis) - (combined.duration_seconds * 1000)
        discord_silence_duration = discord_timestamp - last_discord_timestamp

        silence = AudioSegment.silent(duration=0, frame_rate=48000)

        if discord_silence_duration < 500 and discord_silence_duration >= 0:
            print(f"\t\tdiscord_silence_duration: [{discord_silence_duration}]")
            silence = AudioSegment.silent(duration=discord_silence_duration, frame_rate=48000)
        elif silence_duration > 0:
            print(f"\tsilence_duration: [{silence_duration}]")
            silence = AudioSegment.silent(duration=silence_duration, frame_rate=48000)

        silence = silence.set_channels(2).set_frame_rate(48000).set_sample_width(2)

        print(f"silence frame_rate: {silence.frame_rate}, channels: {silence.channels}, sample_width: {silence.sample_width}")

        # combined += silence + audio
        wave_data.writeframesraw(silence._data)
        wave_data.writeframesraw(audio._data)

        last_end_time = timestamp_millis + len(audio)
        if discord_silence_duration < 250 and discord_silence_duration >= 0:
            last_end_time -= (silence_duration - discord_silence_duration)
            print(f"\t*** subtracted [{silence_duration - discord_silence_duration}] from last_end_time ***")

        last_timestamp_millis = timestamp_millis + segment_length
        last_discord_timestamp = discord_timestamp + segment_length
        # last_now_timestamp = now_timestamp + segment_length

        print(f"\tdone with iteration. new last_end_time: [{last_end_time}] length of combined: [{len(combined)}]")
        # print("len(audio)", len(audio))
        # print("last_end_time", last_end_time)
    # Export combined audio for each speaker

    # combined.export(f"{directory}/output/{file_date}_speaker_{speaker_id}.wav", format="wav")
    wave_data.close()