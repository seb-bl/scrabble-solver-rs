
pub mod word_finder;
pub mod restrictionner;
pub mod letter_set;
pub mod score;

use fst::Set;

use typed_arena::Arena;
use dashmap::DashMap;

use super::Letter;
use super::Square;
use super::LetterTile;

use super::{
    Direction,
    Placement,
    Position,
    Move,
};
use super::{Board, Table};

use letter_set::LetterSet;
use word_finder::TrayRemaining;
use super::{
    BoardBonus,
    LetterScoring,
};
use super::Rules;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RestrictedSquare {
    Empty(LetterSet),
    Filled(LetterTile),
}

#[derive(Clone)]
pub struct ConstrainedBoard {
    /// The direction in which the constraints have been collected (perp of what they will be used for)
    dir: Direction,
    table: Vec<Vec<RestrictedSquare>>,
}

impl ConstrainedBoard {
    pub fn build(board_table: &Table<Square>, dir: Direction, dictionary: &Set<impl AsRef<[u8]>>) -> Self {
        let mut table = vec![vec![RestrictedSquare::Empty(LetterSet::empty()); 15]; 15];
        
        let mut start = Placement(Position { row: 0, col: 0 }, dir);
        
        for i in 0..15 {
            let mut buf = [Square::Empty; 15];
            let mut head = start.clone();
            for j in 0..15 {
                buf[j] = board_table.get(head.0).unwrap().clone();
                head = head.next();
            }
            
            let mut bur_restr = [RestrictedSquare::Empty(LetterSet::empty()); 15];
            restrictionner::find_restrictions(&buf[..], &mut bur_restr[..], dictionary);
            
            for j in 0..15 {
                table[j][i] = bur_restr[j];
            }
            
            start = start.perp().next().perp();
        }
        
        Self {
            table,
            dir,
        }
    }
    
    fn is_empty(&self) -> bool {
        for i in 0..15 {
            for j in 0..15 {
                if let RestrictedSquare::Filled(_) = self.table[i][j] {
                    return false
                }
            }
        }
        true
    }
    
    pub fn explore(&self) -> impl Iterator<Item=(
        Placement,
        &[RestrictedSquare],
        usize,
    )> {
        let mut line = Placement(Position { row: 0, col: 0 }, self.dir.perp());
        let is_empty = self.is_empty();
        std::iter::from_fn(move || {
            if line.0[self.dir] >= 15 {
                return None
            }
            let mut head = line.clone();
            let line_slice = &self.table[line.0[self.dir]][..];
            line = line.perp().next().perp();
            Some(std::iter::from_fn(move || {
                while head.0[self.dir.perp()] < 15 {
                    // skip the square just after a tile
                    match line_slice.get(head.back().0[self.dir.perp()]) {
                        None | Some(RestrictedSquare::Empty(_)) => break,
                        Some(RestrictedSquare::Filled(_)) => {
                            head = head.next();
                            continue
                        },
                    }
                }
                
                if head.0[self.dir.perp()] >= 15 {
                    return None
                }
                
                let sub_slice = &line_slice[head.0[self.dir.perp()]..];
                let place = head.clone();
                head = head.next();
                
                // find minimum length to be attached: first square that is filled or that have constraints (some perpendicular word)
                let mut end = place.clone();
                while end.0[self.dir.perp()] < 15 {
                    if is_empty && end.0 == (Position { row: 7, col: 7 }) {
                        break
                    }
                    match line_slice[end.0[self.dir.perp()]] {
                        RestrictedSquare::Empty(letter_set) if letter_set.is_any() => {
                            end = end.next();
                            continue
                        },
                        _ => break
                    }
                }
                
                if end.0[self.dir.perp()] == 15 { // The line is empty
                    return None
                }
                
                Some((
                    place,
                    sub_slice,
                    (end.0[self.dir.perp()] - place.0[self.dir.perp()] + 1).max(2),
                ))
            }))
        }).flatten()
    }
}

// The algo here is actually more exponential than it needs to be.
// It will branch at every letter that can be replaced by a wildcard, and check
// that wildcards have been used at the end of the word, and discard move that
// have not used all the needed wildcards.
//
// By exploring fewer branches (avoiding exploring branches that will be
// eventually discarded), we could reduce the the complexity (still exponential,
// but more like a binomial). This would avoid branching when all instances of a
// letter need a wildcard, which is, I think, the most common case

// This is good enough because we don't have a lot of wildcards (but this solving this problem could mean twice faster even for 1 or 2 wildcards)

