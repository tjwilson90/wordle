use crate::{Dictionary, WordDictionary, WordMatch};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::ops::ControlFlow;
use std::ptr;

pub struct Guess {
    word: [u8; 5],
    entropy: f64,
    partition: HashMap<WordMatch, WordDictionary>,
}

impl Guess {
    pub fn new(guess: [u8; 5], answers: &WordDictionary) -> Self {
        let partition = answers.partition(guess);
        Guess {
            word: guess,
            entropy: partition.values().map(|d| f64::log2(d.len() as f64)).sum(),
            partition,
        }
    }

    fn fast_solution(&self, depth: usize) -> Option<Solution> {
        if self.entropy < 1.0 && depth > 1 && self.partition.contains_key(&WordMatch::CORRECT) {
            let dict = &self.partition[&WordMatch::CORRECT];
            Some(Solution {
                guess: dict.word(0),
                size: 2 * self.partition.len() as u16 - 1,
                solution: self
                    .partition
                    .iter()
                    .map(|(wm, dict)| {
                        (
                            *wm,
                            Solution {
                                guess: dict.word(0),
                                size: 1,
                                solution: Vec::new(),
                            },
                        )
                    })
                    .collect(),
            })
        } else {
            None
        }
    }

    pub fn slow_solution(
        self,
        guesses: &WordDictionary,
        answers: &WordDictionary,
        breadth: usize,
        depth: usize,
        hard: bool,
    ) -> Option<Solution> {
        let partitions = if hard && !ptr::eq(guesses, answers) {
            guesses.partition(self.word)
        } else {
            HashMap::new()
        };
        let solution = Solution {
            guess: self.word,
            size: 0,
            solution: Vec::with_capacity(self.partition.len()),
        };
        self.partition
            .into_iter()
            .try_fold(solution, |mut solution, (wm, dict)| {
                let guesses = if hard && ptr::eq(guesses, answers) {
                    &dict
                } else if hard {
                    partitions.get(&wm).unwrap()
                } else {
                    guesses
                };
                let sub_solution = solve(guesses, &dict, breadth, depth, hard);

                sub_solution.map(|sub_solution| {
                    solution.size += dict.len() as u16;
                    if wm != WordMatch::CORRECT {
                        solution.size += sub_solution.size;
                    }
                    solution.solution.push((wm, sub_solution));
                    solution
                })
            })
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
        other
            .entropy
            .partial_cmp(&self.entropy)
            .unwrap()
            .then_with(|| self.word.cmp(&other.word))
    }
}

pub struct Solution {
    pub guess: [u8; 5],
    pub size: u16,
    pub solution: Vec<(WordMatch, Solution)>,
}

impl Solution {
    pub fn print(&self, line: &mut String) {
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

pub fn solve(
    guesses: &WordDictionary,
    answers: &WordDictionary,
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
    let cf = guesses.try_for_each(|guess| {
        let guess = Guess::new(guess, answers);
        if guess.partition.len() == 1 {
            // learned nothing, not a useful guess
            return ControlFlow::Continue(());
        }
        if let Some(solution) = guess.fast_solution(depth - 1) {
            return ControlFlow::Break(solution);
        }
        if best_guesses.len() < best_guesses.capacity() {
            best_guesses.push(guess);
        } else if guess < *best_guesses.peek().unwrap() {
            best_guesses.pop();
            best_guesses.push(guess);
        }
        ControlFlow::Continue(())
    });
    if let ControlFlow::Break(solution) = cf {
        return Some(solution);
    }
    best_guesses
        .into_par_iter()
        .filter_map(|guess: Guess| guess.slow_solution(guesses, answers, breadth, depth - 1, hard))
        .min_by_key(|solution: &Solution| solution.size)
}

pub fn solve_hard_limited<D: Dictionary>(guess: D::Word, dict: &D, depth: usize) -> Option<u32> {
    if dict.len() == 1 {
        return Some(1);
    }
    if depth == 0 {
        return None;
    }
    let mut partition = dict.partition(guess);
    if partition.len() == dict.len() {
        return Some(2 * partition.len() as u32 - 1);
    }
    partition.remove(&WordMatch::CORRECT);
    partition.into_values().try_fold(1, |total, dict| {
        dict.par_process(total + dict.len() as u32, |guess| {
            solve_hard_limited(guess, &dict, depth - 1)
        })
    })
}

pub fn solve_easy<D: Dictionary>(
    guess: D::Word,
    guesses: &D,
    answers: &D,
    depth: usize,
) -> Option<u32> {
    if answers.len() == 1 {
        return Some(1);
    }
    if depth == 0 {
        return None;
    }
    let mut partition = answers.partition(guess);
    if partition.len() == 1 {
        return None;
    }
    let init = partition
        .remove(&WordMatch::CORRECT)
        .map(|_| 1)
        .unwrap_or(0);
    if partition.len() == answers.len() {
        return Some(2 * partition.len() as u32 - init);
    }
    partition.into_values().try_fold(init, |total, dict| {
        guesses.par_process(total + dict.len() as u32, |guess| {
            solve_easy(guess, guesses, &dict, depth - 1)
        })
    })
}

pub fn solve_hard<D: Dictionary>(
    guess: D::Word,
    guesses: &D,
    answers: &D,
    depth: usize,
) -> Option<u32> {
    if answers.len() == 1 {
        return Some(1);
    }
    if depth == 0 {
        return None;
    }
    let mut partition = answers.partition(guess);
    if partition.len() == 1 {
        return None;
    }
    let init = partition
        .remove(&WordMatch::CORRECT)
        .map(|_| 1)
        .unwrap_or(0);
    if partition.len() == answers.len() {
        return Some(2 * partition.len() as u32 - init);
    }
    let guess_partition = guesses.partition(guess);
    partition
        .into_iter()
        .try_fold(init, |total, (wm, answers)| {
            guess_partition.get(&wm).and_then(|guesses| {
                guesses.par_process(total + answers.len() as u32, |guess| {
                    solve_easy(guess, guesses, &answers, depth - 1)
                })
            })
        })
}
