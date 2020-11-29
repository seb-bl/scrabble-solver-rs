
use std::rc::Rc;
use std::fmt;

use fst::Automaton;

use super::{RestrictedSquare, LetterTile, Letter};

#[derive(Clone)]
pub struct TrayRemaining {
    letters: [u8; 256],
    n_wildcards: u8,
    /// The total number of remaining letters+wildcards to play
    n_total: u32,
}

impl TrayRemaining {
    pub fn new(letters: [u8; 256], n_wildcards: u8) -> TrayRemaining {
        let n_total = letters.iter().map(|&i| i as u32).sum::<u32>() + n_wildcards as u32;
        TrayRemaining {
            letters,
            n_wildcards,
            n_total,
        }
    }
}

impl fmt::Debug for TrayRemaining {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // we will only print letters
        for l in b'a'..=b'z' {
            for _ in 0..self.letters[l as usize] {
                write!(f, "{}", l as char)?;
            }
        }
        for _ in 0..self.n_wildcards {
            write!(f, "*")?;
        }
        write!(f, "[{} letters]", self.n_total)
    }
}

impl TrayRemaining {
    pub fn remove(&self, letter: u8) -> Option<TrayRemaining> {
        if self.letters[letter as usize] > 0 {
            let mut tmp = self.clone();
            tmp.letters[letter as usize] -= 1;
            tmp.n_total -= 1;
            Some(tmp)
        } else {
            None
        }
    }
    pub fn remove_wildcard(&self) -> Option<TrayRemaining> {
        if self.n_wildcards > 0 {
            let mut tmp = self.clone();
            tmp.n_wildcards -= 1;
            tmp.n_total -= 1;
            Some(tmp)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WildcardAssignmentList {
    Empty,
    Elem(WildcardAssignment, Rc<WildcardAssignmentList>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum WildcardAssignment {
    /// Position of the intersection
    Intersection(usize),
    /// Value of the missing letter
    MissingLetter(u8),
}

#[derive(Debug, Clone)]
pub struct ScrabbleAutomata<'line> {
    /// The line slice that starts at the begin of the word
    pub line: &'line [RestrictedSquare],
    /// What there is in the tray
    pub tray: TrayRemaining,
    /// The required length for a word to be attached
    pub min_len: usize,
    /// Whether a wilcard can be played and used as different letter for the
    /// horizontal and the vertical word in participates in
    ///
    /// This only applies to wildcards in the move being created, wildcards on
    /// the board are always interpreted as signifying anything
    pub wildcards_have_multi_meaning: bool,
}

#[derive(Debug, Clone)]
pub struct ScrabbleAutomataState {
    /// How far we are on the line
    pub position: usize,
    /// Assigned wildcards
    pub wildcards: WildcardAssignmentList,
    /// What is left in the tray
    pub tray: TrayRemaining,
}

impl<'line> Automaton for ScrabbleAutomata<'line> {
    type State = Option<ScrabbleAutomataState>;
    
    fn start(&self) -> Self::State {
        Some(ScrabbleAutomataState {
            position: 0,
            wildcards: WildcardAssignmentList::Empty,
            tray: self.tray.clone(),
        })
    }
    
    fn is_match(&self, state: &Self::State) -> bool {
        if let Some(state) = state {
            if let Some(RestrictedSquare::Filled(_)) = self.line.get(state.position) {
                // there is a letter where the word continues
                false
            } else {
                if self.tray.n_total == state.tray.n_total {
                    // we have not played a single thing
                    false
                } else {
                    if state.position < self.min_len {
                        // the word is too short to be attached
                        false
                    } else {
                        true
                    }
                }
            }
        } else {
            false
        }
    }
    
    fn accept(&self, state: &Self::State, byte: u8) -> Self::State {
        state.as_ref().and_then(|state| {
            match self.line.get(state.position) {
                // we are out of the board
                None => None,
                Some(spot) => match spot {
                    // a wildcard accepts everything
                    RestrictedSquare::Filled(LetterTile::Wildcard) => Some(ScrabbleAutomataState {
                        position: state.position + 1,
                        wildcards: state.wildcards.clone(),
                        tray: state.tray.clone(),
                    }),
                    // letter on the board must match what we accept
                    &RestrictedSquare::Filled(LetterTile::Letter(l)) => if l == Letter(byte) {
                        Some(ScrabbleAutomataState {
                            position: state.position + 1,
                            wildcards: state.wildcards.clone(),
                            tray: state.tray.clone(),
                        })
                    } else {
                        None
                    },
                    
                    // consume the letter from the tray, or a wildcard
                    // if it is accepeted by the intersection
                    RestrictedSquare::Empty(letter_set) => {
                        if letter_set.is_empty() {
                            // intersection is never satisfied
                            None
                        } else {
                            let (new_tray, wildcard_assignment) = if letter_set.contains(Letter(byte)) {
                                // the letter respects restriction from other direction
                                state.tray.remove(byte)
                                .map(|tray| (Some(tray), None)) // we have the needed letter
                                .or_else(|| state.tray.remove_wildcard().map(|tray|
                                    // this is a missing letter
                                    (Some(tray), Some(WildcardAssignment::MissingLetter(byte))))
                                )
                                .unwrap_or((None, None))
                            } else {
                                if self.wildcards_have_multi_meaning {
                                    // the letter does not respect restrictions from other direction
                                    // but a wildcard is allowed to act as a different letter in the other direction, thus satisfy the restrictions
                                    state.tray.remove_wildcard().map(|tray|
                                        (Some(tray), Some(WildcardAssignment::Intersection(state.position)))
                                    )
                                    .unwrap_or((None, None))
                                } else {
                                    (None, None)
                                }
                            };
                            new_tray.map(|tray| ScrabbleAutomataState {
                                position: state.position + 1,
                                wildcards: if let Some(assig) = wildcard_assignment {
                                    WildcardAssignmentList::Elem(assig, Rc::new(state.wildcards.clone()))
                                } else {
                                    state.wildcards.clone()
                                },
                                tray: tray,
                            })
                        }
                    },
                }
            }
        })
    }
    
    fn can_match(&self, state: &Self::State) -> bool {
        state.is_some()
    }
}

#[test]
fn test() {
    use super::LetterSet;
    
    let line = [
        RestrictedSquare::Empty(
            b"abdfghklmopqstx".iter().map(|&l| Letter(l)).collect(),
        ),
        RestrictedSquare::Empty(
            b"abdefghijklmnopqrstuwxyz".iter().map(|&l| Letter(l)).collect()
        ),
        RestrictedSquare::Empty(
            b"a".iter().map(|&l| Letter(l)).collect()
        ),
        RestrictedSquare::Empty(
            LetterSet::any(),
        ),
        RestrictedSquare::Empty(
            LetterSet::any(),
        ),
        RestrictedSquare::Empty(
            LetterSet::any(),
        ),
    ];

    let automaton = ScrabbleAutomata {
        line: &line[..],
        tray: TrayRemaining {
            letters: [1; 256],
            n_wildcards: 1,
            n_total: 257,
        },
        min_len: 0,
        wildcards_have_multi_meaning: true,
    };

    dbg!(&automaton);

    let mut build = fst::SetBuilder::memory();
    build.insert(b"tepa").unwrap();
    let dict = build.into_set();

    use fst::{Streamer, IntoStreamer};

    let mut x = dict.search_with_state(automaton).into_stream();
    
    let mut acc = vec![];
    
    while let Some(w) = x.next() {
        dbg!(std::str::from_utf8(w.0).unwrap(), &w.1);
        acc.push((std::str::from_utf8(w.0).unwrap().to_string(), w.1.expect("reached valid state")))
    }
    
    assert_eq!(acc.len(), 1);
    
    assert_eq!(acc[0].0, "tepa");
    assert_eq!(acc[0].1.position, 4);
    assert_eq!(
        acc[0].1.wildcards,
        WildcardAssignmentList::Elem(WildcardAssignment::Intersection(2), Rc::new(WildcardAssignmentList::Empty)),
    );
}