/// A word can be played on the same place with a different assigment of wildcards.
/// As using more wildcards will only gives a lower score, we only generate moves
/// with the minimum number of wildcards required for the word (by using them as letters we don't have).
pub fn generate_moves_for_word<'a>(
    current_place: Placement,
    first: Option<(Placement, LetterTile, usize)>, // usize is n_steps since last
    others: &mut Vec<(usize, LetterTile)>,
    sub_slice: &[RestrictedSquare], word: &[u8],
    wildcards_intersection: &[bool], wildcards_missing: &[u8; 256],
    moves: &mut Vec<Move<'a>>, arenas_mov: &'a Arena<(usize, LetterTile)>,
) {
    if word.len() == 0 {
        if wildcards_missing.iter().any(|&c| c != 0) {
            // there are wilcards that have not been played that should have
            return
        }
        // base case
        let (first_place, first_letter, _) = first.unwrap();
        if others.len() == 0 {
            moves.push(Move::SingleLetter(first_place.0, first_letter));
        } else {
            moves.push(Move::MultiLetters(first_place, first_letter, arenas_mov.alloc_extend(others.iter().cloned())));
        }
    } else {
        // move to next
        let next_place = current_place.next();
        let next_sub_slice = &sub_slice[1..];
        let next_word = &word[1..];
        let next_wildcards_intersection = &wildcards_intersection[1..];
        
        if let RestrictedSquare::Empty(_) = sub_slice[0] {
            if !wildcards_intersection[0] && wildcards_missing[word[0] as usize] > 0 {
                // extra path for using the wildcards
                let mut wildcards_missing_new = wildcards_missing.clone();
                wildcards_missing_new[word[0] as usize] -= 1;
                
                let (first, was_first) = if let Some((p_first, l_first, n_step)) = first {
                    others.push((n_step, LetterTile::Wildcard));
                    (Some((p_first, l_first, 0)), false)
                } else {
                    (Some((current_place, LetterTile::Wildcard, 0)), true)
                };
                
                generate_moves_for_word(
                    next_place, first, others,
                    next_sub_slice, next_word,
                    next_wildcards_intersection, &wildcards_missing_new,
                    moves, arenas_mov
                );
                
                if !was_first {
                    others.pop();
                }
            }
            
            let tile = if wildcards_intersection[0] {
                LetterTile::Wildcard
            } else {
                LetterTile::Letter(Letter(word[0]))
            };
            
            let (first, was_first) = if let Some((p_first, l_first, n_step)) = first {
                others.push((n_step, tile));
                (Some((p_first, l_first, 0)), false)
            } else {
                (Some((current_place, tile, 0)), true)
            };
            
            generate_moves_for_word(
                next_place, first, others,
                next_sub_slice, next_word,
                next_wildcards_intersection, wildcards_missing,
                moves, arenas_mov
            );
            
            if !was_first {
                others.pop();
            }
            
        } else {
            // we didn't play anything here
            let mut first = first;
            if let Some((_, _, n_step)) = &mut first {
                *n_step += 1;
            }
            generate_moves_for_word(
                next_place, first, others,
                next_sub_slice, next_word,
                next_wildcards_intersection, wildcards_missing,
                moves, arenas_mov
            )
        }
    }
}

pub mod arenas {
    use typed_arena::Arena;
    use std::sync::Mutex;
    
    pub struct Arenas<T>(Mutex<Vec<Box<Arena<T>>>>);
    
    impl<T> Arenas<T> {
        pub fn new() -> Arenas<T> {
            Arenas(Mutex::new(vec![]))
        }
        pub fn new_arena(&self) -> &Arena<T> {
            // NOTE: the limited api of Arenas does not allow to drop the boxes
            // or access the arenas by any other way than from the result of this function
            // before the end of the lifetime bound to the returned reference
                
            let a = Box::new(Arena::new());
            let b = Box::into_raw(a);
            let mut inner = self.0.lock().unwrap();
            inner.push(unsafe { Box::from_raw(b) });
            
            unsafe { b.as_ref() }.unwrap()
        }
        pub fn into_inner(self) -> Vec<Box<Arena<T>>> {
            self.0.into_inner().unwrap()
        }
    }
}
use arenas::Arenas;

pub enum StrList<'a> {
    Empty,
    Elem(&'a str, &'a Self)
}

impl<'a> StrList<'a> {
    pub const EMPTY_LIST: StrList<'static> = StrList::Empty;
    
    pub fn to_vec(&self) -> Vec<&'a str> {
        let mut acc = vec![];
        
        let mut current = self;
        
