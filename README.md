# march_madness

A tool used to produce a large number of [march madness](https://en.wikipedia.org/wiki/NCAA_Division_I_men%27s_basketball_tournament) brackets in the hope of finally getting a [perfect bracket](https://en.wikipedia.org/wiki/March_Madness_pools).

## Scoring

This tool uses the same scoring system as Yahoo. CBS, FoxSports, and NCAA.com.  Thus a perfect bracket woudl get a score of 192 points.

|**RND 1**|**RND 2**|**RND 3**|**RND 4**|**RND 5**|**RND 6**
:--------:|:-------:|:-------:|:-------:|:-------:|:-------:
ESPN|10|20|40|80|160|320
Yahoo|1|2|4|8|16|32
CBS|1|2|4|8|16|32
FoxSports|1|2|4|8|16|32
NCAA.com|1|2|4|8|16|32

## Formats

This tool supports scoring both text (txt) and binary (bin) formats for scoring.  It now only generates the brackets using the newer binary format.

The bracket used for scoring the other brackets (winning_bracket.txt) will use the text file format (for ease of editing and reading).

### TXT (text file)

[TXT](https://en.wikipedia.org/wiki/Text_file) is the original (legacy format) that directly listed the team key as [ascii](https://www.asciitable.com/) into the file.  This consumes a significant amount of disk space when generating millions of brackets.

### BIN (binary file)

A [binary file](https://en.wikipedia.org/wiki/Binary_file) is the new format used by this tool to save disk space and speed up the reading and writing of files.  Instead of writing the team key into the file as ascii, it writes an encoded byte for each team.  The byte is encoded by taking the team number (same number as the text file) adding an offset of 32 and then writing that byte to the file.  This format also excludes the semi-colon as a round delimiter.

Looking at the below example (only the final 3 rounds are shown for simplicity)

```
text file bracket: 5 17 34 49;17 49;17
binary file bracket: %1BQ1Q1
```

## Example

Here is an example bracket (one line inside of a txt file):

```
1 9 5 13 6 3 10 2 17 25 21 20 22 19 26 18 33 40 37 36 38 35 42 34 49 57 53 61 54 51 55 50;1 5 6 10 17 20 19 18 40 37 35 34 49 61 51 50;5 6 17 19 40 34 49 50;5 17 34 49;17 49;17
```

Can be expanded to:

```
round 1 winners: 1 9 5 13 6 3 10 2 17 25 21 20 22 19 26 18 33 40 37 36 38 35 42 34 49 57 53 61 54 51 55 50
round 2 winners:  1   5    6   10    17    20    19    18   40    37     35    34   49     61    51   50
round 3 winners:    5        6          17           19        40           34         49          50
round 4 winners:        5                     17                     34                      49
round 5 winners:                   17                                            49
round 6 winners:                                           17
```

Using the 2024 bracket keys for this example `North Carolina` and `Purdue` played in the championship game and `North Carolina` won.

## Methods


Can choose between different methods:

### Legacy method (0)

probability right seed wins = left seed / (left seed + right seed)

### New method (1)

Samples a normal distribution to determine the score of each team and the team with the higher score wins the game.  Distributions are weighted based on seed.