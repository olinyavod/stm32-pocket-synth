"""Play a MIDI file (or built-in test) through the Pocket Synth.

Usage:
    python play_midi.py                 # plays a built-in Twinkle Twinkle (mono)
    python play_midi.py --list          # lists built-in songs
    python play_midi.py --song tetris   # plays a built-in 2-voice song
    python play_midi.py song.mid        # plays the given file
"""

import sys
import time
import threading
import mido
import mido.backends.rtmidi  # noqa: F401  (force backend on Windows)

PORT_NAME_HINT = "Pocket Synth"


def find_port():
    for name in mido.get_output_names():
        if PORT_NAME_HINT.lower() in name.lower():
            return name
    raise RuntimeError(
        f"No MIDI port matching '{PORT_NAME_HINT}'. "
        f"Available: {mido.get_output_names()}"
    )


# --- Mono test ---

def play_twinkle(out):
    """Twinkle Twinkle Little Star — single-voice test."""
    twinkle = [
        (60, 1), (60, 1), (67, 1), (67, 1), (69, 1), (69, 1), (67, 2),
        (65, 1), (65, 1), (64, 1), (64, 1), (62, 1), (62, 1), (60, 2),
        (67, 1), (67, 1), (65, 1), (65, 1), (64, 1), (64, 1), (62, 2),
        (67, 1), (67, 1), (65, 1), (65, 1), (64, 1), (64, 1), (62, 2),
        (60, 1), (60, 1), (67, 1), (67, 1), (69, 1), (69, 1), (67, 2),
        (65, 1), (65, 1), (64, 1), (64, 1), (62, 1), (62, 1), (60, 2),
    ]
    beat, gap = 0.30, 0.03
    for note, beats in twinkle:
        out.send(mido.Message("note_on", note=note, velocity=100))
        time.sleep(beat * beats - gap)
        out.send(mido.Message("note_off", note=note, velocity=0))
        time.sleep(gap)


# --- Polyphonic two-voice songs ---
# Each entry: (midi_note, duration_in_eighths). note=0 = rest.

# Tetris A-theme (Korobeiniki). 8 bars × 8 eighths.
KOROBEINIKI_MELODY = [
    (76, 2), (71, 1), (72, 1), (74, 2), (72, 1), (71, 1),
    (69, 2), (69, 1), (72, 1), (76, 2), (74, 1), (72, 1),
    (71, 3), (72, 1), (74, 2), (76, 2),
    (72, 2), (69, 2), (69, 2), (0, 2),

    (74, 3), (77, 1), (81, 2), (79, 1), (77, 1),
    (76, 3), (72, 1), (76, 2), (74, 1), (72, 1),
    (71, 2), (71, 1), (72, 1), (74, 2), (76, 2),
    (72, 2), (69, 2), (69, 2), (0, 2),
]
KOROBEINIKI_BASS = [
    (45, 4), (52, 4),   (45, 4), (52, 4),
    (40, 4), (47, 4),   (45, 4), (52, 4),
    (38, 4), (45, 4),   (48, 4), (43, 4),
    (45, 4), (52, 4),   (45, 4), (52, 4),
]

# Super Mario Bros — main theme opening (Koji Kondo). The bass is a simple
# octave-alternating root following the melody key.
MARIO_MELODY = [
    (76, 1), (76, 1), (0, 1), (76, 1), (0, 1), (72, 1), (76, 1), (0, 1),
    (79, 1), (0, 3),                                    (67, 1), (0, 3),
    (72, 1), (0, 2), (67, 1), (0, 2),                   (64, 1), (0, 2),
    (69, 1), (0, 1), (71, 1), (0, 1), (70, 1), (69, 1), (0, 1),
    (67, 1), (76, 1), (79, 1), (81, 1), (0, 1),         (77, 1), (79, 1),
    (0, 1), (76, 1), (0, 1), (72, 1), (74, 1), (71, 1), (0, 2),
]
MARIO_BASS = [
    (48, 8),                                            # C3 drone (bar 1)
    (43, 8),                                            # G2 drone (bar 2)
    (48, 8),                                            # C3 drone (bar 3)
    (45, 4), (43, 4),                                   # A2, G2 (bar 4)
    (53, 4), (48, 4),                                   # F3, C3 (bar 5)
    (50, 4), (43, 4),                                   # D3, G2 (bar 6)
]

