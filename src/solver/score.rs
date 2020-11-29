
use super::{Table, Move, Placement, Direction, Square};
use crate::{LetterScoring, BoardBonus};
use crate::score_rules::ScoreRules;

/// Compute the score of a single move
///
/// This is named naive, as a more efficient method could be implemented by
/// computing parts of score in common with other words only once instead of
/// again for each word
pub fn naive_score(table: &Table<Square>, play: &Move, score_rules: &ScoreRules<impl LetterScoring, impl BoardBonus>) -> u32 {
    let scoring = &score_rules.scoring;
    let bonuses = &score_rules.bonuses;
    
    match play {
        Move::SingleLetter(pos, tile) => {
            let pos = *pos;
            let tile = *tile;
            // count the score of the other letters for the vertical word
            // count the score of the other letters for the horizontal word
            // add twice the score of the letter multiplied by bonus
            // add all, and multiply by bonus
            
            let mut v_score = 0;
            let mut v_place_back = Placement(pos, Direction::Vertical);
            loop {
                v_place_back = v_place_back.back();
                match table.get(v_place_back.0) {
                    Some(Square::Filled(tile)) => {
                        v_score += scoring.score_for(&tile);
                    },
                    _ => break // out of board, or no more letters
                }
            }
            let mut v_place_next = Placement(pos, Direction::Vertical);
            loop {
                v_place_next = v_place_next.next();
                match table.get(v_place_next.0) {
                    Some(Square::Filled(tile)) => {
                        v_score += scoring.score_for(&tile);
                    },
                    _ => break // out of board, or no more letters
                }
            }
            
            let mut h_score = 0;
            let mut h_place_back = Placement(pos, Direction::Horizontal);
            loop {
                h_place_back = h_place_back.back();
                match table.get(h_place_back.0) {
                    Some(Square::Filled(tile)) => {
                        h_score += scoring.score_for(&tile);
                    },
                    _ => break // out of board, or no more letters
                }
            }
            let mut h_place_next = Placement(pos, Direction::Horizontal);
            loop {
                h_place_next = h_place_next.next();
                match table.get(h_place_next.0) {
                    Some(Square::Filled(tile)) => {
                        h_score += scoring.score_for(&tile);
                    },
                    _ => break // out of board, or no more letters
                }
            }
            
            let letter_score = scoring.score_for(&tile);
            
            let bonus = bonuses.bonus_at(pos);
            
            (v_score + h_score + 2 * letter_score * bonus.letter) * bonus.word
        },
        Move::MultiLetters(place, first, others) => {
            let place = *place;
            let first = *first;
            // for each letter, look at a perp word,
            //      if any, compute the score for the other letters of perp word
            //      add the letter multiplied by its bonus
            //      add the full score multiplied by the bonus to the total_perp_score
            
            // compute the score of the word in line
            
            let mut perp_score = 0;
            
            let mut current_place = place.clone();
            let mut current_tile = first;
            let mut others_iter = others.iter().cloned();
            
            loop {
                let mut local_score = 0;
                let mut has_local_word = false;
                let mut local_place_back = Placement(current_place.0, current_place.1.perp());
                loop {
                    local_place_back = local_place_back.back();
                    match table.get(local_place_back.0) {
                        Some(Square::Filled(tile)) => {
                            local_score += scoring.score_for(&tile);
                            has_local_word = true;
                        },
                        _ => break // out of board, or no more letters
                    }
                }
                let mut local_place_next = Placement(current_place.0, current_place.1.perp());
                loop {
                    local_place_next = local_place_next.next();
                    match table.get(local_place_next.0) {
                        Some(Square::Filled(tile)) => {
                            local_score += scoring.score_for(&tile);
                            has_local_word = true;
                        },
                        _ => break // out of board, or no more letters
                    }
                }
                
                let letter_score = scoring.score_for(&current_tile);
                
                let bonus = bonuses.bonus_at(current_place.0);
                
                if has_local_word {
                    perp_score += (local_score + letter_score * bonus.letter) * bonus.word;
                }
                
                // iteration updates
                let (step, next_tile) = match others_iter.next() {
                    Some(o) => o,
                    None => break,
                };
                current_tile = next_tile;
                current_place.0[current_place.1] += step + 1;
            }
            
            
            let mut word_score = 0;
            let mut word_multiplier = 1;
            
            let mut begin_word = place.clone();
            let mut step = 0;
            while let Some(Square::Filled(_)) = table.get(begin_word.back().0) {
                begin_word = begin_word.back();
                step += 1;
            }
            
            let mut current_place = begin_word;
            let mut next_move_tile = Some((first, step));
            let mut others_iter = others.iter().cloned();
            
            loop {
                match table.get(current_place.0) {
                    None => break,
                    Some(Square::Filled(tile)) => {
                        if let Some((_, s)) = next_move_tile {
                            assert!(s != 0);
                        }
                        word_score += scoring.score_for(&tile);
                    },
                    Some(Square::Empty) => {
                        match &next_move_tile {
                            None => break,
                            Some((tile, step)) => {
                                assert_eq!(*step, 0);
                                let score = scoring.score_for(&tile);
                                let bonus = bonuses.bonus_at(current_place.0);
                                word_score += score * bonus.letter;
                                word_multiplier *= bonus.word;
                            }
                        }
                    },
                }
                
                // update
                current_place = current_place.next();
                next_move_tile = next_move_tile.and_then(|(tile, step)| {
                    if step == 0 {
                        match others_iter.next() {
                            Some((step, tile)) => Some((tile, step)),
                            None => None
                        }
                    } else {
                        Some((tile, step - 1))
                    }
                });
            }
            
            word_score * word_multiplier + perp_score + if others.len() == 6 { score_rules.extra_bonus } else { 0 }
        },
    }
}
