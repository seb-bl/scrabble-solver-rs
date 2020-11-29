
pub mod solver;
pub mod score_rules;

use std::fmt;

pub const BOARD_SIZE: usize = 15;

pub use score_rules::{LetterScoring, BoardBonus, Bonus};
use score_rules::ScoreRules;

/// a set of rules that controls the allowed moves and the score
pub struct Rules<Scoring: LetterScoring, Bonuses: BoardBonus, DictionaryStorage: AsRef<[u8]>> {
    pub score_rules: ScoreRules<Scoring, Bonuses>,
    
    /// Whether a wilcard can be played and used as different letter for the
    /// horizontal and the vertical word in participates in
    ///
    /// This only applies to wildcards in the move being created, wildcards on
    /// the board are always interpreted as signifying anything
    pub wildcards_have_multi_meaning: bool,
    
    /// The words that can be played
    ///
    /// Words already on the board are not checked
    pub dictionary: fst::Set<DictionaryStorage>,
}

// we restrict to use u8 as letters, and u8 to represent the number of identical letters in a tray
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Letter(pub u8);

impl fmt::Display for Letter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
         write!(f, "{}", (self.0 as char).escape_default())
    }
}
impl fmt::Debug for Letter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
         write!(f, "{}", self)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Square {
    Empty,
    Filled(LetterTile),
}

impl Square {
    pub fn tile(&self) -> Option<&LetterTile> {
        match self {
            Square::Filled(tile) => Some(tile),
            Square::Empty => None
        }
    }
    pub fn tile_mut(&mut self) -> Option<&mut LetterTile> {
        match self {
            Square::Filled(tile) => Some(tile),
            Square::Empty => None
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LetterTile {
    Wildcard,
    Letter(Letter),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Direction {
    Vertical,
    Horizontal,
}

impl Direction {
    pub fn perp(self) -> Self {
        match self {
            Self::Vertical => Self::Horizontal,
            Self::Horizontal => Self::Vertical,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl std::ops::Index<Direction> for Position {
    type Output = usize;
    /// The coordinate that changes in that direction
    fn index(&self, dir: Direction) -> &Self::Output {
        match dir {
            Direction::Vertical => &self.row,
            Direction::Horizontal => &self.col,
        }
    }
}

impl std::ops::IndexMut<Direction> for Position {
    /// The coordinate that changes in that direction
    fn index_mut(&mut self, dir: Direction) -> &mut Self::Output {
        match dir {
            Direction::Vertical => &mut self.row,
            Direction::Horizontal => &mut self.col,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Placement(pub Position, pub Direction);

impl Placement {
    pub fn next(mut self) -> Self {
        self.0[self.1] = self.0[self.1].saturating_add(1);
        self
    }
    
    pub fn back(mut self) -> Self {
        self.0[self.1] = self.0[self.1].wrapping_sub(1);
        self
    }
    
    /// A placement at the same position, but different direction
    pub fn perp(self) -> Self {
        Self(self.0, self.1.perp())
    }
    
    /// Tries to find the first position on the line formed by the given `positions`
    ///
    /// None if `positions` is empty or if contains at least 2 positions that are not on the same row/column
    ///
    /// Some(Err(p)) if `positions` contains only `p`
    pub fn find_alignment(positions: impl IntoIterator<Item=Position>) -> Option<Result<Placement, Position>> {
        let mut iter = positions.into_iter();
        let first = iter.next()?;
        let second = loop {
            let tmp = iter.next();
            if tmp != Some(first) {
                break tmp
            }
        };
        let second = if let Some(s) = second { s } else { return Some(Err(first)) };
        let dir = if second.row == first.row {
            Direction::Horizontal
        } else if second.col == first.col {
            Direction::Vertical
        } else {
            return None
        };
        let mut start = first;
        start[dir] = start[dir].min(second[dir]);
        
        while let Some(new) = iter.next() {
            if new[dir.perp()] != start[dir.perp()] {
                return None
            } else {
                start[dir] = start[dir].min(new[dir])
            }
        }
        Some(Ok(Placement(start, dir)))
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Move<'a> {
    SingleLetter(Position, LetterTile),
    MultiLetters(Placement, LetterTile, &'a [(usize, LetterTile)]), // usize is the number of skipped squares
}

#[derive(Debug, Clone)]
pub struct Board {
    pub letter_table: Table<Square>,
    pub value_table: Table<Square>,
}

impl Board {
    pub fn empty() -> Self {
        Self {
            letter_table: Table::fill_with(Square::Empty),
            value_table: Table::fill_with(Square::Empty),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Table<T> {
    squares: Vec<Vec<T>>
}

impl<T> Table<T> {
    pub fn fill_with(el: T) -> Self where T: Clone {
        Self {
            squares: vec![vec![el; BOARD_SIZE]; BOARD_SIZE],
        }
    }
    
    pub fn get(&self, pos: Position) -> Option<&T> {
        self.squares.get(pos.row)?.get(pos.col)
    }
    pub fn get_mut(&mut self, pos: Position) -> Option<&mut T> {
        self.squares.get_mut(pos.row)?.get_mut(pos.col)
    }
    pub fn set(&mut self, pos: Position, val: T) {
        self.squares[pos.row][pos.col] = val
    }
}


#[test]
fn test_alignement() {
    let p1 = Position { row: 3, col: 4 };
    let p2 = Position { row: 4, col: 4 };
    let p3 = Position { row: 8, col: 4 };
    let p4 = Position { row: 3, col: 6 };
    
    assert_eq!(
        Placement::find_alignment(vec![]),
        None,
    );
    
    assert_eq!(
        Placement::find_alignment(vec![p1]),
        Some(Err(p1)),
    );
    
    assert_eq!(
        Placement::find_alignment(vec![p1, p1]),
        Some(Err(p1)),
    );
    
    assert_eq!(
        Placement::find_alignment(vec![p1, p2]),
        Some(Ok(Placement(p1, Direction::Vertical))),
    );
    
    assert_eq!(
        Placement::find_alignment(vec![p2, p1, p3]),
        Some(Ok(Placement(p1, Direction::Vertical))),
    );
    
    assert_eq!(
        Placement::find_alignment(vec![p2, p3, p1]),
        Some(Ok(Placement(p1, Direction::Vertical))),
    );
    
    assert_eq!(
        Placement::find_alignment(vec![p3, p2]),
        Some(Ok(Placement(p2, Direction::Vertical))),
    );
    
    assert_eq!(
        Placement::find_alignment(vec![p1, p4]),
        Some(Ok(Placement(p1, Direction::Horizontal))),
    );
    
    assert_eq!(
        Placement::find_alignment(vec![p2, p4]),
        None,
    );
    
    assert_eq!(
        Placement::find_alignment(vec![p1, p2, p4]),
        None,
    );
}
