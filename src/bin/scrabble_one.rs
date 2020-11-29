
use fst::{SetBuilder, Set};

use std::fs::File;
use std::io::{
    BufRead,
    BufReader,
};
use std::convert::TryInto;
use std::time::Instant;
use std::collections::HashMap;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug)]
enum FileOrString {
    File(PathBuf),
    String(String),
}

impl FileOrString {
    pub fn read_to_string(self) -> std::io::Result<String> {
        match self {
            Self::String(s) => Ok(s),
            Self::File(f) => std::fs::read_to_string(f),
        }
    }
}

impl<'de> serde::Deserialize<'de> for FileOrString {
    fn deserialize<D>(deserializer: D) -> Result<FileOrString, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Visitor, MapAccess, Error};
        use std::fmt;
        
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum FileKey {
            File,
        }
        
        struct FileOrStringVisitor;
        
        impl<'de> Visitor<'de> for FileOrStringVisitor {
            type Value = FileOrString;
            
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("enum FileOrString")
            }
            
            fn visit_map<V>(self, mut map: V) -> Result<FileOrString, V::Error>
            where
                V: MapAccess<'de>,
            {
                let _: FileKey = map.next_key()?.ok_or(V::Error::missing_field("file"))?;
                let file = map.next_value()?;
                Ok(FileOrString::File(file))
            }
            
            fn visit_str<E>(self, v: &str) -> Result<FileOrString, E> {
                Ok(FileOrString::String(v.to_owned()))
            }
            fn visit_string<E>(self, v: String) -> Result<FileOrString, E> {
                Ok(FileOrString::String(v))
            }
        }
        
        deserializer.deserialize_any(FileOrStringVisitor)
    }
}

#[derive(Debug, serde::Deserialize)]
struct Settings {
    /// The dictionary of words that are allowed to be played.
    ///
    /// Either a `.txt` file with one word per line, or a `.fst` file generated with `make_fst`
    dictionary: PathBuf,
    
    /// The board as a string or the file containing it (more info in `Opt`)
    board: FileOrString,
    
    /// The tray as a string or the file containing it (more info in `Opt`)
    tray: FileOrString,
    
    /// The number of top result shown, not present means all results are shown
    n_shown: Option<usize>,
    
    letter_score: Option<HashMap<char, u32>>,
    
    #[serde(default)]
    wildcards_have_multi_meaning: bool,
    
    #[serde(default = "fifty")]
    extra_bonus: u32,
    
    #[serde(default)]
    position_format: PositionFormat,
    
    #[serde(default)]
    show_each_score: bool,
}

fn fifty() -> u32 { 50 }

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum PositionFormat {
    LetterDigit,
    DigitLetter,
}

impl Default for PositionFormat {
    fn default() -> Self {
        Self::LetterDigit
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "scrabble_one", about = "Evaluate possible moves for a scrabble board")]
struct Opt {
    /// The config file, if not present, looks for `scrabble-config`
    #[structopt(short = "c", long = "config")]
    config: Option<String>,
    
    /// The dictionary of words that are allowed to be played.
    ///
    /// Either a `.txt` file with one word per line, or a `.fst` file generated with `make_fst`
    #[structopt(short = "d", long = "dictionary")]
    dict: Option<String>,
    
    /// The board, where one line in the file corresponds to one row of the board.
    /// Spaces and underscores are interpreted as empty squares, and stars as wildcards
    #[structopt(short = "b", long = "board")]
    board_file: Option<String>,
    
    /// The tray, a string of the letters contained in the tray, where stars are interpreted as wildcards
    #[structopt(short = "t", long = "tray")]
    tray_string: Option<String>,
    
    /// The number of top result shown, not present means all results are shown
    #[structopt(short = "n", long = "number-shown")]
    n_shown: Option<usize>,
}

fn load_config(opt: Opt) -> Result<Settings, config::ConfigError> {
    let mut s = config::Config::new();
    
    if let Some(f) = opt.config {
        s.merge(config::File::with_name(&f))?;
    }
    
    s.merge(config::Environment::new())?;
    
    if let Some(d) = opt.dict {
        s.set("dictionary", d)?;
    }
    if let Some(b) = opt.board_file {
        s.set("board.file", b)?;
    }
    if let Some(t) = opt.tray_string {
        s.set("tray", t)?;
    }
    if let Some(n) = opt.n_shown {
        s.set::<i64>("n_shown", n.try_into().unwrap())?;
    }
    
    s.try_into()
}

