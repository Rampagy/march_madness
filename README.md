# march_madness

A tool used to produce a large number of [march madness](https://en.wikipedia.org/wiki/NCAA_Division_I_men%27s_basketball_tournament) brackets in the hope of finally getting a [perfect bracket](https://en.wikipedia.org/wiki/March_Madness_pools).

## Scoring

This tool uses the same scoring system as Yahoo, CBS, FoxSports, and NCAA.com.  Thus a perfect bracket would get a score of 192 points.

|**COMPANY**|**RND 1**|**RND 2**|**RND 3**|**RND 4**|**RND 5**|**RND 6**
:----------:|:-------:|:-------:|:-------:|:-------:|:-------:|:-------:
ESPN|10|20|40|80|160|320
Yahoo|1|2|4|8|16|32
CBS|1|2|4|8|16|32
FoxSports|1|2|4|8|16|32
NCAA.com|1|2|4|8|16|32

## Formats

This tool only supports scoring version 2 binary (bin) files.  It also only generates the brackets using the version 2 binary (bin) format.

The bracket used for scoring the other brackets (winning_bracket.txt) will use the text file format (for ease of editing and reading).

### TXT (text file)

[TXT](https://en.wikipedia.org/wiki/Text_file) is the original (legacy format) that directly listed the team key as [ascii](https://www.asciitable.com/) into the file.  This consumes a significant amount of disk space when generating millions of brackets.

This file format was used for the 2024 competition.

### BIN (binary file version 1)

A [binary file](https://en.wikipedia.org/wiki/Binary_file) is a previous format used by this tool to save disk space and speed up the reading and writing of files.  Instead of writing the team key into the file as ascii, it writes an encoded byte for each team.  The byte is encoded by taking the team number (same number as the text file) adding an offset of 32 and then writing that byte to the file.  This format also excludes the semi-colon as a round delimiter.

Looking at the below example (only the final 3 rounds are shown for simplicity)

```
text file bracket: 5 17 34 49;17 49;17
binary file bracket: %1BQ1Q1
```

This file format was used for the 2025 competition.

### BIN (binary file version 2)

An improved [binary file](https://en.wikipedia.org/wiki/Binary_file) is the current format used by this tool to save disk space and speed up the reading and writing of files.  Instead of writing the team key as an encoded byte for each team this format only saves who won each game as a bit in a 64 bit word.  A zero bit indicates that the first team in the match up won, while a one bit indicates the second team in the match up won.  The bits are packed from left to right meaning that the 1 vs 16 seed matchup in the east division will always be represented by the most significant bit of the 64 bit word.

Looking at the below example (only the final 3 rounds are shown for simplicity)

```
text file bracket: 5 17 34 49;17 49;17
binary file (version 1) bracket: %1BQ1Q1
binary file (version 2) bracket: 0b110
```

This file format was used for the 2026 competition.

## Example

Here is an example bracket in each format:

```
*binary file (version 2) format: 0x52 0x42 0x02 0x50 0x07 0xB7 0x85 0x2C
binary file (version 1) format: !)%-&#*"195463:2AHEDFCJBQYU]VSWR!%&*1432HECBQ]SR%&13HBQR%1BQ1Q1
text file format: 1 9 5 13 6 3 10 2 17 25 21 20 22 19 26 18 33 40 37 36 38 35 42 34 49 57 53 61 54 51 55 50;1 5 6 10 17 20 19 18 40 37 35 34 49 61 51 50;5 6 17 19 40 34 49 50;5 17 34 49;17 49;17
```

Which can be expanded to:

```
round 1 winners: 1 9 5 13 6 3 10 2 17 25 21 20 22 19 26 18 33 40 37 36 38 35 42 34 49 57 53 61 54 51 55 50;
round 2 winners:  1   5    6   10    17    20    19    18   40    37     35    34   49     61    51   50;
round 3 winners:    5        6          17           19        40           34         49          50;
round 4 winners:        5                     17                     34                      49;
round 5 winners:                   17                                            49;
round 6 winners:                                           17
```

Using the 2024 bracket keys for this example `North Carolina` and `Purdue` played in the championship game and `North Carolina` won.

*The bytes are shown in hexadecimal representation and space delimited for the sake of understanding as the ascii representation of some of the characters are not visible.  In an actual file the bytes will be written directly to the file and also not space delimited so that it only takes up 8 bytes per bracket.


## Methods

Can choose between different methods:

### Legacy method (0)

probability right seed wins = left seed / (left seed + right seed)

### New method (1)

Samples a normal distribution to determine the score of each team and the team with the higher score wins the game.  Distributions are weighted based on seed.
