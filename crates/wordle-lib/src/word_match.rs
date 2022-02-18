use std::fmt;
use std::fmt::Display;

#[derive(Eq, PartialEq)]
#[repr(u8)]
enum CharMatch {
    Absent = 0,
    Present = 1,
    Correct = 2,
}

#[derive(Clone, Eq, PartialEq, Hash, Copy)]
pub struct WordMatch(pub u8);

impl WordMatch {
    const POWERS: [u8; 5] = [1, 3, 9, 27, 81];
    pub const ABSENT: WordMatch = WordMatch(0);
    pub const CORRECT: WordMatch = WordMatch(242);

    pub fn from(guess: [u8; 5], answer: [u8; 5]) -> Self {
        let mut matches = Self::ABSENT;
        let mut available = 0u64;
        for i in 0..5 {
            let g = guess[i];
            let a = answer[i];
            if g == a {
                matches.set(i, CharMatch::Correct);
            } else {
                available += 1 << (2 * (a - 97));
            }
        }
        for i in 0..5 {
            if matches.get(i) == CharMatch::Absent {
                let g = guess[i];
                if (available >> (2 * (g - 97))) & 3 != 0 {
                    matches.set(i, CharMatch::Present);
                    available -= 1 << (2 * (g - 97));
                }
            }
        }
        matches
    }

    fn get(&self, idx: usize) -> CharMatch {
        match self.0 / Self::POWERS[idx] % 3 {
            0 => CharMatch::Absent,
            1 => CharMatch::Present,
            _ => CharMatch::Correct,
        }
    }

    fn set(&mut self, idx: usize, m: CharMatch) {
        self.0 += m as u8 * Self::POWERS[idx]
    }
}

impl Display for WordMatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..5 {
            match self.get(i) {
                CharMatch::Absent => write!(f, "a")?,
                CharMatch::Present => write!(f, "p")?,
                CharMatch::Correct => write!(f, "c")?,
            }
        }
        Ok(())
    }
}
