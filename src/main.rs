use std::{
    fmt::{Display, Write},
    fs::File,
    path::PathBuf,
};

use clap::Parser;
use rand::random;

/// Generates simple music as Lilypond files.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Number of times to repeat the accompaniment.
    /// Each repetition results in 16 measures of melody.
    #[arg(short, long, default_value_t = 1)]
    repeat: u32,
    /// Path to the Lilypond output
    #[arg(required = true)]
    output: PathBuf,
}

fn main() {
    use std::io::Write;
    let Args { repeat, output } = Args::parse();
    File::create(&output)
        .unwrap()
        .write_all(write_music(repeat).as_bytes())
        .unwrap();
}

fn write_music(repeat: u32) -> String {
    let melody = write_melody(repeat);
    let harmony = write_harmony(repeat);
    format!(
        r#"
\version "2.24.1"
\score {{
\new PianoStaff <<
\new Staff {{
\tempo 4 = {TEMPO}
\clef treble
\key c \major
\time 4/4
{melody}
\fine
}}
\new Staff {{
\clef bass
\key c \major
\time 4/4
{harmony}
\fine
}}
>>
\layout {{}}
\midi {{}}
}}"#
    )
}

fn write_melody(repeat: u32) -> String {
    let mut melody = "{ ".to_string();
    let mut state = MelodyState::new();
    for _ in 0..repeat * REPEAT * CYCLE * MEASURE * STEP {
        state.next_note(&mut melody);
    }
    if state.measure_left() != STEP * MEASURE {
        melody.push('r');
        write_duration(state.measure_left(), &mut melody);
        melody.push(' ');
    }
    melody.push('}');
    melody
}
/// The number of beats per minute.
const TEMPO: u32 = 80;
/// The number of the smallest note generated per beat.
const STEP: u32 = 4;
/// The number of beats per measure.
const MEASURE: u32 = 4;
/// The number of measures for the chord progression to cycle.
const CYCLE: u32 = 4;
/// The number of cycles in the complete harmony.
const REPEAT: u32 = 4;
/// The pitch of the harmony's lowest note.
/// Assumed to be divisible by 12.
const HPITCH: i32 = -12;
/// The pitch of the melody's center.
const MPITCH: i32 = 12;
/// Scales how frequently the speed of notes changes.
const STEADY: f32 = 1.0;
/// How strongly the melody oscillates around its center.
const GRAVITY: f32 = 0.15;
/// How strongly the melody's velocity declines.
const DRAG: f32 = 0.22;
/// The amount of random influence on the melody.
const NUDGE: f32 = 1.5;
/// The amount of random influence on the speed of notes.
const STUTTER: f32 = 0.05;

const HARMONY: [[[i32; MEASURE as usize]; CYCLE as usize]; REPEAT as usize] = [
    [
        // C E G B
        [0, 4, 7, 11],
        // C' A F D
        [12, 9, 5, 2],
        // C E G C'
        [0, 4, 7, 12],
        // D' B G D
        [14, 11, 7, 2],
    ],
    [
        // C E G B
        [0, 4, 7, 11],
        // C' A F D
        [12, 9, 5, 2],
        // C E G C'
        [0, 4, 7, 12],
        // D' B G D
        [14, 11, 7, 2],
    ],
    [
        // E G C' E'
        [4, 7, 12, 16],
        // F' D' C' A
        [17, 14, 12, 9],
        // G B C' E'
        [7, 11, 12, 16],
        // G' F' D' B
        [19, 17, 14, 11],
    ],
    [
        // C' G E C
        [12, 7, 4, 0],
        // D F A C'
        [2, 5, 9, 12],
        // B G E C
        [11, 7, 4, 0],
        // B, D G F
        [-1, 2, 7, 5],
    ],
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Pitch(i32);
impl Pitch {
    fn note(&self) -> Self {
        Self(self.0.rem_euclid(12))
    }
    fn octave(&self) -> i32 {
        self.0.div_euclid(12)
    }
    fn nearest_note(&self, pitches: &[Self]) -> Self {
        assert!(!pitches.is_empty());
        let mut best = 12;
        let mut nearest = *self;
        for pitch in pitches {
            let mut diff = pitch.note().0.abs_diff(self.note().0);
            if diff > 6 {
                diff = 12 - diff;
            }
            if diff == 0 {
                nearest = pitch.note();
                break;
            } else if diff < best {
                nearest = pitch.note();
                best = diff;
            } else if diff == best && random() {
                nearest = pitch.note();
            }
        }
        let mut diff = nearest.note().0 - self.note().0;
        if diff > 6 {
            diff = diff - 12;
        } else if diff < -6 {
            diff = diff + 12;
        }
        Self(self.0 + diff)
    }
    /// Correct for DMaj through AesMaj and fismin through cmin
    fn to_name(&self) -> &'static str {
        match self.note().0 {
            0 => "c",
            1 => "cis",
            2 => "d",
            3 => "ees",
            4 => "e",
            5 => "f",
            6 => "fis",
            7 => "g",
            8 => "aes",
            9 => "a",
            10 => "bes",
            11 => "b",
            _ => unreachable!(),
        }
    }
}
impl Display for Pitch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_name())?;
        let octave = self.octave();
        let adjust_c = if octave >= 0 { '\'' } else { ',' };
        for _ in 0..octave.abs() {
            f.write_char(adjust_c)?;
        }
        Ok(())
    }
}

