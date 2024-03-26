use rand::Rng;
use std::env;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{prelude::*, BufReader, BufWriter, Write};
use std::collections::HashSet;
use glob::glob;
use rand_distr::{Normal, Distribution};


const CREATE_NEW_FILE_BRACKET_THRESHOLD: usize = 20_000_000; // after so many brackets start a new file
const FILE_NAME: &str = "brackets";
const WINNING_BRACKET_FILE_NAME: &str = "winning_bracket.txt";
const BINARY_BYTE_OFFSET: u8 = 32;
const BRACKET_RESOLUTION: usize = 1_000_000; // minimum number (and step) of brackets
const FILE_READ_WRITE_BUFFER_SIZE: usize = 20_971_520; // 20 MB

#[derive(PartialEq)] #[repr(u8)]
enum ProbabilityMethod {
    Year2024 = 0,
    Year2025 = 1,
}


fn get_round_winners(teams: &Vec<u8>, rng: &mut rand::prelude::ThreadRng, method: &ProbabilityMethod) -> Vec<u8> {
    let mut winning_teams: Vec<u8> = Vec::with_capacity(teams.len() / 2);
    let mut distributions: [Normal<f64>; 16] = [Normal::new(0.0, 1.0).unwrap(); 16];

    if *method == ProbabilityMethod::Year2025 {
        let mut mean: f64 = 85.0;
        for i in 0..16 as usize {
            distributions[i] = Normal::new(mean, 10.0).unwrap();
            mean -= 2.0;
        }
    }

    for i in (0..teams.len()).step_by(2) {
        let mut left_seed: usize = (teams[i] % 16) as usize;
        let mut right_seed: usize = (teams[i+1] % 16) as usize;

        if left_seed == 0 {
            left_seed = 16;
        }

        if right_seed == 0 {
            right_seed = 16;
        }

        match method {
            ProbabilityMethod::Year2024 => {
                let prob_left_seed_wins: f32 = right_seed as f32 / (right_seed as f32 + left_seed as f32);

                // sample the population given the above weight/probability
                let rand_num: u32 = rng.gen::<u32>();
                if (rand_num as f32) > (prob_left_seed_wins * (u32::MAX as f32)) {
                    // right seed wins
                    let _ = winning_teams.push(teams[i+1]);
                } else {
                    // left seed wins
                    let _ = winning_teams.push(teams[i]);
                }
            },
            ProbabilityMethod::Year2025 => {
                let left_seed_points: f64 = distributions[left_seed-1].sample(rng);
                let right_seed_points: f64 = distributions[right_seed-1].sample(rng);

                if left_seed_points > right_seed_points {
                    // left seed scored more points in the game
                    winning_teams.push(teams[i]);
                } else {
                    // right seed scored more points in the game
                    winning_teams.push(teams[i+1]);
                }
            },
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


fn generate_bracket(bracket: &mut [u8; 63], method: &ProbabilityMethod) {
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
        teams = get_round_winners(&teams, &mut rng, &method);

        for team in &teams {
            bracket[index] = *team;
            index += 1;
        }
    }
}


fn encode_to_bytes(bracket: &[u8; 63]) -> [u8; 64] {
    let mut encoded_bracket: [u8; 64] = [0; 64];

    for (idx, team) in bracket.into_iter().enumerate() {
        encoded_bracket[idx] = team + BINARY_BYTE_OFFSET;
    }

    encoded_bracket[63] = b'\n';
    return encoded_bracket;
}


fn decode_bytes(bracket: &[u8; 63]) -> [u8; 63] {
    return bracket.map(|x| x-BINARY_BYTE_OFFSET);
}


fn generate_brackets(num_of_brackets: usize, method: &ProbabilityMethod) {
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
        .open(format!("{}_{}.bin", file_number, FILE_NAME))
        .unwrap();
    let mut writer: BufWriter<File> = BufWriter::with_capacity(FILE_READ_WRITE_BUFFER_SIZE, f);

    while i < num_of_brackets {
        let mut bracket: [u8; 63] = [0; 63];
        generate_bracket(&mut bracket, &method);

        // only write to the file if it's a unique bracket (inserted into unique_brackets)
        if unique_brackets.insert(bracket) {
            let _ = writer.write(&encode_to_bytes(&bracket));

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
            if i < num_of_brackets {
                println!("Creating new file..");
                f = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(format!("{}_{}.bin", file_number, FILE_NAME))
                    .unwrap();
                writer = BufWriter::with_capacity(FILE_READ_WRITE_BUFFER_SIZE, f);
            }
        }
    }

    println!("repeated brackets: {}", repeated_brackets);
}


fn parse_bracket(raw_bracket: &String) -> [u8; 63] {
    let mut bracket: [u8; 63] = [0; 63];
    let mut team_index: usize = 0;
    for round_results in raw_bracket.trim().split(";") {
        for team in round_results.trim().split_ascii_whitespace() {
            bracket[team_index] = team.parse::<u8>().unwrap_or(0);
            team_index+=1;
        }
    }

    return bracket;
}


fn score_bracket(bracket: &[u8; 63], winning_bracket: &[u8; 63]) -> u8 {
    let mut score: u8 = 0;
    let mut round_length: u8 = 32; // 32 results in the first round
    let mut round_points: u8 = 1; // points per correct team

    let mut rounds: u8 = 0;
    for (team1, winning_team) in bracket.into_iter().zip(winning_bracket.into_iter()) {
        if team1 == winning_team {
            score += round_points;
        }

        rounds += 1;
        if rounds >= round_length {
            round_points <<= 1;
            round_length += 32 / round_points;
        }
    }

    return score;
}


fn calc_max_bracket_points(winning_bracket: &[u8; 63]) -> u8 {
    let mut score: u8 = 0;
    let mut round_length: u8 = 32; // 32 results in the first round
    let mut round_points: u8 = 1; // points per correct team

    let mut rounds: u8 = 0;
    for winning_team in winning_bracket.into_iter() {
        if *winning_team != 0 {
            score += round_points;
        }

        rounds += 1;
        if rounds >= round_length {
            round_points <<= 1;
            round_length += 32 / round_points;
        }
    }

    return score;
}


fn score_brackets() {
    let winning_bracket: [u8; 63];
    let max_bracket_score: u8;
    let mut total_brackets: usize = 0;
    let mut perfect_brackets: usize = 0;
    let mut bracket_score_accumulator: usize = 0;

    { // Find the winning bracket text file
        let winning_bracket_file_contents: String = fs::read_to_string(WINNING_BRACKET_FILE_NAME).expect("Should have been able to read winning_bracket.txt");
        winning_bracket = parse_bracket(&winning_bracket_file_contents);
        println!("winning bracket: {}\n", get_human_readable_bracket(&winning_bracket));
        max_bracket_score = calc_max_bracket_points(&winning_bracket);
    }

    let mut top_brackets: Vec<(u8, usize, String, [u8; 63])> = Vec::with_capacity(11);
    let mut score_distribution: [usize; 193] = [0; 193];
    for entry in glob("*_brackets*.txt").unwrap()
                                                .chain(glob("*_brackets*.bin").unwrap()) {

        let scoring_bracket_filename: String = entry.unwrap().into_os_string().into_string().unwrap();

        let file: File = File::open(&scoring_bracket_filename).unwrap();
        let reader: BufReader<File> = BufReader::with_capacity(FILE_READ_WRITE_BUFFER_SIZE, file);

        for (line_number, line) in reader.lines().enumerate() {
            let bracket: [u8; 63] =  if scoring_bracket_filename.trim_end().to_ascii_uppercase().ends_with(".TXT") {
                // legacy text format
                parse_bracket(&line.unwrap_or("".to_string()))
            } else {
                // binary encoded format
                let mut temp_bytes: [u8; 63] = [0; 63];
                for (i, ch) in line.unwrap_or("".to_string()).chars().enumerate() {
                    if i > 63 { break; }
                    temp_bytes[i] = ch as u8;
                }
                decode_bytes(&temp_bytes)
            };

            let score: u8 = score_bracket(&bracket, &winning_bracket);
            bracket_score_accumulator = bracket_score_accumulator.saturating_add(score as usize);
            score_distribution[score as usize] += 1;

            // track number of perfect brackets
            perfect_brackets += if score == max_bracket_score { 1 } else { 0 };
            total_brackets += 1;

            if top_brackets.len() < 10 || score > top_brackets[top_brackets.len()-1].0 {
                top_brackets.push(  (score, line_number, scoring_bracket_filename.clone(), bracket)  );
                top_brackets.sort_by_key(|x| (*x).0);
                top_brackets.reverse();

                // if the length is longer than 10, remove the last one as it's not top 10
                if top_brackets.len() > 10 {
                    top_brackets.remove(10);
                }
            }
        }
    }

    let percent_perfect_brackets: f64 = if total_brackets > 0 {
        (perfect_brackets as f64 / total_brackets as f64) * 100 as f64
    } else { 0 as f64 };

    let average_bracket_score: f64 = if total_brackets > 0 {
        bracket_score_accumulator as f64 / total_brackets as f64
    } else { 0 as f64 };

    // track populations of each score
    let mut lowest_score: usize = usize::MAX;
    let mut highest_population: usize = 0;
    let mut highest_population_score: usize = 0;
    for (score, population) in score_distribution.into_iter().enumerate() {
        if score < lowest_score  && population > 0{
            lowest_score = score;
        }

        if population > highest_population {
            highest_population = population;
            highest_population_score = score;
        }
    }

    let highest_population_percent: f64 = if total_brackets > 0 {
        (highest_population as f64 / total_brackets as f64) * 100 as f64
    } else { 0 as f64 };

    println!("Total brackets: {}", total_brackets);
    println!("Perfect brackets: {} ({:.2}%)", perfect_brackets, percent_perfect_brackets);
    println!("Average bracket score: {:.1}", average_bracket_score);
    println!("Most common bracket score: {} ({} brackets or {:.1}%)\n", highest_population_score, highest_population, highest_population_percent);

    for i in (top_brackets[0].0.saturating_sub(2)..=top_brackets[0].0).rev() {
        println!("Brackets with {:3} points: {}", i, score_distribution[i as usize]);
    }

    for i in (lowest_score..=lowest_score.saturating_add(2)).rev() {
        println!("Brackets with {:3} points: {}", i, score_distribution[i]);
    }
    println!();

    for (place, bracket_stats) in top_brackets.iter().enumerate() {
        println!("place: {:<2}   score: {:<3}   line_number: {:<12}   file: {:<16}", place+1, bracket_stats.0, bracket_stats.1+1, bracket_stats.2);
        println!("bracket: {}\n",  get_human_readable_bracket(&bracket_stats.3));
    }
}


fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        if (args.len() == 3 || args.len() == 4) && args[1].trim().to_uppercase() == "--GENERATE" {
            // number is entered in millions (2 is interpreted as 2_000_000)
            let num_brackets: usize = args[2].trim().parse::<usize>().unwrap_or(0) * BRACKET_RESOLUTION;
            let method: ProbabilityMethod = if args.len() == 4 {
                if args[3].trim().parse::<u8>().unwrap_or(0) == 0 {
                    // only use legacy method if explicitly stated
                    ProbabilityMethod::Year2024
                } else { ProbabilityMethod::Year2025 }
            } else { ProbabilityMethod::Year2025 };

            if num_brackets > 0 {
                generate_brackets(num_brackets, &method);
            } else {
                println!("Invalid number of brackets to generate: {}", args[2]);
            }
        } else if args.len() == 2 {
            if args[1].trim().to_uppercase() == "--SCORE" {
                score_brackets();
            } else {
                println!("Unrecognized argument: {}", args[1]);
            }
        } else {
            println!("Too many arguments!");
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

    #[test]
    fn test_score_bracket() {
        let winning_bracket: &str = "1 8 5 4 11 14 7 2 17 24 28 29 22 19 23 18 33 41 44 45 38 35 42 34 49 57 53 52 59 51 55 50;1 4 11 7 17 29 19 18 33 44 38 34 49 52 51 50;1 7 17 18 33 34 49 50;1 17 33 49;17 33;17";
        let test_bracket: &str = "0 8 5 4 11 14 7 2 17 24 28 29 22 19 23 18 33 41 44 45 38 35 42 34 49 57 53 52 59 51 55 50;1 0 11 7 17 29 19 18 33 44 38 34 49 52 51 50;1 7 0 18 33 34 49 50;1 17 33 0;17 0;0";

        let winning_bracket_encoded: [u8; 63] = parse_bracket(&winning_bracket.to_string());
        let test_bracket_encoded: [u8; 63] = parse_bracket(&test_bracket.to_string());

        assert_eq!(score_bracket(&winning_bracket_encoded, &winning_bracket_encoded), 192);
        assert_eq!(score_bracket(&winning_bracket_encoded, &test_bracket_encoded), 129);
    }
}