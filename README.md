# march_madness

Each line inside of the file represents a march madness bracket.  Everything before a semicolon represents 1 round.  Each team is given a key and this is how they are referenced in each bracket.

## Example

Here is an example bracket (one line inside the files):

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