use std::{
    f64::consts::PI,
    fmt::{Display, Write},
    fs::File,
    path::PathBuf,
    process::exit,
};

use clap::Parser;
use midly::{
    num::{u24, u28, u4, u7},
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind,
};
use rand::{thread_rng, Rng, RngCore, SeedableRng};

/// Generates simple music as Lilypond files.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Number of times to repeat the accompaniment
    ///
    /// Each repetition results in 16 measures of melody.
    #[arg(short, long, default_value_t = 1)]
    repeat: u32,
    /// Path to the Lilypond output
    #[arg(required = true)]
    output: PathBuf,
    /// Where to generate (optional) separate MIDI output
    #[arg(short, long)]
    midi: Option<String>,
    /// Whether to write to a file that already exists
    #[arg(long, default_value_t = false)]
    force: bool,
    /// Which default values to use
    ///
    /// Options: "1", "1.1", "1.2"
    #[arg(long, default_value_t = String::from("1"))]
    preset: String,
    /// A seed to use for PRNG
    #[arg(long)]
    seed: Option<u64>,
    /// The harmony preset to use
    ///
    /// Options: "quarter", "up-octaves", "down-octaves", "center-8ths", "mirror", "triples", "quarter-chords"
    #[arg(long)]
    harmony: Option<String>,
    /// The rhythm tendency to use
    ///
    /// "sinusoidal"/"sine": Gradual transitions from short notes to long notes and back
    ///
    /// "saw"/"sawtooth": Quickening notes followed by an abrupt stop
    #[arg(long)]
    rhythm: Option<String>,
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
    ///
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
    /// The force to use in direct MIDI output.
    ///
    /// Must be between 1 and 127.
    #[arg(long)]
    volume: Option<u8>,
}

#[derive(Clone, Debug)]
struct Config {
    /// The harmony preset to use
    harmony: Harmony,
    /// The rhythm tendency to use
    rhythm: Rhythm,
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
    /// The number of times to repeat the harmony.
    repeat: u32,
    /// The RNG seed used.
    seed: u64,
    /// The force to use in direct MIDI output.
    volume: u8,
}
impl Config {
    fn version_1(repeat: u32) -> Config {
        Self {
            harmony: Harmony::Quarter,
            rhythm: Rhythm::Sinusoidal,
            tempo: 80,
            min_len: 1.0,
            max_len: 4.0,
            harmony_base: -12,
            melody_base: 12,
            steady: PI as f32,
            gravity: 0.15,
            drag: 0.22,
            nudge: 1.5,
            stutter: 0.05,
            repeat,
            seed: 0,
            volume: 90,
        }
    }
    fn version_1_1(repeat: u32) -> Config {
        Self {
            harmony: Harmony::CenterEighths,
            min_len: 1.15,
            max_len: 3.5,
            ..Self::version_1(repeat)
        }
    }
    fn version_1_2(repeat: u32) -> Config {
        Self {
            melody_base: 24,
            ..Self::version_1_1(repeat)
        }
    }
}

type SeededRng = rand_xoshiro::Xoshiro256StarStar;

fn main() {
    use std::io::Write;
    let Args {
        repeat,
        output,
        midi,
        seed,
        force,
        preset,
        harmony,
        rhythm,
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
        volume,
    } = Args::parse();
    let mut config = match preset.as_str() {
        "1" => Config::version_1,
        "1.1" => Config::version_1_1,
        "1.2" => Config::version_1_2,
        _ => {
            eprintln!("Unknown preset {preset:?}");
            exit(1);
        }
    }(repeat);
    if let Some(harmony) = harmony.and_then(|h| Harmony::from_str(&h)) {
        config.harmony = harmony;
    }
    if let Some(rhythm) = rhythm.and_then(|r| Rhythm::from_str(&r)) {
        dbg!(&rhythm);
        config.rhythm = rhythm;
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
        stutter,
        volume
    );
    config.seed = seed.unwrap_or_else(|| thread_rng().next_u64());
    if config.harmony_base % 12 != 0 {
        eprintln!("Harmony can only be adjusted by multiples of 12");
        exit(1);
    }
    if !force && output.exists() {
        eprintln!("The output file has already been written to");
        exit(1);
    }
    if let Some(midi_output) = midi {
        let midi = midi_music(&config);
        midi.write_std(File::create(&midi_output).unwrap()).unwrap();
    }
    File::create(&output)
        .unwrap()
        .write_all(write_music(&config).as_bytes())
        .unwrap();
}