# Imperial March (John Williams). The iconic Vader theme — strong march
# rhythm, melody on top + steady bass root motion.
IMPERIAL_MELODY = [
    (67, 2), (67, 2), (67, 2),                          # G G G
    (63, 1), (0, 1), (70, 1), (0, 1),                   # Eb. - Bb -
    (67, 2),                                            # G
    (63, 1), (0, 1), (70, 1), (0, 1),                   # Eb - Bb -
    (67, 4),                                            # G
    (74, 2), (74, 2), (74, 2),                          # D D D
    (75, 1), (0, 1), (70, 1), (0, 1),                   # Eb' - Bb -
    (66, 2),                                            # F#
    (63, 1), (0, 1), (70, 1), (0, 1),                   # Eb - Bb -
    (67, 4),                                            # G
]
IMPERIAL_BASS = [
    (43, 6),                                            # G2
    (39, 2), (46, 2),                                   # Eb2, Bb2
    (43, 2),                                            # G2
    (39, 2), (46, 2),                                   # Eb2, Bb2
    (43, 4),                                            # G2
    (50, 6),                                            # D3
    (51, 2), (46, 2),                                   # Eb3, Bb2
    (42, 2),                                            # F#2
    (39, 2), (46, 2),                                   # Eb2, Bb2
    (43, 4),                                            # G2
]


def play_track(out, track, eighth_seconds, channel):
    gap = 0.015
    for note, eighths in track:
        if eighths <= 0:
            continue
        if note == 0:
            time.sleep(eighth_seconds * eighths)
            continue
        out.send(mido.Message("note_on", note=note, velocity=100, channel=channel))
        time.sleep(eighth_seconds * eighths - gap)
        out.send(mido.Message("note_off", note=note, velocity=0, channel=channel))
        time.sleep(gap)


def play_polyphonic(out, melody, bass, eighth_seconds):
    threads = [
        threading.Thread(target=play_track, args=(out, melody, eighth_seconds, 0)),
        threading.Thread(target=play_track, args=(out, bass, eighth_seconds, 1)),
    ]
    for t in threads:
        t.start()
    for t in threads:
        t.join()


# (melody, bass, eighth-seconds tempo)
SONGS = {
    "tetris":   (KOROBEINIKI_MELODY, KOROBEINIKI_BASS, 0.20),
    "mario":    (MARIO_MELODY,       MARIO_BASS,       0.15),
    "imperial": (IMPERIAL_MELODY,    IMPERIAL_BASS,    0.25),
}


def play_midi_file(out, path):
    print(f"Playing {path}...")
    mid = mido.MidiFile(path)
    print(f"  length: {mid.length:.1f} s, tracks: {len(mid.tracks)}")
    for msg in mid.play():
        if msg.type in ("note_on", "note_off"):
            out.send(msg)
    print("done")


def main():
    args = sys.argv[1:]

    if args == ["--list"]:
        print("Built-in songs:")
        print("  (no args)            Twinkle Twinkle Little Star (mono)")
        for name in SONGS:
            print(f"  --song {name}")
        print("Or pass a path to a .mid file.")
        return

    port = find_port()
    print(f"Opening: {port}")
    with mido.open_output(port) as out:
        out.send(mido.Message("control_change", control=123, value=0))

        if len(args) == 2 and args[0] == "--song":
            name = args[1]
            if name not in SONGS:
                print(f"Unknown song: {name}. Try --list.")
                return
            melody, bass, eighth = SONGS[name]
            print(f"Playing built-in: {name}")
            play_polyphonic(out, melody, bass, eighth)
        elif args == ["--tetris"]:  # backwards compat with our earlier flag
            melody, bass, eighth = SONGS["tetris"]
            print("Playing built-in: tetris")
            play_polyphonic(out, melody, bass, eighth)
        elif len(args) == 1 and not args[0].startswith("--"):
            play_midi_file(out, args[0])
        else:
            print("Playing built-in: Twinkle Twinkle (mono)")
            play_twinkle(out)

        out.send(mido.Message("control_change", control=123, value=0))


if __name__ == "__main__":
    main()
