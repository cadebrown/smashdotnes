#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["mido==1.3.3"]
# ///

import argparse
import hashlib
import sys
import tomllib
import urllib.request
from collections import defaultdict
from dataclasses import dataclass, replace
from pathlib import Path

from mido import MidiFile


ROOT = Path(__file__).resolve().parents[3]
LOCK_PATH = ROOT / "nessy/assets/music_sources.lock"
OUTPUT_PATH = ROOT / "nessy/assets/music.toml"
PITCH_NAMES = ("C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B")

TICKS_PER_BEAT = 960
NTSC_FRAME_RATE_NUM = 3_579_546
NTSC_FRAME_RATE_DEN = 59_561
TEMPO_MILLIBPM = 154_113
SOURCE_CYCLE_TICKS = 128 * TICKS_PER_BEAT
SONG_START_TICK = 0
LOOP_START_TICK = 18 * TICKS_PER_BEAT + TICKS_PER_BEAT // 2
SONG_END_TICK = LOOP_START_TICK + SOURCE_CYCLE_TICKS
DRUM_RELEASE_TICKS = TICKS_PER_BEAT // 8


@dataclass(frozen=True)
class Note:
    start: int
    end: int
    pitch: int
    velocity: int


@dataclass(frozen=True)
class Event:
    tick: int
    pulse1: int | None
    pulse2: int | None
    triangle: int | None
    noise: str | None
    pulse1_channel: int | None
    pulse2_channel: int | None
    triangle_channel: int | None


@dataclass(frozen=True)
class Pattern:
    id: str
    ticks: list[int]
    pulse1: list[str]
    pulse2: list[str]
    triangle: list[str]
    noise: list[str]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Rebuild the NES Menu 1 port from the locked Melee MIDI"
    )
    parser.add_argument(
        "--offline",
        action="store_true",
        help="fail instead of downloading a missing source",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail if the generated score differs from music.toml",
    )
    return parser.parse_args()


def load_source() -> tuple[dict[str, object], dict[str, object], Path]:
    lock = tomllib.loads(LOCK_PATH.read_text())
    sources = lock.get("source", [])
    if len(sources) != 1 or sources[0].get("id") != "ssbm_menu_1":
        raise RuntimeError(f"{LOCK_PATH} must define exactly one ssbm_menu_1 source")
    source = sources[0]
    reference = lock.get("reference")
    if not isinstance(reference, dict):
        raise RuntimeError(f"{LOCK_PATH} must define the original-audio reference")
    if reference.get("tempo_millibpm") != TEMPO_MILLIBPM:
        raise RuntimeError("locked reference tempo does not match the reducer")
    cache = ROOT / str(source["cache"])
    return source, reference, cache


def ensure_source(source: dict[str, object], cache: Path, offline: bool) -> bytes:
    if not cache.exists():
        if offline:
            raise RuntimeError(f"locked MIDI is missing in offline mode: {cache}")
        cache.parent.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(str(source["url"])) as response:
            cache.write_bytes(response.read())
    payload = cache.read_bytes()
    actual_md5 = hashlib.md5(payload).hexdigest()
    actual_sha256 = hashlib.sha256(payload).hexdigest()
    if actual_md5 != source["md5"] or actual_sha256 != source["sha256"]:
        raise RuntimeError(
            f"locked MIDI digest mismatch: md5={actual_md5} sha256={actual_sha256}"
        )
    return payload


def collect_notes(midi: MidiFile) -> tuple[dict[int, list[Note]], dict[int, list[int]]]:
    notes: dict[int, list[Note]] = defaultdict(list)
    drums: dict[int, list[int]] = defaultdict(list)
    active: dict[tuple[int, int], list[tuple[int, int]]] = defaultdict(list)
    for track in midi.tracks:
        tick = 0
        for message in track:
            tick += message.time
            if message.type == "note_on" and message.velocity > 0:
                active[(message.channel, message.note)].append((tick, message.velocity))
                if message.channel == 9:
                    drums[tick].append(message.note)
            elif message.type == "note_off" or (
                message.type == "note_on" and message.velocity == 0
            ):
                key = (message.channel, message.note)
                if active[key]:
                    start, velocity = active[key].pop(0)
                    notes[message.channel].append(
                        Note(start, tick, message.note, velocity)
                    )
    for channel_notes in notes.values():
        channel_notes.sort(key=lambda note: (note.start, note.pitch))
    return notes, drums


def source_beat(tick: int) -> float:
    return ((tick - TICKS_PER_BEAT) % SOURCE_CYCLE_TICKS) / TICKS_PER_BEAT + 1


