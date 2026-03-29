use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{prelude::*, BufReader, BufWriter, Write};
use std::time::Instant;
use std::collections::HashSet;
use glob::glob;
use rand_distr::{Normal, Distribution};
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;

/// Generates and scores march madness brackets
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Generate new brackets.
    /// If not provided it scores existing brackets.
    #[arg(short, long, default_value_t = false)]
    generate: bool,

    /// Number of brackets to generate (in millions)
    #[arg(short, long, default_value_t = 1)]
    count: usize,
}

const UNIQUE_BRACKETS_MAX_SIZE: usize = 64*1024*1024*1024; // 64 or 32 gibibytes?
const CREATE_NEW_FILE_BRACKET_THRESHOLD: usize = 120_000_000; // after so many brackets start a new file
const FILE_NAME: &str = "brackets";
const WINNING_BRACKET_FILE_NAME: &str = "winning_bracket.txt";
const BRACKET_RESOLUTION: usize = 1_000_000; // minimum number (and step) of brackets
const FILE_READ_WRITE_BUFFER_SIZE: usize = 16*1024*1024; // 8 mebibytes
const STARTING_BRACKET: [u8; 64] = [
    1,  16,  8,  9,  5, 12,  4, 13,  6, 11,  3, 14,  7, 10,  2, 15, // east
    17, 32, 24, 25, 21, 28, 20, 29, 22, 27, 19, 30, 23, 26, 18, 31, // west
    33, 48, 40, 41, 37, 44, 36, 45, 38, 43, 35, 46, 39, 42, 34, 47, // south
    49, 64, 56, 57, 53, 60, 52, 61, 54, 59, 51, 62, 55, 58, 50, 63, // midwest
];
const SEED1_MEAN_SCORE: f64 = 85.0;
const TEAMS_SCORE_STDEV: f64 = 10.0;

