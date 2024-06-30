use std::{
    f32::consts::PI,
    fmt::{Display, Write},
    fs::File,
    path::PathBuf,
    process::exit,
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
    /// Which default values to use
    /// Options: "1", "1.1"
    #[arg(long, default_value_t = String::from("1"))]
    preset: String,
    /// The harmony preset to use
    /// Options: "quarter", "center-8ths"
    #[arg(long)]
    harmony: Option<String>,
    /// The number of beats per minute.
    #[arg(long)]
    tempo: Option<u32>,
    /// The minimum length (in steps) of notes generated (ignoring stutter).
    #[arg(long)]
    min_len: Option<f32>,
    /// The maximum length (in steps) of notes generated (ignoring stutter).
    #[arg(long)]
    max_len: Option<f32>,
    /// The pitch of the harmony's lowest note.
    /// Assumed to be divisible by 12.
    #[arg(long)]
    harmony_base: Option<i32>,
    /// The pitch of the melody's center.
    #[arg(long)]
    melody_base: Option<i32>,
    /// Scales how frequently the speed of notes changes, in measures.
    #[arg(long)]
    steady: Option<f32>,
    /// How strongly the melody oscillates around its center.
    #[arg(long)]
    gravity: Option<f32>,
    /// How strongly the melody's velocity declines.
    #[arg(long)]
    drag: Option<f32>,
    /// The amount of random influence on the melody.
    #[arg(long)]
    nudge: Option<f32>,
    /// The amount of random influence on the speed of notes.
    #[arg(long)]
    stutter: Option<f32>,
}

#[derive(Clone)]
struct Config {
    /// The harmony preset to use
    harmony: Harmony,
    /// The number of beats per minute.
    tempo: u32,
    /// The minimum length (in steps) of notes generated (ignoring stutter).
    min_len: f32,
    /// The maximum length (in steps) of notes generated (ignoring stutter).
    max_len: f32,
    /// The pitch of the harmony's lowest note.
    /// Assumed to be divisible by 12.
    harmony_base: i32,
    /// The pitch of the melody's center.
    melody_base: i32,
    /// Scales how frequently the speed of notes changes, in measures.
    steady: f32,
    /// How strongly the melody oscillates around its center.
    gravity: f32,
    /// How strongly the melody's velocity declines.
    drag: f32,
    /// The amount of random influence on the melody.
    nudge: f32,
    /// The amount of random influence on the speed of notes.
    stutter: f32,
}

const VERSION1: Config = Config {
    harmony: Harmony::Quarter,
    tempo: 80,
    min_len: 1.0,
    max_len: 4.0,
    harmony_base: -12,
    melody_base: 12,
    steady: PI,
    gravity: 0.15,
    drag: 0.22,
    nudge: 1.5,
    stutter: 0.05,
};
const VERSION1_1: Config = Config {
    harmony: Harmony::CenterEighths,
    min_len: 1.15,
    max_len: 3.5,
    ..VERSION1
};

fn main() {
    use std::io::Write;
    let Args {
        repeat,
        output,
        preset,
        harmony,
        tempo,
        min_len,
        max_len,
        harmony_base,
        melody_base,
        steady,
        gravity,
        drag,
        nudge,
        stutter,
    } = Args::parse();
    let mut config = match preset.as_str() {
        "1" => VERSION1.clone(),
        "1.1" => VERSION1_1.clone(),
        _ => {
            eprintln!("Unknown preset {preset:?}");
            exit(1);
        }
    };
    if let Some(harmony) = harmony {
        let Some(harmony) = Harmony::from_str(&harmony) else {
            eprintln!("Unknown harmony {harmony:?}");
            exit(1);
        };
        config.harmony = harmony;
    }
    macro_rules! default {
        ($($field:ident),*) => {
            $(if let Some($field) = $field {
                config.$field = $field;
            })*
        };
    }
    default!(
        tempo,
        min_len,
        max_len,
        harmony_base,
        melody_base,
        steady,
        gravity,
        drag,
        nudge,
        stutter
    );
    if config.harmony_base % 12 != 0 {
        eprintln!("Harmony can only be adjusted by multiples of 12");
        exit(1);
    }
    File::create(&output)
        .unwrap()
        .write_all(write_music(&config, repeat).as_bytes())
        .unwrap();
}

