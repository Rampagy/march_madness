use rand::Rng;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;

const NUM_OF_BRACKETS: usize = 10_000_000;


fn get_round_winners(teams: &Vec<u8>, rng: &mut rand::prelude::ThreadRng) -> Vec<u8> {
    let mut winning_teams: Vec<u8> = Vec::with_capacity(teams.len() / 2);

    for i in (0..teams.len()).step_by(2) {
        let mut left_seed: f32 = (teams[i] % 16) as f32;
        let mut right_seed: f32 = (teams[i+1] % 16) as f32;

        if left_seed == 0.0 {
            left_seed = 16.0;
        }

        if right_seed == 0.0 {
            right_seed = 16.0;
        } 

        let prob_left_seed_wins: f32 = right_seed / (right_seed + left_seed);

        // sample the population given the above weight/probability
        let rand_num: u8 = rng.gen::<u8>();

        if (rand_num as f32) > (prob_left_seed_wins * (255 as f32)) {
            let _ = winning_teams.push(teams[i+1]);
        } else {
            let _ = winning_teams.push(teams[i]);
        }
    }

    return winning_teams;
}


fn main() {
    let file_path: &str = "brackets.txt";

    let mut f: fs::File = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(file_path)
        .unwrap();

    for i in 0..NUM_OF_BRACKETS {
        let mut bracket: String = "".to_string();

        // initialize the starting bracket
        let mut teams: Vec<u8> = vec![
            1,  16,  8,  9,  5, 12,  4, 13,  6, 11,  3, 14,  7, 10,  2, 15, // east
            17, 32, 24, 25, 21, 28, 20, 29, 22, 27, 19, 30, 23, 26, 18, 31, // west
            33, 48, 40, 41, 37, 44, 36, 45, 38, 43, 35, 46, 39, 42, 34, 47, // south
            49, 64, 56, 57, 53, 60, 52, 61, 54, 59, 51, 62, 55, 58, 50, 63, // midwest
        ];

        let mut rng: rand::prelude::ThreadRng = rand::thread_rng();

        while (&teams).len() > 1 {
            teams = get_round_winners(&teams, &mut rng);

            for team in &teams {
                bracket += format!("{} ", team).as_str();
            }

            bracket = bracket.trim().to_string();
            bracket += ";";
        }

        let _ = bracket.trim_end_matches(';');
        bracket += "\n";
        let _ = f.write(bracket.as_bytes());

        if i % 10000 == 0 {
            println!("{}", i);
        }
    }
}