fn get_round_winners(teams: &mut [u8; 64], rng: &mut rand::prelude::ThreadRng, teams_len: u8, distribution: &Normal<f64>) {
    let mut winning_teams: [u8; 64] = [0; 64];

    for i in (0..teams_len as usize).step_by(2) {
        let mut left_seed: usize = (teams[i] % 16) as usize;
        let mut right_seed: usize = (teams[i+1] % 16) as usize;

        if left_seed == 0 {
            left_seed = 16;
        }

        if right_seed == 0 {
            right_seed = 16;
        }

        let left_mean: f64 = SEED1_MEAN_SCORE - (left_seed - 1) as f64;
        let right_mean: f64 = SEED1_MEAN_SCORE - (right_seed - 1) as f64;

        let left_seed_points: f64 = left_mean + distribution.sample(rng);
        let right_seed_points: f64 = right_mean + distribution.sample(rng);

        if left_seed_points > right_seed_points {
            // left seed scored more points in the game
            winning_teams[i/2] = teams[i];
        } else {
            // right seed scored more points in the game
            winning_teams[i/2] = teams[i+1];
        }
    }

    for i in 0..teams_len as usize {
        teams[i] = winning_teams[i];
    }
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


fn generate_bracket(bracket: &mut [u8; 63], distribution: &Normal<f64>) {
    // initialize the starting bracket
    let mut teams: [u8; 64] = [
        1,  16,  8,  9,  5, 12,  4, 13,  6, 11,  3, 14,  7, 10,  2, 15, // east
        17, 32, 24, 25, 21, 28, 20, 29, 22, 27, 19, 30, 23, 26, 18, 31, // west
        33, 48, 40, 41, 37, 44, 36, 45, 38, 43, 35, 46, 39, 42, 34, 47, // south
        49, 64, 56, 57, 53, 60, 52, 61, 54, 59, 51, 62, 55, 58, 50, 63, // midwest
    ];

    let mut rng: rand::prelude::ThreadRng = rand::rng();
    let mut index: usize = 0;
    let mut team_length: u8 = 64;
    while team_length > 1 {
        get_round_winners(&mut teams, &mut rng, team_length, &distribution);
        team_length /= 2;

        for i in 0..team_length as usize {
            bracket[index] = teams[i];
            index += 1;
        }
    }
}


fn encode_to_bytes(bracket: &[u8; 63]) -> u64 {
    let mut encoded_bracket: u64 = 0;

    for (idx, top_team) in bracket.into_iter().enumerate() {
        // 0 bit if first (top) team won, 1 bit if second (bottom) team won
        let encoded_bit: u8 = 
            if idx < 32 { // first  round
                // compare the round 1 winning team to the top position in the bracket
                // if the team that won is not the top team encode a 1 bit because the second (bottom) team won
                (STARTING_BRACKET[idx*2] != *top_team) as u8 
            } else { // the rest of the rounds
                // compare the rounds winning team to the top position in the bracket
                // if the team that won is not the top team encode a 1 bit because the second (bottom) team won
                (bracket[2*(idx-32)] != *top_team) as u8
            };

        // add the bit to the bracket
        encoded_bracket = (encoded_bracket << 1) | (1 * encoded_bit) as u64;
    }

    // left justify the 63 bits
    return encoded_bracket << 1;
}


fn decode_and_score(bracket: &[u8; 8], winning_bracket: &[u8; 63], decoded_bracket: &mut [u8; 63]) -> u8 {
    let mut bit_count: usize = 0; // also game_count
    let mut round_size: usize = 32;
    let mut round_count: usize = 0;
    let mut round_score: u8 = 1;
    let mut score: u8 = 0;

    for &b in bracket {
        let mut mask: u8 = 0x80;
        while mask > 0 && bit_count < 63 {

            let second_team_offset: usize = ((mask & b) != 0) as usize;
            decoded_bracket[bit_count] = if bit_count < 32 {
                // the first round needs to get the winners from the starting bracket
                 STARTING_BRACKET[2*bit_count + second_team_offset]
            } else {
                // the subsequent rounds need to get the winners from the previous round winners
                decoded_bracket[2*(bit_count - 32) + second_team_offset]
            };

            // calculate the score
            score += round_score * (decoded_bracket[bit_count] == winning_bracket[bit_count]) as u8;

            mask >>= 1;
            bit_count += 1;
            round_count += 1;

            let should_advance: u8 = (round_count >= round_size) as u8;
            round_count *= 1 - should_advance as usize;
            round_size >>= should_advance;
            round_score <<= should_advance;
        }
    }

    return score;
}


fn generate_brackets(num_of_brackets: usize) {
    let unique_brackets_limit: usize = (UNIQUE_BRACKETS_MAX_SIZE / 8).min(num_of_brackets) + 2*CREATE_NEW_FILE_BRACKET_THRESHOLD; // convert from bytes to brackets and 2GB of buffer 
    let mut unique_brackets: HashSet<u64> = HashSet::with_capacity(unique_brackets_limit);  // this function is only guaranteed to give you at least this much, but it could give you more (so much more it uses all your memory...)
    let mut i: usize = 0;
    let mut repeated_brackets: HashSet<u64> = HashSet::new();
    let mut file_number: usize = 0;
    let mut file_count: usize = 0;

    // check to see if hashset overallocated, if so reduce until it's below the max limit
    let mut attempted_new_capacity: usize = unique_brackets_limit;
    println!("attempted bracket capacity: {}, got bracket capacity: {}", attempted_new_capacity, unique_brackets.capacity());
    while unique_brackets.capacity() > unique_brackets_limit {
        // try shrinking the capacity
        attempted_new_capacity = (attempted_new_capacity as f64 * 0.9) as usize;
        unique_brackets.shrink_to(attempted_new_capacity);
        println!("attempted bracket capacity: {}, got bracket capacity: {}", attempted_new_capacity, unique_brackets.capacity());
    }

    // create single distribution with mean 0 and stdev 10
    let distribution: Normal<f64> = Normal::new(0.0, TEAMS_SCORE_STDEV).unwrap();

    let m: MultiProgress = MultiProgress::new();
    let sty: ProgressStyle = ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>12}/{len:12} ({eta_precise}) {msg}",
        )
        .unwrap()
        .progress_chars("##-");
    let progress_bar: ProgressBar = m.add(ProgressBar::new(num_of_brackets as u64));
    progress_bar.set_style(sty.clone());
    progress_bar.set_message("generating");

    // open a file
    let mut f: fs::File = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(format!("{}_{}.bin", file_number, FILE_NAME))
        .unwrap();
    let mut writer: BufWriter<File> = BufWriter::with_capacity(FILE_READ_WRITE_BUFFER_SIZE, f);

    progress_bar.force_draw();
    while i < num_of_brackets {
        let mut bracket: [u8; 63] = [0; 63];
        generate_bracket(&mut bracket, &distribution);
        let encoded_bracket: u64 = encode_to_bytes(&bracket);

        // only write to the file if it's a unique bracket (inserted into unique_brackets)
        if unique_brackets.insert(encoded_bracket) {
            let _ = writer.write(&encoded_bracket.to_be_bytes());

            i += 1;
            file_count += 1;
            progress_bar.inc(1);
        } else {
            repeated_brackets.insert(encoded_bracket);
            progress_bar.set_message(format!("{} repeats", repeated_brackets.len()));
            progress_bar.force_draw();
        }

        if file_count >= CREATE_NEW_FILE_BRACKET_THRESHOLD {
            file_count = 0;
            file_number += 1;

            progress_bar.set_message(format!("closing file"));
            progress_bar.force_draw();

            // force write of any remaining bytes to the file
            writer.flush().expect("unable to write to file");

            progress_bar.set_message(format!("optimizing cache"));
            progress_bar.force_draw();

            // check to see if hashset is getting too big and remove some if necessary
            remove_brackets(&mut unique_brackets, &mut repeated_brackets);

            // create a new file (if there are more brackets to create)
            if i < num_of_brackets {
                progress_bar.set_message(format!("creating new file"));
                progress_bar.force_draw();

                f = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(format!("{}_{}.bin", file_number, FILE_NAME))
                    .unwrap();
                writer = BufWriter::with_capacity(FILE_READ_WRITE_BUFFER_SIZE, f);
            }

            progress_bar.set_message(format!("{} repeats", repeated_brackets.len()));
            progress_bar.force_draw();
        }
    }

    // force write of any remaining bytes to the file
    writer.flush().expect("unable to write to file");

    progress_bar.finish();
    println!("Bracket generation complete!");
}