fn main() {
    simple_logger::SimpleLogger::from_env().init().unwrap();
    
    let opt = Opt::from_args();
    
    let conf = load_config(opt).expect("config");
    
    let board = conf.board.read_to_string().expect("read board");
    let tray = conf.tray.read_to_string().expect("read tray");
    let n_shown = conf.n_shown;
    let wildcards_have_multi_meaning = conf.wildcards_have_multi_meaning;
    let extra_bonus = conf.extra_bonus;
    let position_format = conf.position_format;
    let show_each_score = conf.show_each_score;
    
    let dict = conf.dictionary;
    
    match dict.extension().and_then(|s| s.to_str()) {
        Some("fst") => {
            let start = Instant::now();
            let data = std::fs::read(dict).expect("reading the words fst file");
            let dictionary = Set::new(data).expect("converting fst file in set");
            log::info!("dictionary loaded in {:?}", Instant::now() - start);
            
            if let Some(letter_score) = conf.letter_score {
                main_with_dict(
                    dictionary,
                    board,
                    tray,
                    n_shown,
                    SimpleLetterScore { map: letter_score },
                    wildcards_have_multi_meaning,
                    extra_bonus,
                    position_format,
                    show_each_score,
                )
            } else {
                main_with_dict(
                    dictionary,
                    board,
                    tray,
                    n_shown,
                    scrabble::score_rules::EnglishScrabbleScoring,
                    wildcards_have_multi_meaning,
                    extra_bonus,
                    position_format,
                    show_each_score,
                )
            }
        },
        Some("txt") => {
            let start = Instant::now();
            let file = BufReader::new(File::open(dict).expect("opening the words list file"));
            let mut words = file.lines().map(|l|
                l.expect("reading line from word list").trim().to_lowercase()
            ).collect::<Vec<_>>();
            log::info!("words loaded in {:?}", Instant::now() - start);
            
            let start = Instant::now();
            words.sort_unstable();
            log::info!("words sorted in {:?}", Instant::now() - start);
            
            let start = Instant::now();
            let mut build = SetBuilder::memory();
            build.extend_iter(words).unwrap();
            let dictionary = build.into_set();
            log::info!("dictionary build in {:?}", Instant::now() - start);
            
            if let Some(letter_score) = conf.letter_score {
                main_with_dict(
                    dictionary,
                    board,
                    tray,
                    n_shown,
                    SimpleLetterScore { map: letter_score },
                    wildcards_have_multi_meaning,
                    extra_bonus,
                    position_format,
                    show_each_score,
                )
            } else {
                main_with_dict(
                    dictionary,
                    board,
                    tray,
                    n_shown,
                    scrabble::score_rules::EnglishScrabbleScoring,
                    wildcards_have_multi_meaning,
                    extra_bonus,
                    position_format,
                    show_each_score,
                )
            }
        },
        _ => {
            panic!("dictionary file is neither .txt of .fst")
        },
    }
}