fn write_music(config: &Config, repeat: u32) -> String {
    let melody = write_melody(config, repeat);
    let harmony = write_harmony(config, repeat);
    let tempo = config.tempo;
    format!(
        r#"
\version "2.24.1"
\score {{
\new PianoStaff <<
\new Staff {{
\tempo 4 = {tempo}
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

fn write_melody(config: &Config, repeat: u32) -> String {
    let mut melody = "{ ".to_string();
    let mut state = MelodyState::new(config);
    for _ in 0..repeat * REPEAT {
        for _ in 0..CYCLE * MEASURE * STEP {
            state.next_note(&mut melody);
        }
        melody.push('\n');
    }
    if state.measure_left() != STEP * MEASURE {
        melody.push('r');
        write_duration(state.measure_left(), &mut melody);
        melody.push(' ');
    }
    melody.push('}');
    melody
}

/// The number of the smallest note generated per beat.
const STEP: u32 = 4;
/// The number of beats per measure.
const MEASURE: u32 = 4;
/// The number of measures for the chord progression to cycle.
const CYCLE: u32 = 4;
/// The number of cycles in the complete harmony.
const REPEAT: u32 = 4;

#[derive(Clone)]
enum Harmony {
    Quarter,
    CenterEighths,
}
impl Harmony {
    fn from_str(str: &str) -> Option<Self> {
        match str {
            "quarter" => Some(Harmony::Quarter),
            "center-8ths" => Some(Harmony::CenterEighths),
            _ => None,
        }
    }
}

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

struct MelodyState<'a> {
    pitch: f32,
    velocity: f32,
    progress: f32,
    last_note: u32,
    time: u32,
    note: Note,
    config: &'a Config,
}
impl<'a> MelodyState<'a> {
    fn new(config: &'a Config) -> Self {
        MelodyState {
            pitch: config.melody_base as f32,
            velocity: 0.0,
            progress: 0.0,
            last_note: 0,
            time: 0,
            note: Note {
                pitch: Pitch(config.melody_base),
                duration: 1,
            },
            config,
        }
    }
    fn measure_left(&self) -> u32 {
        STEP * MEASURE - (self.last_note % (STEP * MEASURE))
    }
    fn next_note(&mut self, out: &mut String) {
        let nudge = self.config.nudge;
        let nudge = if random() { nudge } else { -nudge };
        let gravity = (self.pitch - self.config.melody_base as f32) * -self.config.gravity;
        let velocity = (self.velocity + gravity) * (1.0 - self.config.drag) + nudge;
        self.pitch += velocity;
        self.velocity = velocity;

        let med_len: f32 = (self.config.max_len + self.config.min_len) / 2.0;
        let dev_len: f32 = (self.config.max_len - self.config.min_len) / 2.0;
        let clock = self.time as f32 * (2.0 * PI) / (STEP * MEASURE) as f32 / self.config.steady;
        let speed = 1.0 / (dev_len * clock.cos() + med_len);
        self.progress += speed;

        self.time += 1;
        if (self.progress > 1.0 || random::<f32>() < self.config.stutter)
            && random::<f32>() > self.config.stutter
        {
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

fn write_harmony(config: &Config, repeat: u32) -> String {
    let mut result = format!("\\repeat unfold {repeat} {{\n");
    for cycle in &HARMONY {
        for chord in cycle {
            match config.harmony {
                Harmony::Quarter => {
                    for pitch in chord {
                        write!(&mut result, "{}4 ", Pitch(pitch + config.harmony_base)).unwrap();
                    }
                }
                Harmony::CenterEighths => {
                    let [p0, p1, p2, p3] = chord.map(|p| Pitch(p + config.harmony_base));
                    write!(&mut result, "{p0}4 {p1}8 {p2} {p1} {p2} {p3}4 ").unwrap();
                }
            }
            result.push_str("| ");
        }
        result.push('\n');
    }
    result.push('}');
    result
}