        while let StrList::Elem(elem, list) = current {
            current = list;
            acc.push(*elem);
        }
        
        acc
    }
}

impl<'a> std::fmt::Debug for StrList<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.to_vec().fmt(f)
    }
}

pub struct EvaluationResult<'a> {
    pub words: dashmap::ReadOnlyView<Move<'a>, &'a StrList<'a>>,
    pub score: Vec<(Move<'a>, u32)>,
}

/// Evaluate all the words that can be played on the board, and the score with the associated move
///
/// Provides the score of each move (the returned vec is sorted), and the words created by each move
pub fn evaluate<'a>(
    arenas_str: &'a Arenas<u8>,
    arenas_str_list: &'a Arenas<StrList<'a>>,
    arenas_mov: &'a Arenas<(usize, LetterTile)>,
    tray: &TrayRemaining, board: &Board,
    rules: Rules<impl LetterScoring, impl BoardBonus, impl AsRef<[u8]> + Sync>,
) -> EvaluationResult<'a> {
    use fst::{IntoStreamer, Streamer};
    use word_finder::ScrabbleAutomata;
    use rayon::prelude::*;
    
    let dictionary = &rules.dictionary;
    
    let prepared_h = ConstrainedBoard::build(&board.letter_table, Direction::Vertical, &dictionary);
    let prepared_v = ConstrainedBoard::build(&board.letter_table, Direction::Horizontal, &dictionary);
    
    let found_moves: DashMap<Move, &StrList> = DashMap::new();
    
    prepared_v.explore().chain(prepared_h.explore())
    .collect::<Vec<_>>()
    .into_par_iter()
    .for_each_init(
        || (arenas_str.new_arena(), arenas_mov.new_arena(), arenas_str_list.new_arena()),
        |(arena_str, arena_mov, arena_str_list), (
            placement,
            restr_slice,
            min_len,
        )| {
            let automaton = ScrabbleAutomata {
                line: restr_slice,
                tray: tray.clone(),
                min_len,
                wildcards_have_multi_meaning: rules.wildcards_have_multi_meaning,
            };
            
            let mut wildcards_intersection = vec![];
            let mut moves = Vec::new();
            let mut others = Vec::new();
            
            let mut matches = dictionary.search_with_state(automaton).into_stream();
            while let Some((word, state)) = matches.next() {
                use word_finder::{WildcardAssignment, WildcardAssignmentList};
                
                wildcards_intersection.clear();
                wildcards_intersection.extend(std::iter::repeat(false).take(word.len()));
                let mut wildcards_missing = [0; 256];
                
                let mut wild_list = state.unwrap().wildcards;
                while let WildcardAssignmentList::Elem(wild_assignment, rem) = wild_list {
                    wild_list = (*rem).clone();
                    match wild_assignment {
                        WildcardAssignment::Intersection(i) => wildcards_intersection[i] = true,
                        WildcardAssignment::MissingLetter(l) => wildcards_missing[l as usize] += 1,
                    }
                }
                
                others.clear();
                
                generate_moves_for_word(
                    /*current_place*/ placement,
                    /*first*/ None,
                    /*others*/ &mut others,
                    /*sub_slice*/ restr_slice, word,
                    &wildcards_intersection[..], &wildcards_missing,
                    &mut moves, arena_mov
                );
                
                for a_move in moves.drain(..) {
                    let str_on_arena = arena_str.alloc_str(std::str::from_utf8(word).unwrap());
                    
                    let mut entry = found_moves.entry(a_move).or_insert(&StrList::EMPTY_LIST); //.push(str_on_arena)
                    
                    let list = arena_str_list.alloc(StrList::Elem(str_on_arena, entry.value()));
                    
                    *entry.value_mut() = list;
                }
            }
        }
    );
    
    let mut score_per_move = vec![];
    
    let found_moves = found_moves.into_read_only();
    
    found_moves.keys()
    .collect::<Vec<_>>()
    .into_par_iter()
    .map(|a_move| {
        let mut score = score::naive_score(
            &board.value_table,
            &a_move,
            &rules.score_rules,
        );
        // extra bonus of 50 points if we used 7 letters
        if let Move::MultiLetters(_, _, others) = a_move {
            if 1 + others.len() == 7 {
                score += 50
            }
        }
        (a_move.clone(), score)
    }).collect_into_vec(&mut score_per_move);
    
    score_per_move.par_sort_unstable_by_key(|(_, s)| *s);
    
    EvaluationResult {
        words: found_moves,
        score: score_per_move,
    }
}