fn main_with_dict(
    dict: fst::Set<impl AsRef<[u8]> + Sync>,
    board_string: String,
    tray_string: String,
    n_shown: Option<usize>,
    letter_score: impl scrabble::LetterScoring,
    wildcards_have_multi_meaning: bool,
    extra_bonus: u32,
    position_format: PositionFormat,
    show_each_score: bool,
) {
    
    use scrabble::{
        Board,
        Letter,
        LetterTile,
        Position,
        Square,
        solver::{
            arenas::Arenas,
            StrList,
            word_finder::TrayRemaining,
            evaluate,
        },
    };
    
    let start = Instant::now();
    
    // fill tray
    let mut letters = [0u8; 256];
    let mut wild_count = 0;
    
    for byte in tray_string.bytes() {
        if byte.is_ascii_alphabetic() {
            letters[byte.to_ascii_lowercase() as usize] += 1;
        } else if byte == b'*' {
            wild_count += 1;
        } else {
            log::warn!("a byte in the given tray is neither a letter or a wildcard (*): {}", byte);
        }
    }
    
    let tray = TrayRemaining::new(letters, wild_count);
    
    // fill board
    let mut board = Board::empty();
    let file = BufReader::new(board_string.as_bytes());
    file.lines().enumerate().for_each(|(i, line)| {
        let line = line.expect("reading board line");
        line.bytes().enumerate().for_each(|(j, byte)| {
            let (
                letter_tile,
                value_tile,
            ) = if byte.is_ascii_alphabetic() {
                let t = LetterTile::Letter(Letter(byte.to_ascii_lowercase()));
                (t, if byte.is_ascii_uppercase() {LetterTile::Wildcard} else {t})
            } else if byte == b'*' {
                (LetterTile::Wildcard, LetterTile::Wildcard)
            } else if byte == b' ' || byte == b'_' {
                return // leave empty
            } else {
                log::warn!("a byte in the given board is neither a letter, a wildcard (*), or empty ( _): {}", byte);
                return
            };
            board.letter_table.set(Position { row: i, col: j }, Square::Filled(letter_tile));
            board.value_table.set(Position { row: i, col: j }, Square::Filled(value_tile));
        })
    });
    
    log::info!("board info loaded in {:?}", Instant::now() - start);
    
    // evaluate
    
    let arenas_str: Arenas<u8> = Arenas::new();
    let arenas_str_list: Arenas<StrList> = Arenas::new();
    let arenas_mov: Arenas<(usize, LetterTile)> = Arenas::new();
    
    let start = Instant::now();
    
    use scrabble::score_rules::{ScoreRules, ScrabbleBonus};
    use scrabble::Rules;
    
    let scrabble::solver::EvaluationResult {
        words: found_moves,
        score: score_per_move,
    } = evaluate(
        &arenas_str, &arenas_str_list, &arenas_mov,
        &tray, &board,
        Rules {
            score_rules: ScoreRules {
                scoring: letter_score,
                bonuses: ScrabbleBonus,
                extra_bonus,
            },
            wildcards_have_multi_meaning,
            dictionary: dict,
        },
    );
    
    log::info!("scores evaluated in {:?} ({} possible moves)", Instant::now() - start, score_per_move.len());
    
    // print moves
    
    let mut last_score = None;
    if let Some(n) = n_shown {
        for (mov, score) in score_per_move.into_iter().rev().take(n) {
            if !show_each_score && last_score == Some(score) {
                print!("{:>3}  ", " ")
            } else {
                last_score = Some(score);
                print!("{:>3}: ", score)
            }
            println!("{:<23} {:?}", format_move(&mov, position_format), found_moves.get(&mov).unwrap());
        }
    } else {
        for (mov, score) in score_per_move.into_iter().rev() {
            if !show_each_score && last_score == Some(score) {
                print!("{:>3}  ", " ")
            } else {
                last_score = Some(score);
                print!("{:>3}: ", score)
            }
            println!("{:<23} {:?}", format_move(&mov, position_format), found_moves.get(&mov).unwrap());
        }
    }
}

fn format_move(
    mov: &scrabble::Move,
    position_format: PositionFormat,
) -> String {
    use scrabble::{Direction, Move::*};
    match mov {
        SingleLetter(pos, tile) => {
            format!(
                "{},   {}",
                position_format.format(pos),
                tile_to_char(tile)
            )
        },
        MultiLetters(place, first, others) => {
            format!(
                "{} {}, {}",
                position_format.format(&place.0),
                match place.1 {
                    Direction::Horizontal => "→",
                    Direction::Vertical => "↓",
                },
                std::iter::once(tile_to_char(first)).chain(
                    others.iter().map(|(n, tile)|
                        std::iter::repeat('_').take(*n).chain(std::iter::once(tile_to_char(tile)))
                    ).flatten()
                ).collect::<String>(),
            )
        },
    }
}

impl PositionFormat {
    fn format(&self, pos: &scrabble::Position) -> String {
        match self {
            Self::LetterDigit => format!("{:>2}-{:<2}", (b'A' + pos.col as u8) as char, pos.row+1),
            Self::DigitLetter => format!("{:>2}-{:<2}", pos.col+1, (b'A' + pos.row as u8) as char),
        }
    }
}

fn tile_to_char(tile: &scrabble::LetterTile) -> char {
    match tile {
        scrabble::LetterTile::Letter(l) => l.0 as char,
        scrabble::LetterTile::Wildcard => '*',
    }
}

struct SimpleLetterScore {
    map: HashMap<char, u32>,
}

impl scrabble::LetterScoring for SimpleLetterScore {
    fn score_for(&self, letter: &scrabble::LetterTile) -> u32 {
        self.map[&tile_to_char(letter)]
    }
}
