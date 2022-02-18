pub struct Config {
    pub hard: bool,
    pub breadth: usize,
    pub depth: usize,
    pub limit_guesses: bool,
    pub first_guess: Option<[u8; 5]>,
    pub search: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hard: false,
            breadth: 10,
            depth: 6,
            limit_guesses: false,
            first_guess: None,
            search: false,
        }
    }
}

impl Config {
    pub fn from_args(mut args: std::env::Args) -> Self {
        let mut this = Self::default();
        while let Some(arg) = args.next() {
            if arg == "--hard" {
                this.hard = true;
            } else if arg == "--breadth" {
                this.breadth = args.next().unwrap().parse().unwrap();
            } else if arg == "--depth" {
                this.depth = args.next().unwrap().parse().unwrap();
            } else if arg == "--limit-guesses" {
                this.limit_guesses = true;
            } else if arg == "--guess" {
                this.first_guess = args
                    .next()
                    .and_then(|guess| guess.as_bytes().try_into().ok());
            } else if arg == "--search" {
                this.search = true;
            }
        }
        this
    }
}
