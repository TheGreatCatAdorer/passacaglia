# Passacaglia[^1]

A command-line tool which generates music as [Lilypond files](https://lilypond.org/), which can produce both sheet music and MIDI output.

Passacaglia has a variety of configuration options, selected by command-line options and with presets (named after the versions they were introduced in), including:

- the rhythm of the `--harmony`

- the `--tempo` in beats per minute

- the `--min-len` and `--max-len` of typical generated notes

- the `--harmony-base` and `--melody-base` pitches

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

## Changelog

`nightly`
- Breaking: Added `--force` parameter, which is now required to overwrite a `.ly` file.

- Added additional harmonies: "up-octaves", "down-octaves", "mirror", "triples", "quarter-chords".

- Added harmonic combinations: `+` for random selection and `*` for sequencing.
  Harmonic combinations only apply to the initial generation: it remains in a `\repeat` block.

`1.1.0`
- Added configuration, including presets and harmonies.

`1.0.0`
- Initial release.

[^1]: Technically, this should be named Chaconne, as the available harmonies are in C major, not a minor key, and the music is in 4/4 time.