fn write_duration(duration: u32, out: &mut String) {
    let mut magnitude = duration.ilog2() as i32;
    let mut printed = false;
    loop {
        if duration & (1 << magnitude) != 0 {
            if printed {
                out.push('~');
            }
            out.push_str(match magnitude {
                0 => "16",
                1 => "8",
                2 => "4",
                3 => "2",
                4 => "1",
                _ => panic!(),
            });
            magnitude -= 1;
            while magnitude >= 0 {
                if duration & (1 << magnitude) != 0 {
                    out.push('.');
                    magnitude -= 1;
                } else {
                    break;
                }
            }
            printed = true;
        } else {
            magnitude -= 1;
        }
        if magnitude < 0 {
            break;
        }
    }
}

#[derive(Clone, Copy)]
struct Note {
    pitch: Pitch,
    duration: u32,
}

struct MelodyState {
    pitch: f32,
    velocity: f32,
    progress: f32,
    last_note: u32,
    time: u32,
    note: Note,
}
impl MelodyState {
    fn new() -> Self {
        MelodyState {
            pitch: MPITCH as f32,
            velocity: 0.0,
            progress: 0.0,
            last_note: 0,
            time: 0,
            note: Note {
                pitch: Pitch(MPITCH),
                duration: 1,
            },
        }
    }
    fn measure_left(&self) -> u32 {
        STEP * MEASURE - (self.last_note % (STEP * MEASURE))
    }
    fn next_note(&mut self, out: &mut String) {
        let nudge = if random() { NUDGE } else { -NUDGE };
        let gravity = (self.pitch - MPITCH as f32) * -GRAVITY;
        let velocity = (self.velocity + gravity) * (1.0 - DRAG) + nudge;
        self.pitch += velocity;
        self.velocity = velocity;

        let speed = 3.0 - (self.time as f32 / STEP as f32 / STEADY).cos();
        let speed = speed * speed / 4.0 / STEP as f32;
        self.progress += speed;

        self.time += 1;
        if (self.progress > 1.0 || random::<f32>() < STUTTER) && random::<f32>() > STUTTER {
            self.progress -= 1.0;
            let measure_left = self.measure_left();
            write!(out, "{}", self.note.pitch).unwrap();
            write_duration(self.note.duration.min(measure_left), out);
            if self.note.duration > measure_left {
                out.push('~');
                write_duration(self.note.duration - measure_left, out);
            }
            out.push(' ');
            self.last_note = self.time;
            let mut pitch = Pitch(self.pitch.round() as i32);
            if self.last_note % STEP != STEP - 1 {
                pitch = pitch.nearest_note(&harmony_chord(self.time));
            }
            self.note = Note { pitch, duration: 1 };
        } else {
            self.note.duration += 1;
        }
    }
}

fn harmony_chord(time: u32) -> &'static [Pitch] {
    match (time / STEP / MEASURE) % CYCLE {
        // C E G B
        0 | 2 => &[Pitch(0), Pitch(4), Pitch(7), Pitch(11)],
        // C D F A
        1 => &[Pitch(0), Pitch(2), Pitch(5), Pitch(9)],
        // D F G B
        3 => &[Pitch(2), Pitch(5), Pitch(7), Pitch(11)],
        _ => unreachable!(),
    }
}

fn write_harmony(repeat: u32) -> String {
    let mut result = format!("\\repeat unfold {repeat} {{\n");
    for cycle in &HARMONY {
        for chord in cycle {
            for pitch in chord {
                write!(&mut result, "{}4 ", Pitch(pitch + HPITCH)).unwrap();
            }
            result.push_str("| ");
        }
        result.push('\n');
    }
    result.push('}');
    result
}
