use rand::Rng;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::collections::HashSet;


const CREATE_NEW_FILE_BRACKET_THRESHOLD: usize = 10_000_000; // after so many brackets start a new file
const FILE_NAME: &str = "brackets";
const WINNING_BRACKET_FILE_NAME: &str = "winning_bracket.txt";


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


fn get_human_readable_bracket(bracket: &[u8; 63]) -> String {
    let mut human_bracket: String = "".to_string();

    let mut games: [u8; 2] = [0, 32];
    let mut games_left: u8 = 32;
    while games_left > 0 {

        for i in games[0]..games[1] {
            human_bracket += format!("{} ", bracket[i as usize]).as_str();
        }

        human_bracket = human_bracket.trim().to_string() + ";";
        games[0] = games[1];
        games_left >>= 1;
        games[1] += games_left;
    }

    human_bracket = human_bracket.trim_end_matches(";").to_string();
    return human_bracket;
}


fn generate_bracket(bracket: &mut [u8; 63]) {
    // initialize the starting bracket
    let mut teams: Vec<u8> = vec![
        1,  16,  8,  9,  5, 12,  4, 13,  6, 11,  3, 14,  7, 10,  2, 15, // east
        17, 32, 24, 25, 21, 28, 20, 29, 22, 27, 19, 30, 23, 26, 18, 31, // west
        33, 48, 40, 41, 37, 44, 36, 45, 38, 43, 35, 46, 39, 42, 34, 47, // south
        49, 64, 56, 57, 53, 60, 52, 61, 54, 59, 51, 62, 55, 58, 50, 63, // midwest
    ];

    let mut rng: rand::prelude::ThreadRng = rand::thread_rng();
    let mut index: usize = 0;
    while (&teams).len() > 1 {
        teams = get_round_winners(&teams, &mut rng);

        for team in &teams {
            bracket[index] = *team;
            index += 1;
        }
    }
}

fn generate_brackets(num_of_brackets: usize) {
    let mut unique_brackets: HashSet<[u8; 63]> = HashSet::with_capacity(num_of_brackets);
    let mut i: usize = 0;
    let mut repeated_brackets: usize = 0;
    let mut file_number: usize = 0;
    let mut file_count: usize = 0;

    // open a file
    let mut f: fs::File = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(format!("{}_{}.txt", file_number, FILE_NAME))
        .unwrap();

    while i < num_of_brackets {
        let mut bracket: [u8; 63] = [0; 63];
        generate_bracket(&mut bracket);

        // only write to the file if it's a unique bracket (inserted into unique_brackets)
        if unique_brackets.insert(bracket) {

            let human_bracket: String = get_human_readable_bracket(&bracket) + "\n";
            let _ = f.write(human_bracket.as_bytes());

            if (i+1) % 1_000_000 == 0 {
                println!("{}", i);
                println!("repeated brackets: {}", repeated_brackets);
            }

            i += 1;
            file_count += 1;
        } else {
            repeated_brackets += 1;
        }

        if file_count >= CREATE_NEW_FILE_BRACKET_THRESHOLD {
            file_count = 0;
            file_number += 1;

            // create a new file (if there are more brackets to create)
            println!("Creating new file..");
            if i < num_of_brackets {
                f = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(format!("{}_{}.txt", FILE_NAME, file_number))
                    .unwrap();
            }
        }
    }

    println!("repeated brackets: {}", repeated_brackets);
}


fn score_brackets() {
    let mut winning_bracket: [u8; 63] = [0; 63];
    // Find the winning bracket text file
    let winning_bracket_file_contents: String = fs::read_to_string(WINNING_BRACKET_FILE_NAME).expect("Should have been able to read winning_bracket.txt");

    let mut team_index: usize = 0;
    for round_results in winning_bracket_file_contents.trim().split(";") {
        for team in round_results.trim().split_ascii_whitespace() {
            winning_bracket[team_index] = team.parse::<u8>().unwrap_or(0);
            team_index+=1;
        }
    }

    // TODO: Parse the contents of the winning bracket text file
    // TODO: Find all of the text files that match this pattern: NUM_brackets*.txt
    // TODO: go line by line of each file scoring each bracket -> store the average and the top ten scores (filename, line number, score)
    // TODO: print out summary and also save summary to file
}


fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        if args.len() > 2 {
            if args[1].trim().to_uppercase() == "--GENERATE" {
                // number is entered in millions (2 is interpreted as 2_000_000)
                let num_brackets: usize = args[2].trim().parse::<usize>().unwrap_or(0) * 1_000_000;

                if num_brackets > 0 {
                    generate_brackets(num_brackets);
                } else {
                    println!("Invalid number of brackets to generate: {}", args[2]);
                }
            }
        } else if args.len() == 2 {
            if args[1].trim().to_uppercase() == "--SCORE" {
                score_brackets();
            }
        }
    } else {
        // not enough arguments, do nothing
        // TODO: print help instead
        println!("Not enough arguments");
    }
}



#[cfg(test)] #[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn test_get_human_readable_bracket() {
        let a: [u8; 63] = [35; 63];
        let b: String = get_human_readable_bracket(&a);
        assert_eq!(b, "35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35;35 35 35 35 35 35 35 35 35 35 35 35 35 35 35 35;35 35 35 35 35 35 35 35;35 35 35 35;35 35;35".to_string());
    }
}