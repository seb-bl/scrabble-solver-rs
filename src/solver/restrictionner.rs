
use fst::{Automaton, Set, IntoStreamer, Streamer};

use super::{Square, RestrictedSquare, LetterTile, Letter, LetterSet};

struct RestrictionChecker<'a> {
    prefix: &'a [LetterTile],
    suffix: &'a [LetterTile],
}

#[derive(Clone, Debug)]
enum RestrictionCheckerState {
    Prefix(usize),
    Mid,
    Suffix(usize, Letter),
    Done(Letter),
}

impl<'a> Automaton for RestrictionChecker<'a> {
    type State = Option<RestrictionCheckerState>;
    fn start(&self) -> Self::State {
        if self.prefix.len() == 0 {
            Some(RestrictionCheckerState::Mid)
        } else {
            Some(RestrictionCheckerState::Prefix(0))
        }
    }
    fn is_match(&self, state: &Self::State) -> bool {
        match state {
            Some(RestrictionCheckerState::Done(_)) => true,
            _ => false,
        }
    }
    fn accept(&self, state: &Self::State, byte: u8) -> Self::State {
        state.as_ref().and_then(|state| {
            match state {
                &RestrictionCheckerState::Prefix(i) => {
                    let ok = match self.prefix[i] {
                        LetterTile::Wildcard => true,
                        LetterTile::Letter(l) => l == Letter(byte),
                    };
                    if ok {
                        Some(if i+1 == self.prefix.len() {
                            RestrictionCheckerState::Mid
                        } else {
                            RestrictionCheckerState::Prefix(i+1)
                        })
                    } else {
                        None
                    }
                },
                RestrictionCheckerState::Mid => {
                    Some(if self.suffix.len() == 0 {
                        RestrictionCheckerState::Done(Letter(byte))
                    } else {
                        RestrictionCheckerState::Suffix(0, Letter(byte))
                    })
                },
                &RestrictionCheckerState::Suffix(i, l) => {
                    let ok = match self.suffix.get(i) {
                        None => false,
                        Some(LetterTile::Wildcard) => true,
                        Some(&LetterTile::Letter(l)) => l == Letter(byte),
                    };
                    if ok {
                        Some(if i+1 == self.suffix.len() {
                            RestrictionCheckerState::Done(l)
                        } else {
                            RestrictionCheckerState::Suffix(i+1, l)
                        })
                    } else {
                        None
                    }
                },
                RestrictionCheckerState::Done(_) => None,
            }
        })
    }
    
    fn can_match(&self, state: &Self::State) -> bool {
        state.is_some()
    }
}

pub fn find_restrictions(
    line: &[Square], restr: &mut [RestrictedSquare],
    dictionary: &Set<impl AsRef<[u8]>>,
) {
    assert_eq!(line.len(), restr.len());
    
    let mut prefix = vec![];
    let mut suffix = vec![];
    
    for (i, r) in restr.iter_mut().enumerate() {
        *r = if let Some(&tile) = line[i].tile() {
            RestrictedSquare::Filled(tile)
        } else {
            // find prefix
            prefix.clear();
            for j in (0..i).rev() {
                if let Some(&s) = line[j].tile() {
                    prefix.insert(0, s)
                } else {
                    break
                }
            }
            
            // find suffix
            suffix.clear();
            for j in (i+1)..(line.len()) {
                if let Some(&s) = line[j].tile() {
                    suffix.push(s)
                } else {
                    break
                }
            }
            
            RestrictedSquare::Empty(if prefix.is_empty() && suffix.is_empty() {
                // if prefix == suffix == "" then ALPHABET
                LetterSet::any()
            } else {
                // make regex: prefix[a-z]suffix
                let automaton = RestrictionChecker {
                    prefix: &prefix[..],
                    suffix: &suffix[..],
                };
                // check against dict
                let mut matches = dictionary.search_with_state(automaton).into_stream();
                let mut letter_set = LetterSet::empty();
                while let Some((_, state)) = matches.next() {
                    if let Some(RestrictionCheckerState::Done(l)) = state {
                        letter_set.insert(l);
                    } else {
                        unreachable!("not in final state");
                    }
                }
                
                letter_set
            })
        }
    }
}

#[test]
fn test() {
    use fst::SetBuilder;
    use std::iter::FromIterator;
    
    let mut words = vec![
        "lore",
        "love",
        "elle",
        "bles",
    ];
    
    words.sort_unstable();
    
    let mut build = SetBuilder::memory();
    build.extend_iter(words).unwrap();
    let dict = build.into_set();

    let line = [
        Square::Filled(LetterTile::Wildcard),
        Square::Empty,
        Square::Empty,
        Square::Filled(LetterTile::Wildcard),
        Square::Filled(LetterTile::Letter(Letter(b'l'))),
        Square::Filled(LetterTile::Letter(Letter(b'e'))),
        Square::Empty,
        Square::Empty,
        Square::Empty,
        Square::Filled(LetterTile::Letter(Letter(b'l'))),
        Square::Filled(LetterTile::Letter(Letter(b'o'))),
        Square::Empty,
        Square::Filled(LetterTile::Letter(Letter(b'e'))),
    ];

    let mut restr = [
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Empty(LetterSet::empty()),
    ];

    find_restrictions(
        &line, &mut restr,
        &dict,
    );

    dbg!(&restr);
    
    assert_eq!(restr, [
        RestrictedSquare::Filled(LetterTile::Wildcard),
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Empty(LetterSet::from_iter(vec![Letter(b'e')])),
        RestrictedSquare::Filled(LetterTile::Wildcard),
        RestrictedSquare::Filled(LetterTile::Letter(Letter(b'l'))),
        RestrictedSquare::Filled(LetterTile::Letter(Letter(b'e'))),
        RestrictedSquare::Empty(LetterSet::from_iter(vec![Letter(b's')])),
        RestrictedSquare::Empty(LetterSet::any()),
        RestrictedSquare::Empty(LetterSet::empty()),
        RestrictedSquare::Filled(LetterTile::Letter(Letter(b'l'))),
        RestrictedSquare::Filled(LetterTile::Letter(Letter(b'o'))),
        RestrictedSquare::Empty(LetterSet::from_iter(vec![Letter(b'v'), Letter(b'r')])),
        RestrictedSquare::Filled(LetterTile::Letter(Letter(b'e'))),
    ]);
}