fn remove_brackets(unique_brackets: &mut HashSet<u64>, repeated_brackets: &mut HashSet<u64>) {
    if (unique_brackets.capacity()>>1) + (unique_brackets.capacity()>>2) < unique_brackets.len() { // when it gets to 75% full or more
        let target_size: usize = (unique_brackets.capacity()>>1) + (unique_brackets.capacity()>>3); // clear it down to at least 62.5%
        let need_to_remove: usize = (unique_brackets.len() - target_size) + repeated_brackets.len();

        let mut removed = 0;
        let to_keep: HashSet<u64> = unique_brackets
            .drain()
            .filter(|b| {
                if removed >= need_to_remove {
                    true // keep this one - we've removed enough
                } else if repeated_brackets.contains(b) {
                    removed += 1; // count it as removed (space reserved) but keep it
                    true // keep this one
                } else {
                    removed += 1;
                    false // remove this one
                }
            })
            .collect();
        
        *unique_brackets = to_keep;
    }
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


fn print_results<'a, 'b>(perfect_brackets: usize, total_brackets: usize, bracket_score_accumulator: usize, max_bracket_score: u8, 
                         score_distribution: &'a [usize; 193], top_brackets: &'b Vec<(u8, usize, String, [u8; 63])>) {
    const TOP_NUM_BRACKETS_TO_SHOW: u8 = 3;
    const BOT_NUM_BRACKETS_TO_SHOW: usize = 3;

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
        if score < lowest_score  && *population > 0 {
            lowest_score = score;
        }

        if *population > highest_population {
            highest_population = *population;
            highest_population_score = score;
        }
    }

    let highest_population_percent: f64 = if total_brackets > 0 {
        (highest_population as f64 / total_brackets as f64) * 100 as f64
    } else { 0 as f64 };

    println!("Total brackets: {}", total_brackets);
    println!("Perfect brackets: {} ({:.2}%)", perfect_brackets, percent_perfect_brackets);
    println!("Max Bracket score: {}", max_bracket_score);
    println!("Average bracket score: {:.1}", average_bracket_score);
    println!("Most common bracket score: {} ({} brackets or {:.1}%)\n", highest_population_score, highest_population, highest_population_percent);

    for i in (top_brackets[0].0.saturating_sub(TOP_NUM_BRACKETS_TO_SHOW-1)..=top_brackets[0].0).rev() {
        println!("Brackets with {:3} points: {}", i, score_distribution[i as usize]);
    }

    for i in (lowest_score..=lowest_score.saturating_add(BOT_NUM_BRACKETS_TO_SHOW-1)).rev() {
        println!("Brackets with {:3} points: {}", i, score_distribution[i]);
    }
    println!();

    for (place, bracket_stats) in top_brackets.iter().enumerate() {
        println!("place: {:<2}   score: {:<3}   starting_byte: {:<12}   file: {:<16}", place+1, bracket_stats.0, bracket_stats.1, bracket_stats.2);
        println!("bracket: {}\n",  get_human_readable_bracket(&bracket_stats.3));
    }
}


