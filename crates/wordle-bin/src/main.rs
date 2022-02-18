use std::error::Error;
use wordle_lib::{
    solve, solve_easy, solve_hard, solve_hard_limited, Config, Guess, OffsetDictionary,
    WordDictionary,
};
use wordle_lib::{Dictionary, LEGAL_ANSWERS, LEGAL_GUESSES};

fn main() -> Result<(), Box<dyn Error>> {
    let conf = Config::from_args(std::env::args());
    let answers = &WordDictionary::new(LEGAL_ANSWERS);
    let mut guesses = &WordDictionary::new(LEGAL_GUESSES);
    if conf.limit_guesses {
        guesses = answers;
    }
    if conf.search && conf.hard && conf.limit_guesses {
        let dict = &OffsetDictionary::new();
        let go = |idx, guess: [u8; 5]| {
            if let Some(total) = solve_hard_limited(idx as u16, dict, conf.depth - 1) {
                eprintln!(
                    "{}: {}",
                    String::from_utf8_lossy(&guess),
                    total as f32 / answers.len() as f32
                );
            } else {
                eprintln!("{}: no solution", String::from_utf8_lossy(&guess));
            }
        };
        match conf.first_guess {
            Some(guess) => go(answers.index_of(guess).unwrap(), guess),
            None => {
                let mut i = 0;
                guesses.for_each(|guess| {
                    go(i, guess);
                    i += 1;
                })
            }
        }
    } else if conf.search && conf.hard {
        let go = |guess| {
            if let Some(total) = solve_hard(guess, guesses, answers, conf.depth - 1) {
                eprintln!(
                    "{}: {}",
                    String::from_utf8_lossy(&guess),
                    total as f32 / answers.len() as f32
                );
            } else {
                eprintln!("{}: no solution", String::from_utf8_lossy(&guess));
            }
        };
        match conf.first_guess {
            Some(guess) => go(guess),
            None => guesses.for_each(|guess| go(guess)),
        }
    } else if conf.search {
        let go = |guess| {
            if let Some(total) = solve_easy(guess, guesses, answers, conf.depth - 1) {
                eprintln!(
                    "{}: {}",
                    String::from_utf8_lossy(&guess),
                    total as f32 / answers.len() as f32
                );
            } else {
                eprintln!("{}: no solution", String::from_utf8_lossy(&guess));
            }
        };
        match conf.first_guess {
            Some(guess) => go(guess),
            None => guesses.for_each(|guess| go(guess)),
        }
    } else {
        let solution = if let Some(guess) = conf.first_guess {
            let guess = Guess::new(guess, answers);
            guess.slow_solution(guesses, answers, conf.breadth, conf.depth - 1, conf.hard)
        } else {
            solve(guesses, answers, conf.breadth, conf.depth, conf.hard)
        };
        if let Some(solution) = solution {
            solution.print(&mut String::new());
            eprintln!("mean: {}", solution.size as f32 / answers.len() as f32);
        } else {
            eprintln!("no solution");
        }
    }
    Ok(())
}
