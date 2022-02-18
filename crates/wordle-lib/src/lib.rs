#![feature(array_chunks)]

pub use config::*;
pub use dict::*;
pub use solve::*;
pub use word_match::*;

mod config;
mod dict;
mod solve;
mod word_match;

pub const LEGAL_GUESSES: &'static [u8] = include_bytes!("../../../guesses.txt");
pub const LEGAL_ANSWERS: &'static [u8] = include_bytes!("../../../answers.txt");