// Structure to hold scoring results from a single file
#[derive(Clone)]
struct FileScoreResult {
    total_brackets: usize,
    perfect_brackets: usize,
    bracket_score_accumulator: usize,
    score_distribution: [usize; 193],
    top_brackets: Vec<(u8, usize, String, [u8; 63])>,
}


fn score_single_file(
    filename: &str,
    winning_bracket: &[u8; 63],
    max_bracket_score: u8,
) -> FileScoreResult {
    let mut total_brackets: usize = 0;
    let mut perfect_brackets: usize = 0;
    let mut bracket_score_accumulator: usize = 0;
    let mut score_distribution: [usize; 193] = [0; 193];
    let mut top_brackets: Vec<(u8, usize, String, [u8; 63])> = Vec::with_capacity(11);

    let file = File::open(&filename).unwrap();
    let mut reader: BufReader<File> =
        BufReader::with_capacity(FILE_READ_WRITE_BUFFER_SIZE, file.try_clone().unwrap());

    let mut temp_bytes: [u8; 8] = [0; 8];
    let mut bytes: usize = 0;
    while reader.read_exact(&mut temp_bytes).is_ok() {
        let mut bracket: [u8; 63] = [0; 63];
        let score: u8 = decode_and_score(&temp_bytes, winning_bracket, &mut bracket);

        bracket_score_accumulator += score as usize;
        score_distribution[score as usize] += 1;

        perfect_brackets += (score == max_bracket_score) as usize;
        total_brackets += 1;

        if top_brackets.len() < 10 || score > top_brackets[top_brackets.len() - 1].0 {
            top_brackets.push((score, bytes, filename.to_string(), bracket));
            top_brackets.sort_by_key(|x| x.0);
            top_brackets.reverse();

            if top_brackets.len() > 10 {
                top_brackets.remove(10);
            }
        }

        bytes += 8;
    }

    FileScoreResult {
        total_brackets,
        perfect_brackets,
        bracket_score_accumulator,
        score_distribution,
        top_brackets,
    }
}


fn score_brackets() {
    let winning_bracket: [u8; 63];
    let max_bracket_score: u8;

    { // Find the winning bracket text file
        let winning_bracket_file_contents: String = fs::read_to_string(WINNING_BRACKET_FILE_NAME).expect("Should have been able to read winning_bracket.txt");
        winning_bracket = parse_bracket(&winning_bracket_file_contents);
        println!("winning bracket: {}\n", get_human_readable_bracket(&winning_bracket));
        max_bracket_score = calc_max_bracket_points(&winning_bracket);
    }

    // Collect all bracket files first
    let bracket_files: Vec<String> = glob("*_brackets*.txt")
        .unwrap()
        .chain(glob("*_brackets*.bin").unwrap())
        .map(|entry| entry.unwrap().into_os_string().into_string().unwrap())
        .collect();

    let total_files = bracket_files.len();
    println!("Processing {} bracket files in parallel...\n", total_files);

    // Process all files in parallel using rayon
    let file_results: Vec<FileScoreResult> = bracket_files
        .par_iter()
        .map(|filename| {
            score_single_file(filename, &winning_bracket, max_bracket_score)
        })
        .collect();

    // Merge results from all files
    let mut total_brackets: usize = 0;
    let mut perfect_brackets: usize = 0;
    let mut bracket_score_accumulator: usize = 0;
    let mut score_distribution: [usize; 193] = [0; 193];
    let mut top_brackets: Vec<(u8, usize, String, [u8; 63])> = Vec::with_capacity(11);

    for result in file_results {
        total_brackets += result.total_brackets;
        perfect_brackets += result.perfect_brackets;
        bracket_score_accumulator += result.bracket_score_accumulator;

        // Merge score distributions
        for (idx, count) in result.score_distribution.iter().enumerate() {
            score_distribution[idx] += count;
        }

        // Merge top brackets
        for bracket in result.top_brackets {
            top_brackets.push(bracket);
        }
    }

    // Keep only top 10 overall
    top_brackets.sort_by_key(|x| x.0);
    top_brackets.reverse();
    if top_brackets.len() > 10 {
        top_brackets.truncate(10);
    }

    println!();

    // print results for all files
    print_results(perfect_brackets, total_brackets, bracket_score_accumulator, max_bracket_score, &score_distribution, &top_brackets);
}


