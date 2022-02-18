use crate::WordMatch;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rayon::slice::ParallelSlice;
use std::cmp;
use std::collections::HashMap;
use std::ops::ControlFlow;

pub trait Dictionary: Sync {
    type Word: Copy;

    fn len(&self) -> usize;

    fn partition(&self, guess: Self::Word) -> HashMap<WordMatch, Self>
    where
        Self: Sized;

    fn for_each<F>(&self, f: F)
    where
        F: FnMut(Self::Word);

    fn try_for_each<F, R>(&self, f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Word) -> ControlFlow<R>;

    fn par_process<F>(&self, weight: u32, f: F) -> Option<u32>
    where
        F: Fn(Self::Word) -> Option<u32> + Sync + Send;
}

pub struct WordDictionary(Vec<u8>);

impl WordDictionary {
    pub fn new(words: &[u8]) -> Self {
        assert_eq!(words.len() % 5, 0);
        Self(words.to_vec())
    }

    pub fn index_of(&self, word: [u8; 5]) -> Option<usize> {
        for i in 0..self.len() {
            if word == self.word(i) {
                return Some(i);
            }
        }
        None
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self(Vec::with_capacity(5 * cap))
    }

    pub fn word(&self, idx: usize) -> [u8; 5] {
        unsafe { self.0[5 * idx..5 * idx + 5].try_into().unwrap_unchecked() }
    }

    pub fn push(&mut self, word: [u8; 5]) {
        self.0.extend_from_slice(&word);
    }
}

impl Dictionary for WordDictionary {
    type Word = [u8; 5];

    fn len(&self) -> usize {
        self.0.len() / 5
    }

    fn partition(&self, guess: Self::Word) -> HashMap<WordMatch, Self> {
        let mut partition = HashMap::with_capacity(cmp::min(self.len(), 243));
        self.for_each(|answer| {
            partition
                .entry(WordMatch::from(guess, answer))
                .or_insert_with(|| Self::with_capacity(self.len() / 50))
                .push(answer);
        });
        partition
    }

    fn for_each<F>(&self, f: F)
    where
        F: FnMut(Self::Word),
    {
        self.0.array_chunks().copied().for_each(f)
    }

    fn try_for_each<F, R>(&self, f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Word) -> ControlFlow<R>,
    {
        self.0.array_chunks().copied().try_for_each(f)
    }

    fn par_process<F>(&self, weight: u32, f: F) -> Option<u32>
    where
        F: Fn(Self::Word) -> Option<u32> + Sync + Send,
    {
        self.0
            .par_chunks(5)
            .map(|word| unsafe { word.try_into().unwrap_unchecked() })
            .filter_map(f)
            .min()
            .map(|sub_weight| weight + sub_weight)
    }
}

const MATCHES: &'static [u8] = include_bytes!("../../../matches.bin");

pub struct OffsetDictionary {
    pub words: Vec<u16>,
}

impl OffsetDictionary {
    pub fn new() -> Self {
        Self {
            words: (0..2309).collect(),
        }
    }

    fn push(&mut self, word: u16) {
        self.words.push(word);
    }
}

impl Dictionary for OffsetDictionary {
    type Word = u16;

    fn len(&self) -> usize {
        self.words.len()
    }

    fn partition(&self, guess: Self::Word) -> HashMap<WordMatch, Self>
    where
        Self: Sized,
    {
        let mut partition = HashMap::with_capacity(cmp::min(self.len(), 243));
        self.for_each(|answer| {
            partition
                .entry(WordMatch(MATCHES[guess as usize * 2309 + answer as usize]))
                .or_insert_with(|| OffsetDictionary {
                    words: Vec::with_capacity(self.len() / 50),
                })
                .push(answer);
        });
        partition
    }

    fn for_each<F>(&self, f: F)
    where
        F: FnMut(Self::Word),
    {
        self.words.iter().copied().for_each(f)
    }

    fn try_for_each<F, R>(&self, f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Word) -> ControlFlow<R>,
    {
        self.words.iter().copied().try_for_each(f)
    }

    fn par_process<F>(&self, weight: u32, f: F) -> Option<u32>
    where
        F: Fn(Self::Word) -> Option<u32> + Sync + Send,
    {
        self.words
            .par_iter()
            .copied()
            .filter_map(f)
            .min()
            .map(|sub_weight| weight + sub_weight)
    }
}

#[cfg(test)]
mod test {
    use crate::{Dictionary, WordDictionary, WordMatch, LEGAL_ANSWERS};
    use std::fs::File;
    use std::io::{BufWriter, Write};

    #[test]
    fn gen_dict() {
        let guesses = WordDictionary::new(LEGAL_ANSWERS);
        let mut matches = BufWriter::new(File::create("../../matches.bin").unwrap());
        guesses.for_each(|guess| {
            guesses.for_each(|answer| {
                matches.write(&[WordMatch::from(guess, answer).0]).unwrap();
            })
        });
    }
}