def channel_priority(tick: int, voice: str) -> tuple[int, ...]:
    beat = source_beat(tick)
    if voice == "lead":
        if beat < 33:
            return (0, 3)
        if beat < 65:
            return (5, 0, 3)
        if beat < 81:
            return (3, 13, 0, 5)
        if beat < 97:
            return (11, 5, 3, 0, 8)
        if beat < 113:
            return (10, 0, 5)
        return (3, 7, 0, 5)
    if voice == "harmony":
        if beat < 33:
            return (10, 2, 4)
        if beat < 65:
            return (10, 0, 2, 4)
        if beat < 97:
            return (14, 5, 0)
        if beat < 113:
            return (0, 5, 4)
        return (0, 5, 4)
    if beat < 97:
        return (1, 12, 4, 6)
    if beat < 113:
        return (4, 6, 1, 12)
    return (1, 4, 6, 12)


def active_notes(notes: dict[int, list[Note]], channel: int, tick: int) -> list[Note]:
    return [note for note in notes[channel] if note.start <= tick < note.end]


def fit_pitch(pitch: int, low: int, high: int) -> int:
    while pitch > high:
        pitch -= 12
    while pitch < low:
        pitch += 12
    return pitch


def pick_voice(
    notes: dict[int, list[Note]], tick: int, voice: str, lead: int | None = None
) -> tuple[int, int] | None:
    low, high = {
        "lead": (60, 84),
        "harmony": (48, 76),
        "bass": (36, 52),
    }[voice]
    for channel in channel_priority(tick, voice):
        pitches = {
            fit_pitch(note.pitch, low, high)
            for note in active_notes(notes, channel, tick)
        }
        if not pitches:
            continue
        if voice == "bass":
            return min(pitches), channel
        if voice == "harmony" and lead is not None:
            below = [pitch for pitch in pitches if pitch <= lead - 3]
            if below:
                return max(below), channel
            for pitch in sorted(pitches, reverse=True):
                while pitch > lead - 3:
                    pitch -= 12
                if pitch >= low:
                    return pitch, channel
        return max(pitches), channel
    return None


def drum_token(pitches: list[int]) -> str:
    pitch_set = set(pitches)
    if pitch_set & {35, 36}:
        return "kick"
    if pitch_set & {37, 38, 39, 40}:
        return "snare"
    return "hat"


def drum_controls(drums: dict[int, list[int]]) -> dict[int, str]:
    controls: dict[int, str] = {}
    releases: set[int] = set()
    current_release = -1
    for tick in sorted(t for t in drums if SONG_START_TICK <= t < SONG_END_TICK):
        release = min(tick + DRUM_RELEASE_TICKS, SONG_END_TICK)
        releases.add(release)
        controls[tick] = drum_token(drums[tick])
    for tick in sorted(set(controls) | releases):
        if tick in controls:
            current_release = min(tick + DRUM_RELEASE_TICKS, SONG_END_TICK)
        elif tick == current_release:
            controls[tick] = "rest"
    return controls