fn main() {
    let args: Args = Args::parse();

    if args.generate {
        // number is entered in millions (2 is interpreted as 2_000_000)
        let num_brackets: usize = args.count * BRACKET_RESOLUTION;

        if num_brackets > 0 {
            generate_brackets(num_brackets);
        } else {
            println!("Invalid number of brackets to generate: {}", args.count);
        }
    } else {
        let now: Instant = Instant::now();
        score_brackets();
        let elapsed: std::time::Duration = now.elapsed();
        println!("elapsed time: {:.2?}", elapsed);
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
    fn test_encode_bracket() {
        let test_bracket: [u8; 63] = [ // expanded for clarity
            1, 8, 5, 4, 11, 14, 7, 2, 17, 24, 28, 29, 22, 19, 23, 18, 33, 41, 44, 45, 38, 35, 42, 34, 49, 57, 53, 52, 59, 51, 55, 50, 
              1,    5,    11,     7,     17,    29,     19,     18,     33,      44,    38,     34,      49,     52,    51,     50, 
                 1,            7,           17,             18,              33,             34,             49,             50, 
                        1,                           17,                             33,                            49, 
                                     17,                                                            33, 
                                                                     33
        ];

        let test_bracket_encoded: [u8; 8] = encode_to_bytes(&test_bracket).to_be_bytes();

        let answer: [u8; 8] = [12, 48, 114, 72, 7, 23, 85, 10];
        assert!(test_bracket_encoded.iter().eq(answer.iter()));
    }

    #[test]
    fn test_decode_bracket() {
        let answer: [u8; 63] = [1, 9, 5, 13, 6, 3, 10, 2, 17, 25, 21, 20, 22, 19, 26, 18, 33, 40, 37, 
                                36, 38, 35, 42, 34, 49, 57, 53, 61, 54, 51, 55, 50, 1, 5, 6, 10, 17, 
                                20, 19, 18, 40, 37, 35, 34, 49, 61, 51, 50, 5, 6, 17, 19, 40, 34, 49, 
                                50, 5, 17, 34, 49, 17, 49, 17];
        let test_bracket: [u8; 8] = [0x52, 0x42, 0x02, 0x50, 0x07, 0xB7, 0x85, 0x2C];
        let mut test_bracket_decoded: [u8; 63] = [0; 63];
        let _ = decode_and_score(&test_bracket, &[0; 63], &mut test_bracket_decoded);
        assert!(test_bracket_decoded.iter().eq(answer.iter()));
    }

    #[test]
    fn test_score_bracket() {
        // this test uses the encode and decode functionality from the two previous tests, so if one of those is failing and 
        // this is failing it may be a cascading failure and it is worth to to resolve the encode/decode test ebfore this test

        let test_bracket: &str = "1 8 5 4 11 14 7 2 17 24 28 29 22 19 23 18 33 41 44 45 38 35 42 34 49 57 53 52 59 51 55 50;1 4 11 7 17 29 19 18 33 44 38 34 49 52 51 50;1 7 17 18 33 34 49 50;1 17 33 49;17 33;17";
        let winning_bracket: &str = "0 8 5 4 11 14 7 2 17 24 28 29 22 19 23 18 33 41 44 45 38 35 42 34 49 57 53 52 59 51 55 50;1 0 11 7 17 29 19 18 33 44 38 34 49 52 51 50;1 7 0 18 33 34 49 50;1 17 33 0;17 0;0";

        let winning_bracket_encoded: [u8; 63] = parse_bracket(&winning_bracket.to_string());
        let test_bracket_encoded: [u8; 8] = encode_to_bytes(&parse_bracket(&test_bracket.to_string())).to_be_bytes();
        let mut test_bracket_decoded: [u8; 63] = [0; 63];

        // check scoring functionality
        assert_eq!(decode_and_score(&test_bracket_encoded, &winning_bracket_encoded, &mut test_bracket_decoded), 129);
    }


    fn encode_to_bytes_binary_version1(bracket: &[u8; 63]) -> [u8; 64] {
        let mut encoded_bracket: [u8; 64] = [0; 64];
    
        for (idx, team) in bracket.into_iter().enumerate() {
            encoded_bracket[idx] = team + 32;
        }
    
        encoded_bracket[63] = b'\n';
        return encoded_bracket;
    }

    #[test]
    fn test_print_example() {
        // this is not a test, but it was convenient to put it here

        let test_bracket: [u8; 63] = [1, 9, 5, 13, 6, 3, 10, 2, 17, 25, 21, 20, 22, 19, 26, 18, 33, 40, 37, 36, 38, 35, 42, 34, 49, 57, 53, 61, 54, 51, 55, 50, 1, 5, 6, 10, 17, 20, 19, 18, 40, 37, 35, 34, 49, 61, 51, 50, 5, 6, 17, 19, 40, 34, 49, 50, 5, 17, 34, 49, 17, 49, 17];
        let test_bracket_encoded1: [u8; 64] = encode_to_bytes_binary_version1(&test_bracket);
        let test_bracket_encoded2: [u8; 8] = encode_to_bytes(&test_bracket).to_be_bytes();

        for t in test_bracket {
            print!("{} ", t);
        }
        println!();

        for t in test_bracket_encoded1 {
            print!("{}", t as char);
        }

        for t in test_bracket_encoded2 {
            print!("0x{:0>2X} ", t);
        }
        println!();

    }

    #[test]
    fn test_bracket_generation() {
        let distribution: Normal<f64> = Normal::new(0.0, TEAMS_SCORE_STDEV).unwrap();
        let mut bracket: [u8; 63] = [0; 63];
        
        generate_bracket(&mut bracket, &distribution);
        
        // Verify round 1: Each winner should be one of the two starting teams
        for game_idx in 0..32 {
            let winner: u8 = bracket[game_idx];
            let team1: u8 = STARTING_BRACKET[2 * game_idx];
            let team2: u8 = STARTING_BRACKET[2 * game_idx + 1];
            
            assert!(winner == team1 || winner == team2,
                "Round 1 game {} winner {} must be either {} or {}",
                game_idx, winner, team1, team2);
        }
        
        // Verify rounds 2-6: Each winner should be one of the two previous round winners
        let mut round_start: usize = 32;
        let mut round_size: usize = 16;
        let mut prev_round_start: usize = 0;
        
        for round in 2..=6 {
            for game_idx in 0..round_size {
                let winner: u8 = bracket[round_start + game_idx];
                let team1: u8 = bracket[prev_round_start + 2 * game_idx];
                let team2: u8 = bracket[prev_round_start + 2 * game_idx + 1];
                
                assert!(winner == team1 || winner == team2,
                    "Round {} game {} winner {} must be either {} or {}",
                    round, game_idx, winner, team1, team2);
            }
            
            prev_round_start = round_start;
            round_start += round_size;
            round_size /= 2;
        }
    }

    #[test]
    fn test_bracket_generation_sanitycheck() {
        let distribution: Normal<f64> = Normal::new(0.0, TEAMS_SCORE_STDEV).unwrap();
        let num_brackets: i32 = 1000;
        
        // Track wins for each seed (1-16), using indices 0-15
        let mut seed_wins: [usize; 16] = [0; 16];
        
        for _ in 0..num_brackets {
            let mut bracket: [u8; 63] = [0; 63];
            generate_bracket(&mut bracket, &distribution);
            
            // Count wins for each seed in this bracket
            for winner_team in bracket.iter() {
                let seed: usize = (*winner_team % 16) as usize;
                // Convert seed 0 (which is 16) to index 15, otherwise seed X to index X-1
                let seed_index: usize = if seed == 0 { 15 } else { seed - 1 };
                seed_wins[seed_index] += 1;
            }
        }
        
        // Verify that lower seed numbers (better seeds) win more often than higher seed numbers (worse seeds)
        for seed_index in 0..15 {
            assert!(seed_wins[seed_index] > seed_wins[seed_index + 1],
                "Seed {} should win more often ({} total wins) than seed {} ({} total wins)",
                seed_index + 1, seed_wins[seed_index], seed_index + 2, seed_wins[seed_index + 1]);
        }
    }
}