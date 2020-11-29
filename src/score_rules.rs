
use super::{LetterTile, Letter, Position, BOARD_SIZE};

/// Rules that infuence the score
pub struct ScoreRules<Scoring: LetterScoring, Bonuses: BoardBonus> {
    pub scoring: Scoring,
    pub bonuses: Bonuses,
    /// The amount of bonus points in case of bingo/scrabble (aka all seven letters of the tray are played)
    pub extra_bonus: u32,
}

///
pub trait LetterScoring: Sync {
    fn score_for(&self, letter: &LetterTile) -> u32;
}

pub trait BoardBonus: Sync {
    fn bonus_at(&self, position: Position) -> Bonus;
}

pub struct Bonus {
    pub letter: u32,
    pub word: u32,
}

pub struct EnglishScrabbleScoring;
impl LetterScoring for EnglishScrabbleScoring {
    fn score_for(&self, letter: &LetterTile) -> u32 {
        match letter {
            LetterTile::Wildcard => 0,
            LetterTile::Letter(Letter(l)) => match l {
                b'a' => 1,
                b'b' => 3,
                b'c' => 4,
                b'd' => 2,
                b'e' => 1,
                b'f' => 4,
                b'g' => 2,
                b'h' => 4,
                b'i' => 1,
                b'j' => 8,
                b'k' => 5,
                b'l' => 1,
                b'm' => 3,
                b'n' => 1,
                b'o' => 1,
                b'p' => 3,
                b'q' => 10,
                b'r' => 1,
                b's' => 1,
                b't' => 1,
                b'u' => 1,
                b'v' => 4,
                b'w' => 4,
                b'x' => 8,
                b'y' => 4,
                b'z' => 10,
                _ => {
                    log::warn!("unrecognized letter for score {}", l);
                    0
                },
            },
        }
    }
}
pub struct EnglishWordsWithFriendsScoring;
impl LetterScoring for EnglishWordsWithFriendsScoring {
    fn score_for(&self, letter: &LetterTile) -> u32 {
        match letter {
            LetterTile::Wildcard => 0,
            LetterTile::Letter(Letter(l)) => match l {
                b'a' => 1,
                b'b' => 4,
                b'c' => 4,
                b'd' => 2,
                b'e' => 1,
                b'f' => 4,
                b'g' => 3,
                b'h' => 3,
                b'i' => 1,
                b'j' => 10,
                b'k' => 5,
                b'l' => 2,
                b'm' => 4,
                b'n' => 2,
                b'o' => 1,
                b'p' => 4,
                b'q' => 10,
                b'r' => 1,
                b's' => 1,
                b't' => 1,
                b'u' => 2,
                b'v' => 5,
                b'w' => 4,
                b'x' => 8,
                b'y' => 3,
                b'z' => 10,
                _ => {
                    log::warn!("unrecognized letter for score {}", l);
                    0
                },
            },
        }
    }
}

pub struct ScrabbleBonus;
impl BoardBonus for ScrabbleBonus {
    fn bonus_at(&self, position: Position) -> Bonus {
        let Position { row, col } = position;
        
        if row > BOARD_SIZE || col > BOARD_SIZE {
            log::error!("index for bonus is out of board");
            return Bonus { letter: u32::MAX, word: u32::MAX };
        }
        
        assert_eq!(BOARD_SIZE, 15);
        
        fn fold_half(a: usize) -> usize {
            if a >= 7 {
                a - 7
            } else {
                7 - a
            }
        }
        
        // use the fact the bonus are symetrical from center
        let row = fold_half(row);
        let col = fold_half(col);
        
        match (row, col) {
            | (7, 0) | (0, 7)
            | (7, 7) => Bonus { letter: 1, word: 3 },
            
            | (1, 1)
            | (4, 0) | (0, 4)
            | (5, 1) | (1, 5)
            | (7, 4) | (4, 7) => Bonus { letter: 2, word: 1 },
            
            | (2, 2)
            | (6, 2) | (2, 6) => Bonus { letter: 3, word: 1 },
            
            (row, col) if row == col => Bonus { letter: 1, word: 2 },
            
            _ => Bonus { letter: 1, word: 1 }
        }
    }
}