def target_frame(tick: int) -> int:
    numerator = tick * 60 * 1000 * NTSC_FRAME_RATE_NUM
    denominator = TICKS_PER_BEAT * TEMPO_MILLIBPM * NTSC_FRAME_RATE_DEN
    return (numerator + denominator // 2) // denominator


def render_events(
    notes: dict[int, list[Note]], drums: dict[int, list[int]]
) -> list[Event]:
    controls = drum_controls(drums)
    boundaries = {SONG_START_TICK, LOOP_START_TICK, SONG_END_TICK}
    for channel, channel_notes in notes.items():
        if channel == 9:
            continue
        for note in channel_notes:
            if SONG_START_TICK < note.start < SONG_END_TICK:
                boundaries.add(note.start)
            if SONG_START_TICK < note.end < SONG_END_TICK:
                boundaries.add(note.end)
    boundaries.update(controls)
    for cycle_start in (TICKS_PER_BEAT, TICKS_PER_BEAT + SOURCE_CYCLE_TICKS):
        for beat in (1, 33, 65, 81, 97, 113, 129):
            tick = cycle_start + (beat - 1) * TICKS_PER_BEAT
            if SONG_START_TICK < tick < SONG_END_TICK:
                boundaries.add(tick)

    events = []
    previous = (None, None, None)
    for tick in sorted(boundaries):
        if tick >= SONG_END_TICK:
            continue
        lead_pick = pick_voice(notes, tick, "lead")
        lead = lead_pick[0] if lead_pick is not None else None
        harmony_pick = pick_voice(notes, tick, "harmony", lead)
        harmony = harmony_pick[0] if harmony_pick is not None else None
        bass_pick = pick_voice(notes, tick, "bass")
        bass = bass_pick[0] if bass_pick is not None else None
        state = (lead, harmony, bass)
        control = controls.get(tick)
        if (
            state != previous
            or control is not None
            or tick
            in {
                SONG_START_TICK,
                LOOP_START_TICK,
            }
        ):
            events.append(Event(
                tick,
                lead,
                harmony,
                bass,
                control,
                lead_pick[1] if lead_pick is not None else None,
                harmony_pick[1] if harmony_pick is not None else None,
                bass_pick[1] if bass_pick is not None else None,
            ))
            previous = state
    return events


def smooth_transient_channel_fallbacks(events: list[Event]) -> tuple[list[Event], int]:
    smoothed = list(events)
    substitutions = 0
    voices = (
        ("pulse1", "pulse1_channel"),
        ("pulse2", "pulse2_channel"),
    )

    for pitch_field, channel_field in voices:
        index = 1
        while index + 1 < len(smoothed):
            previous_pitch = getattr(smoothed[index - 1], pitch_field)
            previous_channel = getattr(smoothed[index - 1], channel_field)
            pitch = getattr(smoothed[index], pitch_field)
            channel = getattr(smoothed[index], channel_field)
            end = index + 1
            while end < len(smoothed):
                if (
                    getattr(smoothed[end], pitch_field) != pitch
                    or getattr(smoothed[end], channel_field) != channel
                ):
                    break
                end += 1

            if (
                end < len(smoothed)
                and pitch is not None
                and previous_pitch is not None
                and channel is not None
                and previous_channel is not None
                and channel != previous_channel
                and getattr(smoothed[end], channel_field) == previous_channel
                and target_frame(smoothed[end].tick) - target_frame(smoothed[index].tick) == 1
            ):
                for event_index in range(index, end):
                    smoothed[event_index] = replace(
                        smoothed[event_index],
                        **{
                            pitch_field: previous_pitch,
                            channel_field: previous_channel,
                        },
                    )
                substitutions += 1
            index = end

    return smoothed, substitutions


def group_segment(events: list[Event], start: int, end: int) -> list[Event]:
    selected = [event for event in events if start <= event.tick < end]
    if not selected or selected[0].tick != start:
        raise RuntimeError(f"segment {start}:{end} has no boundary event")
    grouped: list[tuple[int, Event]] = []
    noise_priority = {None: 0, "rest": 1, "hat": 2, "snare": 3, "kick": 4}
    for event in selected:
        frame = target_frame(event.tick)
        if grouped and grouped[-1][0] == frame:
            previous = grouped[-1][1]
            noise = max((previous.noise, event.noise), key=noise_priority.__getitem__)
            grouped[-1] = (
                frame,
                Event(
                    previous.tick,
                    event.pulse1,
                    event.pulse2,
                    event.triangle,
                    noise,
                    event.pulse1_channel,
                    event.pulse2_channel,
                    event.triangle_channel,
                ),
            )
        else:
            grouped.append((frame, event))
    return [event for _, event in grouped]


def pitch_name(pitch: int) -> str:
    return f"{PITCH_NAMES[pitch % 12]}{pitch // 12 - 1}"


def voice_token(pitch: int | None, previous: int | None, first: bool) -> str:
    if pitch is None:
        return "-" if first or previous is not None else "="
    if not first and pitch == previous:
        return "="
    return pitch_name(pitch)


def build_pattern(
    pattern_id: str, events: list[Event], start: int, end: int
) -> Pattern:
    grouped = group_segment(events, start, end)
    ticks = [
        (grouped[index + 1].tick if index + 1 < len(grouped) else end) - event.tick
        for index, event in enumerate(grouped)
    ]
    if min(ticks) <= 0 or sum(ticks) != end - start:
        raise RuntimeError(f"invalid event timing in {pattern_id}")
    if any(
        target_frame(event.tick + ticks[index]) == target_frame(event.tick)
        for index, event in enumerate(grouped)
    ):
        raise RuntimeError(f"{pattern_id} contains a sub-frame event")

    pulse1 = []
    pulse2 = []
    triangle = []
    noise = []
    previous = (None, None, None)
    for index, event in enumerate(grouped):
        first = index == 0
        pulse1.append(voice_token(event.pulse1, previous[0], first))
        pulse2.append(voice_token(event.pulse2, previous[1], first))
        triangle.append(voice_token(event.triangle, previous[2], first))
        noise.append(event.noise or ("rest" if first else "hold"))
        previous = (event.pulse1, event.pulse2, event.triangle)
    return Pattern(pattern_id, ticks, pulse1, pulse2, triangle, noise)


def toml_array(values: list[object]) -> str:
    rows = []
    for start in range(0, len(values), 12):
        rendered = []
        for value in values[start : start + 12]:
            rendered.append(f'"{value}"' if isinstance(value, str) else str(value))
        rows.append("  " + ", ".join(rendered) + ",")
    return "[\n" + "\n".join(rows) + "\n]"


def build_score(
    source: dict[str, object], reference: dict[str, object], midi: MidiFile
) -> str:
    if midi.type != 0 or midi.ticks_per_beat != TICKS_PER_BEAT or len(midi.tracks) != 1:
        raise RuntimeError(
            f"unexpected MIDI structure: type={midi.type} tracks={len(midi.tracks)} "
            f"division={midi.ticks_per_beat}"
        )
    tempos = [
        message.tempo
        for track in midi.tracks
        for message in track
        if message.type == "set_tempo"
    ]
    if tempos != [387_097]:
        raise RuntimeError(f"unexpected source tempo map: {tempos}")

    notes, drums = collect_notes(midi)
    events, substitutions = smooth_transient_channel_fallbacks(render_events(notes, drums))
    if substitutions != 6:
        raise RuntimeError(
            f"smoothed {substitutions} transient channel fallbacks, expected 6"
        )
    patterns = [
        build_pattern("intro", events, SONG_START_TICK, LOOP_START_TICK),
        build_pattern("loop", events, LOOP_START_TICK, SONG_END_TICK),
    ]
    steps = sum(len(pattern.ticks) for pattern in patterns)
    loop_frames = target_frame(SONG_END_TICK) - target_frame(LOOP_START_TICK)
    if loop_frames != 2995:
        raise RuntimeError(
            f"Menu 1 loop resolves to {loop_frames}, expected 2995 frames"
        )

    lines = [
        "# Generated by nessy/tools/music/rebuild_music.py from the locked VGMusic MIDI.",
        f"# {source['title']}; sequenced by {source['sequenced_by']}.",
        f"# Source SHA-256: {source['sha256']}",
        "# Retimed to the original 1,594,676-sample loop: 154.113 BPM, 2,995 NTSC frames.",
        "# The event reduction preserves the source lead handoffs, counterline, bass, and drums.",
        "# Six one-frame channel fallbacks are held through to prevent transition blips.",
        "",
        "[timing]",
        f"ticks_per_beat = {TICKS_PER_BEAT}",
        f"ntsc_frame_rate_num = {NTSC_FRAME_RATE_NUM}",
        f"ntsc_frame_rate_den = {NTSC_FRAME_RATE_DEN}",
        "",
        "[[song]]",
        'id = "menu"',
        "loop_order = 1",
        f"tempo_millibpm = {TEMPO_MILLIBPM}",
        "pulse1_duty = 2",
        "pulse1_volume = 11",
        "pulse2_duty = 1",
        "pulse2_volume = 7",
        'order = ["intro", "loop"]',
        "",
    ]
    for pattern in patterns:
        lines.extend(
            [
                "[[song.pattern]]",
                f'id = "{pattern.id}"',
                f"ticks = {toml_array(pattern.ticks)}",
                f"pulse1 = {toml_array(pattern.pulse1)}",
                f"pulse2 = {toml_array(pattern.pulse2)}",
                f"triangle = {toml_array(pattern.triangle)}",
                f"noise = {toml_array(pattern.noise)}",
                "",
            ]
        )
    score = "\n".join(lines)
    print(
        f"Menu 1: {steps} events, {target_frame(SONG_END_TICK)} total frames, "
        f"{loop_frames} loop frames ({reference['tempo_millibpm'] / 1000:.3f} BPM), "
        f"{substitutions} transient fallbacks smoothed"
    )
    return score


def main() -> int:
    args = parse_args()
    source, reference, cache = load_source()
    ensure_source(source, cache, args.offline)
    score = build_score(source, reference, MidiFile(cache))
    if args.check:
        if not OUTPUT_PATH.exists() or OUTPUT_PATH.read_text() != score:
            print(
                f"FAIL: {OUTPUT_PATH} is not synchronized with the locked MIDI",
                file=sys.stderr,
            )
            return 1
        print(f"PASS: {OUTPUT_PATH} matches {source['sha256']}")
        return 0
    OUTPUT_PATH.write_text(score)
    print(f"Wrote {OUTPUT_PATH}")
    print(f"Source: {cache} ({source['sha256']})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