fn midi_music(config: &Config) -> Smf<'static> {
    let rng = &mut SeededRng::seed_from_u64(config.seed);
    let mut state = MelodyState::new(config);
    let mut melody = MidiWriter::new(config);
    for _ in 0..config.repeat * REPEAT {
        for _ in 0..CYCLE * MEASURE * STEP {
            state.next_note(rng, &mut melody);
        }
    }
    let mut harmony = MidiWriter::new(config);
    write_harmony(config, &mut harmony);
    make_midi(config, vec![melody.output, harmony.output])
}

fn write_music(config: &Config) -> String {
    let rng = &mut SeededRng::seed_from_u64(config.seed);
    let melody = write_melody(config, rng);
    let mut harmony_writer = LilypondWriter::new();
    write_harmony(config, &mut harmony_writer);
    let harmony = harmony_writer.output;
    let tempo = config.tempo;
    format!(
        r#"\version "2.24.1"
% generated by passacaglia
% {config:?}
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

fn write_melody(config: &Config, rng: &mut SeededRng) -> String {
    let mut state = MelodyState::new(config);
    let mut melody = LilypondWriter::new();
    melody.output = "{ ".to_string();
    for _ in 0..config.repeat * REPEAT {
        for _ in 0..CYCLE * MEASURE * STEP {
            state.next_note(rng, &mut melody);
        }
        melody.push('\n');
    }
    if state.measure_left() != STEP * MEASURE {
        melody.push('r');
        write_duration(state.measure_left(), &mut melody.output);
        melody.push(' ');
    }
    melody.push('}');
    melody.output
}

/// The number of the smallest note generated per beat.
const STEP: u32 = 4;
/// The number of beats per measure.
const MEASURE: u32 = 4;
/// The number of measures for the chord progression to cycle.
const CYCLE: u32 = 4;
/// The number of cycles in the complete harmony.
const REPEAT: u32 = 4;

#[derive(Clone, Debug)]
enum Harmony {
    Quarter,
    UpOctaves,
    DownOctaves,
    CenterEighths,
    Mirror,
    Triples,
    QuarterChords,
}
impl Harmony {
    fn from_str(str: &str) -> Option<Self> {
        match str {
            "quarter" => Some(Harmony::Quarter),
            "up-octaves" => Some(Harmony::UpOctaves),
            "down-octaves" => Some(Harmony::DownOctaves),
            "center-8ths" => Some(Harmony::CenterEighths),
            "mirror" => Some(Harmony::Mirror),
            "triples" => Some(Harmony::Triples),
            "quarter-chords" => Some(Harmony::QuarterChords),
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

#[derive(Clone, Debug)]
enum Rhythm {
    Sinusoidal,
    Sawtooth,
}
impl Rhythm {
    fn from_str(str: &str) -> Option<Self> {
        match str {
            "sine" | "sinusoidal" => Some(Rhythm::Sinusoidal),
            "saw" | "sawtooth" => Some(Rhythm::Sawtooth),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Pitch(i32);
impl Pitch {
    fn note(&self) -> Self {
        Self(self.0.rem_euclid(12))
    }
    fn octave(&self) -> i32 {
        self.0.div_euclid(12)
    }
    fn nearest_note(&self, rng: &mut SeededRng, pitches: &[Self]) -> Self {
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
            } else if diff == best && rng.gen() {
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

trait WriteMusic {
    fn write_note(&mut self, note: Note);
    fn write_chord(&mut self, chord: &[Pitch], duration: u32);
    fn repeat(&mut self, times: u32, inner: impl Fn(&mut Self));
}

struct LilypondWriter {
    measure_left: u32,
    output: String,
}
impl LilypondWriter {
    fn new() -> Self {
        Self {
            measure_left: STEP * MEASURE,
            output: String::new(),
        }
    }
    fn push(&mut self, ch: char) {
        self.output.push(ch);
    }
    fn write_duration(&mut self, duration: u32) {
        write_duration(duration.min(self.measure_left), &mut self.output);
        if duration > self.measure_left {
            self.output.push('~');
            write_duration(duration - self.measure_left, &mut self.output);
            self.measure_left += STEP * MEASURE;
        }
        self.output.push(' ');
        self.measure_left -= duration;
        if self.measure_left == 0 {
            self.measure_left = STEP * MEASURE;
        }
    }
}

impl WriteMusic for LilypondWriter {
    fn write_note(&mut self, Note { pitch, duration }: Note) {
        write!(&mut self.output, "{pitch}").unwrap();
        self.write_duration(duration);
    }
    fn write_chord(&mut self, chord: &[Pitch], duration: u32) {
        self.output.push('<');
        for (i, pitch) in chord.iter().enumerate() {
            write!(&mut self.output, "{pitch}").unwrap();
            if i < chord.len() - 1 {
                self.output.push(' ');
            }
        }
        self.output.push('>');
        self.write_duration(duration);
    }
    fn repeat(&mut self, times: u32, inner: impl Fn(&mut Self)) {
        write!(&mut self.output, "\\repeat unfold {times} {{\n").unwrap();
        inner(self);
        self.output.push('}');
    }
}

struct MidiWriter {
    volume: u7,
    output: Track<'static>,
}
impl MidiWriter {
    fn new(config: &Config) -> Self {
        MidiWriter {
            volume: u7::new(config.volume),
            output: vec![],
        }
    }
}
fn make_midi<'a>(config: &Config, mut tracks: Vec<Track<'a>>) -> Smf<'a> {
    let control = vec![
        TrackEvent {
            delta: u28::new(0),
            // Represents a time signature MEASURE/4
            // 24 times 1/24 of a quarter note is one beat/metronome tick
            // 8 is the number of 32nd notes per quarter
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(MEASURE as u8, 4, 24, 8)),
        },
        TrackEvent {
            delta: u28::new(0),
            // microseconds/beat
            kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(60_000_000 / config.tempo))),
        },
        TrackEvent {
            delta: u28::new(STEP * MEASURE * CYCLE * REPEAT * config.repeat),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ];
    tracks.insert(0, control);
    Smf {
        header: Header {
            format: Format::Parallel,
            timing: Timing::Metrical((STEP as u16).into()),
        },
        tracks,
    }
}
fn pitch_to_midi(Pitch(pitch): Pitch) -> u7 {
    const MIDDLE_C: i32 = 48;
    u7::new((pitch + MIDDLE_C).try_into().unwrap())
}

impl WriteMusic for MidiWriter {
    fn write_note(&mut self, Note { pitch, duration }: Note) {
        self.output.push(TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Midi {
                channel: u4::new(0),
                message: MidiMessage::NoteOn {
                    key: pitch_to_midi(pitch),
                    vel: self.volume,
                },
            },
        });
        self.output.push(TrackEvent {
            delta: u28::new(duration),
            kind: TrackEventKind::Midi {
                channel: u4::new(0),
                message: MidiMessage::NoteOn {
                    key: pitch_to_midi(pitch),
                    vel: u7::new(0),
                },
            },
        });
    }
    fn write_chord(&mut self, chord: &[Pitch], duration: u32) {
        for &pitch in chord {
            self.output.push(TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Midi {
                    channel: u4::new(0),
                    message: MidiMessage::NoteOn {
                        key: pitch_to_midi(pitch),
                        vel: self.volume,
                    },
                },
            });
        }
        let mut delta: Option<u28> = Some(u28::new(duration));
        for &pitch in chord {
            let delta = delta.take().unwrap_or(u28::new(0));
            self.output.push(TrackEvent {
                delta,
                kind: TrackEventKind::Midi {
                    channel: u4::new(0),
                    message: MidiMessage::NoteOn {
                        key: pitch_to_midi(pitch),
                        vel: u7::new(0),
                    },
                },
            });
        }
    }
    fn repeat(&mut self, times: u32, inner: impl Fn(&mut Self)) {
        for _ in 0..times {
            inner(self);
        }
    }
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
    fn next_note(&mut self, rng: &mut SeededRng, out: &mut impl WriteMusic) {
        let nudge = self.config.nudge;
        let nudge = if rng.gen() { nudge } else { -nudge };
        let gravity = (self.pitch - self.config.melody_base as f32) * -self.config.gravity;
        let velocity = (self.velocity + gravity) * (1.0 - self.config.drag) + nudge;
        self.pitch += velocity;
        self.velocity = velocity;

        let med_len: f32 = (self.config.max_len + self.config.min_len) / 2.0;
        let dev_len: f32 = (self.config.max_len - self.config.min_len) / 2.0;
        let clock = self.time as f64 / (STEP * MEASURE) as f64 / self.config.steady as f64;
        // Positive increases time to next note; negative decreases it.
        let add_time = match &self.config.rhythm {
            Rhythm::Sinusoidal => (clock * 2.0 * PI).cos() as f32,
            Rhythm::Sawtooth => 1.0 - 2.0 * (clock as f32 % 1.0),
        };
        let speed = 1.0 / (dev_len * add_time + med_len);
        self.progress += speed;
        self.time += 1;
        if (self.progress > 1.0 || rng.gen::<f32>() < self.config.stutter)
            && rng.gen::<f32>() > self.config.stutter
        {
            self.progress -= 1.0;
            out.write_note(self.note);
            self.last_note = self.time;
            let mut pitch = Pitch(self.pitch.round() as i32);
            if self.last_note % STEP != STEP - 1 {
                pitch = pitch.nearest_note(rng, &harmony_chord(self.time));
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

fn write_harmony(config: &Config, out: &mut impl WriteMusic) {
    let note = |pitch, duration| Note {
        pitch: Pitch(pitch + config.harmony_base),
        duration,
    };
    out.repeat(config.repeat, |out| {
        for cycle in &HARMONY {
            for chord in cycle {
                let [p0, p1, p2, p3] = *chord;
                match config.harmony {
                    Harmony::Quarter => {
                        for &pitch in chord {
                            out.write_note(note(pitch, 4));
                        }
                    }
                    Harmony::UpOctaves => {
                        for &pitch in chord {
                            out.write_note(note(pitch - 12, 2));
                            out.write_note(note(pitch, 2));
                        }
                    }
                    Harmony::DownOctaves => {
                        for &pitch in chord {
                            out.write_note(note(pitch, 2));
                            out.write_note(note(pitch - 12, 2));
                        }
                    }
                    Harmony::CenterEighths => {
                        let harmony = [
                            note(p0, 4),
                            note(p1, 2),
                            note(p2, 2),
                            note(p1, 2),
                            note(p2, 2),
                            note(p3, 4),
                        ];
                        for note in harmony {
                            out.write_note(note);
                        }
                    }
                    Harmony::Mirror => {
                        let harmony = [
                            note(p0, 2),
                            note(p0 - 12, 2),
                            note(p1 - 12, 2),
                            note(p2 - 12, 2),
                            note(p3 - 12, 2),
                            note(p1, 2),
                            note(p2, 2),
                            note(p3, 2),
                        ];
                        for note in harmony {
                            out.write_note(note);
                        }
                    }
                    Harmony::Triples => {
                        let harmony = [
                            note(p0, 1),
                            note(p1, 1),
                            note(p2, 2),
                            note(p0, 1),
                            note(p1, 1),
                            note(p2, 2),
                            note(p1, 1),
                            note(p2, 1),
                            note(p3, 2),
                            note(p3, 4),
                        ];
                        for note in harmony {
                            out.write_note(note);
                        }
                    }
                    Harmony::QuarterChords => {
                        let harmony = [[p0, p1, p2], [p0, p1, p3], [p0, p2, p3], [p1, p2, p3]];
                        for [d0, d1, d2] in harmony {
                            let chord = [
                                Pitch(d0 + config.harmony_base),
                                Pitch(d1 + config.harmony_base),
                                Pitch(d2 + config.harmony_base),
                            ];
                            out.write_chord(&chord, 4);
                        }
                    }
                }
            }
        }
    });
}
