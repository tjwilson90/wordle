#![feature(array_chunks)]
#![feature(slice_group_by)]

use rayon::iter::ParallelIterator;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator};
use rayon::slice::ParallelSlice;
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap};
use std::error::Error;
use std::fmt::Display;
use std::ops::ControlFlow;
use std::{cmp, fmt, ptr};

const LEGAL_GUESSES: &'static [u8] = include_bytes!("../guesses.txt");
const LEGAL_ANSWERS: &'static [u8] = include_bytes!("../answers.txt");

#[derive(Eq, PartialEq)]
#[repr(u8)]
enum CharMatch {
    Absent = 0,
    Present = 1,
    Correct = 2,
}

#[derive(Clone, Eq, PartialEq, Hash, Copy)]
struct WordMatch(u8);

impl WordMatch {
    const POWERS: [u8; 5] = [1, 3, 9, 27, 81];
    const ABSENT: WordMatch = WordMatch(0);
    const CORRECT: WordMatch = WordMatch(242);

    fn idx(self) -> usize {
        self.0 as usize
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

struct Guess {
    word: [u8; 5],
    max_partition_len: usize,
    partition: HashMap<WordMatch, Dictionary>,
}

impl Guess {
    fn new(guess: [u8; 5], answers: &Dictionary) -> Self {
        let mut partition = HashMap::with_capacity(cmp::min(answers.len(), 243));
        for answer in answers.iter() {
            let m = word_match(guess, answer);
            partition
                .entry(m)
                .or_insert_with(|| Dictionary::with_capacity(answers.len() / 100))
                .push(answer);
        }
        Guess {
            word: guess,
            max_partition_len: partition.values().map(|d| d.len()).max().unwrap(),
            partition,
        }
    }

    fn fast_solution(&self, depth: usize) -> Option<Solution> {
        if self.max_partition_len == 1
            && depth > 1
            && self.partition.contains_key(&WordMatch::CORRECT)
        {
            let dict = &self.partition[&WordMatch::CORRECT];
            let mut solution = Solution {
                guess: dict.word(0),
                size: (2 * self.partition.len() - 1) as u16,
                solution: Vec::with_capacity(self.partition.len()),
            };
            for (wm, dict) in &self.partition {
                solution.solution.push((
                    *wm,
                    Solution {
                        guess: dict.word(0),
                        size: 1,
                        solution: Vec::new(),
                    },
                ))
            }
            Some(solution)
        } else {
            None
        }
    }

    fn slow_solution(
        &self,
        guesses: &Dictionary,
        answers: &Dictionary,
        breadth: usize,
        depth: usize,
        hard: bool,
    ) -> Option<Solution> {
        let mut solution = Solution {
            guess: self.word,
            size: 0,
            solution: Vec::with_capacity(self.partition.len()),
        };
        for (wm, dict) in &self.partition {
            let sub_solution = if hard && ptr::eq(guesses, answers) {
                solve(&dict, &dict, breadth, depth, hard)
            } else if hard {
                let mut sub_guesses = Dictionary::with_capacity(guesses.len() / 100);
                for word in guesses.iter() {
                    if word_match(self.word, word) == *wm {
                        sub_guesses.push(word);
                    }
                }
                solve(&sub_guesses, &dict, breadth, depth, hard)
            } else {
                solve(guesses, &dict, breadth, depth, hard)
            };
            if let Some(sub_solution) = sub_solution {
                solution.size += dict.len() as u16;
                if *wm != WordMatch::CORRECT {
                    solution.size += sub_solution.size;
                }
                solution.solution.push((*wm, sub_solution));
            } else {
                return None;
            }
        }
        Some(solution)
    }
}

impl PartialEq for Guess {
    fn eq(&self, other: &Self) -> bool {
        self.word.eq(&other.word)
    }
}

impl Eq for Guess {}

impl PartialOrd for Guess {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Guess {
    fn cmp(&self, other: &Self) -> Ordering {
        self.max_partition_len
            .cmp(&other.max_partition_len)
            .then_with(|| other.partition.len().cmp(&self.partition.len()))
            .then_with(|| self.word.cmp(&other.word))
    }
}

struct Dictionary(Vec<u8>);

impl Dictionary {
    fn new(words: &[u8]) -> Self {
        assert_eq!(words.len() % 5, 0);
        Self(words.to_vec())
    }

    fn with_capacity(cap: usize) -> Self {
        Self(Vec::with_capacity(5 * cap))
    }

    fn push(&mut self, word: [u8; 5]) {
        self.0.extend_from_slice(&word);
    }

    fn len(&self) -> usize {
        self.0.len() / 5
    }

    fn word(&self, idx: usize) -> [u8; 5] {
        unsafe { self.0[5 * idx..5 * idx + 5].try_into().unwrap_unchecked() }
    }

    fn iter(&self) -> impl Iterator<Item = [u8; 5]> + Clone + ExactSizeIterator + '_ {
        self.0.array_chunks().map(|word| *word)
    }

    fn par_iter(&self) -> impl ParallelIterator<Item = [u8; 5]> + '_ {
        self.0
            .par_chunks(5)
            .map(|word| unsafe { word.try_into().unwrap_unchecked() })
    }
}

struct Solution {
    guess: [u8; 5],
    size: u16,
    solution: Vec<(WordMatch, Solution)>,
}

impl Solution {
    fn print(&self, line: &mut String) {
        line.push(' ');
        line.push_str(std::str::from_utf8(&self.guess).unwrap());
        if self.solution.is_empty() {
            println!("{}", line);
        } else {
            for (wm, sub) in self.solution.iter() {
                if *wm == WordMatch::CORRECT {
                    println!("{}", line);
                } else {
                    line.push(' ');
                    line.push_str(&wm.to_string());
                    sub.print(line);
                    line.drain(line.len() - 6..);
                }
            }
        }
        line.drain(line.len() - 6..);
    }
}

fn word_match(guess: [u8; 5], answer: [u8; 5]) -> WordMatch {
    let mut matches = WordMatch::ABSENT;
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

fn solve(
    guesses: &Dictionary,
    answers: &Dictionary,
    breadth: usize,
    depth: usize,
    hard: bool,
) -> Option<Solution> {
    if answers.len() == 1 {
        return Some(Solution {
            guess: answers.word(0),
            size: 1,
            solution: Vec::new(),
        });
    }
    if depth == 1 {
        return None;
    }
    let mut best_guesses = BinaryHeap::with_capacity(breadth);
    for guess in guesses.iter() {
        let guess = Guess::new(guess, answers);
        if guess.partition.len() == 1 {
            // learned nothing, not a useful guess
            continue;
        }
        if let Some(solution) = guess.fast_solution(depth - 1) {
            return Some(solution);
        }
        if best_guesses.len() < best_guesses.capacity() {
            best_guesses.push(guess);
        } else if guess < *best_guesses.peek().unwrap() {
            best_guesses.pop();
            best_guesses.push(guess);
        }
    }
    best_guesses
        .into_par_iter()
        .filter_map(|guess: Guess| guess.slow_solution(guesses, answers, breadth, depth - 1, hard))
        .min_by_key(|solution: &Solution| solution.size)
}

fn solve3(guess: [u8; 5], dict: &Dictionary, depth: usize) -> Option<u16> {
    if depth == 0 {
        return if dict.len() == 1 { Some(1) } else { None };
    }
    let mut partition = HashMap::with_capacity(cmp::min(dict.len(), 243));
    for answer in dict.iter() {
        partition
            .entry(word_match(guess, answer))
            .or_insert_with(|| Dictionary::with_capacity(dict.len() / 50))
            .push(answer);
    }
    partition.remove(&WordMatch::CORRECT);
    if partition.len() == dict.len() - 1 {
        return Some(2 * partition.len() as u16 + 1);
    }
    partition.into_values().try_fold(1, |total, dict| {
        dict.par_iter()
            .filter_map(|(guess)| solve3(guess, &dict, depth - 1))
            .min()
            .map(|sub_total| total + dict.len() as u16 + sub_total)
    })
}

struct Solver {
    breadth: usize,
    hard: bool,
}

impl Solver {
    fn solve(
        &self,
        guess: [u8; 5],
        guesses: &Dictionary,
        answers: &Dictionary,
        depth: usize,
    ) -> Option<Solution> {
        if depth == 0 {
            return if answers.len() == 1 {
                Some(Solution {
                    guess,
                    size: 1,
                    solution: Vec::new(),
                })
            } else {
                None
            };
        }
        let mut partition = HashMap::with_capacity(cmp::min(answers.len(), 243));
        for answer in answers.iter() {
            partition
                .entry(word_match(guess, answer))
                .or_insert_with(|| Dictionary::with_capacity(answers.len() / 50))
                .push(answer);
        }
        partition.remove(&WordMatch::CORRECT);
        if partition.len() == answers.len() - 1 {
            return Some(Solution {
                guess,
                size: 2 * partition.len() as u16 + 1,
                solution: partition
                    .into_iter()
                    .map(|(wm, dict)| {
                        (
                            wm,
                            Solution {
                                guess: dict.word(0),
                                size: 1,
                                solution: Vec::new(),
                            },
                        )
                    })
                    .collect(),
            });
        }
        let solution = Solution {
            guess,
            size: 1,
            solution: Vec::with_capacity(partition.len()),
        };
        partition
            .into_iter()
            .try_fold(solution, |mut solution, (wm, dict)| {
                let mut next_guesses = Dictionary::with_capacity(guesses.len() / 100);
                let next_guesses = if self.hard && ptr::eq(guesses, answers) {
                    &dict
                } else if self.hard {
                    for word in guesses.iter() {
                        if word_match(guess, word) == wm {
                            next_guesses.push(word);
                        }
                    }
                    &next_guesses
                } else {
                    guesses
                };
                next_guesses
                    .par_iter()
                    .filter_map(|guess| self.solve(guess, &dict, &dict, depth - 1))
                    .min_by_key(|solution| solution.size)
                    .map(|sub_solution| {
                        solution.size += sub_solution.size + dict.len() as u16;
                        solution.solution.push((wm, sub_solution));
                        solution
                    })
            })
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut hard = false;
    let mut breadth = 10;
    let mut depth = 6;
    let mut limit_guesses = false;
    let mut first_guess = None;
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--hard" {
            hard = true;
        } else if arg == "--breadth" {
            breadth = args.next().unwrap().parse().unwrap();
        } else if arg == "--depth" {
            depth = args.next().unwrap().parse().unwrap();
        } else if arg == "--limit-guesses" {
            limit_guesses = true;
        } else if arg == "--guess" {
            first_guess = args.next();
        }
    }
    let answers = &Dictionary::new(LEGAL_ANSWERS);
    let mut guesses = &Dictionary::new(LEGAL_GUESSES);
    if limit_guesses {
        guesses = answers;
    }
    // for guess in guesses.iter().take(5) {
    //     if let Some(total) = solve3(guess, answers, depth - 1) {
    //         eprintln!(
    //             "{}: mean: {}",
    //             String::from_utf8_lossy(&guess),
    //             total as f32 / answers.len() as f32
    //         );
    //     } else {
    //         eprintln!("{}: no solution", String::from_utf8_lossy(&guess));
    //     }
    // }
    let solution = if let Some(guess) = first_guess {
        let guess = Guess::new(guess.as_bytes().try_into().unwrap(), answers);
        guess.slow_solution(guesses, answers, breadth, depth - 1, hard)
    } else {
        solve(guesses, answers, breadth, depth, hard)
    };
    if let Some(solution) = solution {
        solution.print(&mut String::new());
        eprintln!("mean: {}", solution.size as f32 / answers.len() as f32);
    } else {
        eprintln!("no solution");
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_word_match() {
        assert_eq!(word_match(*b"sanes", *b"boats").to_string(), "apaac");
        assert_eq!(word_match(*b"tonka", *b"aunty").to_string(), "pacap");
        assert_eq!(word_match(*b"lares", *b"coach").to_string(), "apaaa");
    }
}
