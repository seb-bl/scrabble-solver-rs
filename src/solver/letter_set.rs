use super::Letter;

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct LetterSet {
    // bit is one if letter is in it
    accepted: [u128; 2],
}

impl LetterSet {
    pub fn empty() -> Self {
        Self { accepted: [0; 2] }
    }
    pub fn any() -> Self {
        Self { accepted: [u128::MAX; 2] }
    }
    pub fn contains(&self, letter: Letter) -> bool {
        let i = letter.0 as usize;
        (self.accepted[i / 128]  & (1 << (i%128))) != 0
    }
    pub fn insert(&mut self, letter: Letter) {
        let i = letter.0 as usize;
        self.accepted[i / 128]  |= 1 << (i%128)
    }
    pub fn from_many(iter: impl Iterator<Item=Letter>) -> Self {
        let mut tmp = Self::empty();
        iter.for_each(|l| tmp.insert(l));
        tmp
    }
    pub fn is_empty(&self) -> bool {
        self.accepted.iter().all(|&l| l == 0)
    }
    pub fn is_any(&self) -> bool {
        self.accepted.iter().all(|&l| l == u128::MAX)
    }
    
    pub const ALPHABET: Self = {
        let mut tmp = Self { accepted: [0; 2] };
        let mut i = b'a';
        while i <= b'z' {
            tmp.accepted[i as usize / 128]  |= 1 << (i as usize%128);
            i += 1;
        }
        tmp
    };
}

impl Default for LetterSet {
    fn default() -> Self {
        Self::empty()
    }
}

impl std::iter::FromIterator<Letter> for LetterSet {
   fn from_iter<T>(iter: T) -> Self where T: IntoIterator<Item=Letter> {
       let mut tmp = Self::default();
       iter.into_iter().for_each(|l| tmp.insert(l));
       tmp
    }
}

use std::fmt;

impl fmt::Debug for LetterSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_any() {
            write!(f, ".")
        } else {
            write!(f, "[")?;
            for l in 0..=255u8 {
                if self.contains(Letter(l)) {
                    write!(f, "{}", Letter(l))?;
                }
            }
            write!(f, "]")
        }
    }
}


#[test]
fn letter_set() {
    let empty = LetterSet::empty();
    for l in 0..=255u8 {
        assert_eq!(empty.contains(Letter(l)), false);
    }
    assert!(empty.is_empty());
    assert!(!empty.is_any());
    
    let some = vec![6, 42, 17, 17, 230];
    let not_empty: LetterSet = some.iter().map(|&i| Letter(i)).collect();
    for &i in &some {
        assert!(not_empty.contains(Letter(i)));
    }
    for l in 0..=255u8 {
        if some.iter().any(|&i| i == l) {
            continue
        }
        assert_eq!(not_empty.contains(Letter(l)), false);
    }
    assert!(!not_empty.is_empty());
    assert!(!not_empty.is_any());
    
    let any = LetterSet::any();
    for l in 0..=255u8 {
        assert!(any.contains(Letter(l)));
    }
    assert!(!any.is_empty());
    assert!(any.is_any());
}
