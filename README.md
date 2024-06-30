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

[^1]: Technically, this should be named Chaconne, as the available harmonies are in C major, not a minor key, and the music is in 4/4 time.
