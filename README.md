
# Scrabble Helper

A helper that finds all words that can be played on a scrabble board, and
evaluates their scores.

# Usage

## Board file

The board is given as a text file where each line is a row on the board (
example at `board.txt`).

Tiles on the board are represented as letters in the file. Wildcard letter are
in uppercase, while normal letters are in lowercase.

An empty square can be represented with an underscore or a space.

## Dictionary

The words that can be played are put in a text file (with a `.txt` extension),
one word per line (example at `wwfwordlist.txt`).

## Tray

The letters of the tray are passed directly as argument. A wildcard is
represented with a star `*` (for example `trean*o` has 6 letters and a wildcard)

## Example

To show top 5 moves for the board in `board.txt` with `trean*o` in the tray:

```
cargo run --bin scrabble_one -- --dictionary wwfwordlist.txt --board board.txt -n 5 --tray trean*o
```

# Config

The executable can accept a config to set the arguments, or modify other parameters.

An example can be found in `scrabble-config.yaml`.

Use the config like this:

```
./scrabble_one -c scrabble-config --tray trean*o
```

## `wildcards_have_multi_meaning`

A parameter of the config is `wildcards_have_multi_meaning`. It can be set to
`true` to relax the rule which makes a wilcard represent a fixed single letter
from the moment it is played until the end.

If set to `true`, then a wildcard can have a different meaning for the vertical
word than for the horizontal one, and change during the game. Wildcards on the
board must also be represented as stars `*` instead of uppercase letter.

# Faster dictionary loading

If you enable info logging (`RUST_LOG=info`), you may notice that some time is
spent to prepare the words. This is because the dictionary is turned into a
compressed representation that allows fast browsing with an automaton (thanks
to the [fst](https://crates.io/crates/fst) crate)

The compressed representation can be loaded directly from a file. To create such
a file, use the following command:

```
cargo run --bin make_fst -- --input-list wwfwordlist.txt --output-fst wwfwordlist.fst
```

It can then be passed as dictionary file (with a `.fst` extension):

```
./scrabble_one --dictionary wwfwordlist.fst --board board.txt -n 5 --tray trean*o
```

-----

I got the idea to make such a tool thanks to [this post](https://jamesmcm.github.io/blog/2020/10/11/programming-projects/#scrabble-solver), where antoher tool is presented [scala-scrabble-solver](https://github.com/jamesmcm/scala-scrabble-solver) from which I copied the `wwfwordlist.txt`
