# Passacaglia[^1]

A command-line tool which generates music as [Lilypond files](https://lilypond.org/), which can be processed into sheet music, and as MIDI files to be played with an electronic synthesizer.

Passacaglia has a variety of configuration options, selected by command-line options and with presets (named after the versions they were introduced in), including:

- the rhythm of the `--harmony`

- the `--tempo` in beats per minute

- the `--min-len` and `--max-len` of typical generated notes, in sixteenth notes

- the `--harmony-base` and `--melody-base` pitches (half-steps above or below middle C)

- how `--steady` the melody's rhythms are

- how frequently rhythms should lengthen and split notes (`--stutter`)

- `--gravity`, `--drag`, and `--nudge`, which control how pitches are generated, loosely based on a physics simulation in which a force acts on the current pitch in a random direction.

Passacaglia is dual-licensed under the MPL-2.0 and CC-BY-SA-4.0.

## How it works

Passacaglia's algorithms determine two parameters independently: rhythm and ideal pitch.

The rhythm parameter determines on which sixteenth-note tick the algorithm should end the current melodic note and start a new one. This happens based on an internal progress tracker, which fills towards 1 by an amount equal to the reciprocal of a cosine function (with a cycle length of `--steady` measures) that varies between the `--min-len` and `--max-len` parameters (also measured in sixteenth notes). The algorithm typically starts notes when the progress exceeds 1, but this can be delayed or preempted with a probability equal to the `--stutter` parameter.

The ideal pitch begins at the `--melody-base`. The pitch's *velocity* (increase or decrease) is randomly modified at each sixtheenth-note tick by `--nudge` half-steps per tick, with the intention of creating scales and arpeggios. In order to keep the ideal pitch in a controlled range, it is also influenced by `--gravity`, which exerts a constant force (which increases, rather than decreasing, with distance) towards the `--melody-base`, causing oscillations, and `--drag`, which reduces velocity carried over from previous ticks in order to tame oscillations.

When the rhythm determines that a note should begin, it is created with a pitch based on the ideal pitch: if the note begins just before a beat, then the ideal pitch itself is used; otherwise, the nearest chord to the ideal pitch is, with ties broken randomly.

The harmony does not vary significantly over a piece: it repeats a chord progression with predetermined notes, and only minor rhythmic customization is possible via the `--harmony` argument. The `--harmony-base` argument is the lowest pitch that the harmony plays.

## Organization

Passacaglia is currently contained in one single file; it has only one meaningful stage. While generating the melody and saving it in some representative format would be ideal, Passacaglia currently ensures that the generated Lilypond and MIDI files match by generating a random seed and re-seeding the PRNG each time. This seed can be found in the generated Lilypond file.

Uniformity between the two is also assured by using the `WriteMusic` trait to implement the music-generation algorithms only once, rather than once per backend. Since notes and chords are handled very differently by the two backends, the trait contains separate functions for the two, and, since Lilypond contains a repetition facility whereas MIDI does not, there is also such a method in the trait, which can either provide textual context (in the case of Lilypond) for a section or make it be generated several times.

The MIDI format itself is very interesting: time signatures have a field to specify how metronome ticks relate to quarter notes and another to specify the number of 32nd notes per quarter (no, I'm not sure why one would change that from 8); tempo is indicated as microseconds per beat, rather than beats per minute, to better fit computer timing systems; and different MIDI tracks within a file can be either ignored, played simultaneously, or interpreted as different, sequential songs. Passacaglia follows Lilypond's convention and plays different tracks simultaneously.

Since I released Passacaglia several months ago and have since found different parameters I prefer, the CLI argument parser takes most options as `Option`s and overlays them onto different presets (defaulting to one that replicates the original behavior).

## Limitations

Passacaglia's key, chord progressions, harmonic expression, and time signature are currently frozen; I'm open to adding flexibility but it would complicate formatting and I'd need to figure out an input format.

In terms of musical quality, it's decent: the syncopation is engaging and sometimes produces some really neat bits, but the algorithm doesn't have any way to approach an ending, and in fact is very likely to stop just when I'd expect one. There also isn't any variation in dynamics yet; if I do add some, I'll probably have it emphasize highs and lows.

## Changelog

`1.2.0`
- Breaking: Added `--force` parameter, which is now required to overwrite a `.ly` file.

- Added additional harmonies: "up-octaves", "down-octaves", "mirror", "triples", "quarter-chords".

- Added direct MIDI output through the `-m`/`--midi <FILE>` option, with configurable `--volume`.

- Harmonies are now represented in code, rather than Lilypond fragments which are grouped together. This unfortunately degrades the aesthetics, but does allow the same methods to be used between Lilypond and MIDI output.

`1.1.0`
- Added configuration, including presets and harmonies.

`1.0.0`
- Initial release.

[^1]: Technically, this should be named Chaconne, as the available harmonies are in C major, not a minor key, and the music is in 4/4 time.
