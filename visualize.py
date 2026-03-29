from PIL import Image, ImageDraw, ImageFont
import os
import json
import argparse

BRACKET_KEYS_FILE = 'bracket_keys_2026.json'
WINNING_BRACKET_PATH = 'winning_bracket.txt'
HEIGHT = 720
WIDTH = 1280
FONT_HEIGHT = 12
LINE_LENGTH = 80
WIN_COLOR = (0, 255, 0, 255) # rgba
LOST_COLOR = (255, 75, 0, 255) # rgba

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Visualizes a march madness bracket')
    parser.add_argument('vbracket', help='bracket to be visualized')
    parser.add_argument('filename', help='name of file to be saved')
    args = parser.parse_args()

    bracket_keys = {}
    with open(BRACKET_KEYS_FILE, 'r') as file:
        bracket_keys = json.load(file)

    winning_bracket = []
    with open(WINNING_BRACKET_PATH, 'r') as file:
        winning_bracket = file.read()

    # initialize image
    img = Image.new('RGBA', (WIDTH, HEIGHT), color=(255, 255, 255, 255))
    draw = ImageDraw.Draw(img)
    font = ImageFont.load_default()

    # draw the bracket starting round
    prev_coords = []
    next_round_coords = []
    for (i, (key, name)) in enumerate(bracket_keys.items()):

        # draw the name
        y = (HEIGHT / 32) * (i % 32) + 5
        x = 10 if i < 32 else WIDTH - 85
        name_str = ''
        if int(key) < 17:
            name_str = str(int(key) % 17) + ' ' + name
        else:
            name_str = str(((int(key) -1) % 16) + 1) + ' ' + name
        draw.text((x, y), name_str, fill=(0,0,0), font=font)

        # draw a line directly below the name
        coords = []
        if (i < 32):
            x_offset = 2
            start = (x-x_offset, y + FONT_HEIGHT)
            end = (x+LINE_LENGTH-x_offset, y + FONT_HEIGHT)
            coords = [start, end]
        else:
            x_offset = 5
            start = (x-x_offset, y + FONT_HEIGHT)
            end = (x+LINE_LENGTH-x_offset, y + FONT_HEIGHT)
            coords = [end, start]
        draw.line(coords, fill='black', width = 1)

        # draw line between the curent and previous competitors
        if (i % 2) == 1:
            draw.line([coords[1], prev_coords[1]], fill='black', width = 1)
            next_y = (coords[1][1] + prev_coords[1][1]) / 2
            next_round_coords += [(coords[1][0], next_y)]

        prev_coords = coords.copy()

    # rest of the rounds
    for round, winners in enumerate(args.vbracket.split(';')):
        true_winners = winning_bracket.split(';')[round].split()
        round_length = len(winners.split())
        if round_length == 1:
            # champion requires special logic
            break
        temp_next_round_coords = []
        prev_line_coords = []
        for win_idx, winner in enumerate(winners.split()):
            name_str = ''
            if int(winner) < 17:
                name_str = str(int(winner) % 17) + ' ' + bracket_keys[winner]
            else:
                name_str = str(((int(winner) -1) % 16) + 1) + ' ' + bracket_keys[winner]

            text_coord = (0, 0)
            line_coord = (0, 0)
            if win_idx < (round_length / 2):
                text_coord = (next_round_coords[win_idx][0]+10, next_round_coords[win_idx][1] - FONT_HEIGHT)

                # line coordinates
                start = next_round_coords[win_idx]
                end = (next_round_coords[win_idx][0] + LINE_LENGTH, next_round_coords[win_idx][1])
                line_coord = (start, end)
            else:
                text_coord = (next_round_coords[win_idx][0]+10 - LINE_LENGTH, next_round_coords[win_idx][1] - FONT_HEIGHT)

                # line coordinates
                start = next_round_coords[win_idx]
                end = (next_round_coords[win_idx][0] - LINE_LENGTH, next_round_coords[win_idx][1])
                line_coord = (start, end)

            bbox = draw.textbbox(text_coord, name_str, font=font)
            # highlight in green if correct, red if wrong, no highlight if game is not complete yet
            if true_winners[win_idx] == winner:
                draw.rectangle(bbox, fill=WIN_COLOR)
            elif true_winners[win_idx] != winner and int(true_winners[win_idx]) > 0:
                draw.rectangle(bbox, fill=LOST_COLOR)

            # draw text and line
            draw.text(text_coord, name_str, fill=(0,0,0), font=font)
            draw.line(line_coord, fill='black', width = 1)

            # draw line between the current and previous competitors
            if (win_idx % 2) == 1:
                draw.line([line_coord[1], prev_line_coords[1]], fill='black', width = 1)
                next_y = (line_coord[1][1] + prev_line_coords[1][1]) / 2
                temp_next_round_coords += [(line_coord[1][0], next_y)]
            
            prev_line_coords = line_coord
        
        next_round_coords = temp_next_round_coords.copy()

    # draw the champ
    est_champ = int(args.vbracket.split(';')[-1]) # estimated champ
    true_champ = int(winning_bracket.split(';')[-1]) # true champ
    name_str = ''
    if est_champ < 17:
        name_str = str(est_champ % 17) + ' ' + bracket_keys[str(est_champ)]
    else:
        name_str = str(((est_champ -1) % 16) + 1) + ' ' + bracket_keys[str(est_champ)]

    (_, _, width, height) = font.getmask(name_str).getbbox()
    champ_coord = (WIDTH/2 - width/2, HEIGHT/2 - height)
    bbox = draw.textbbox(champ_coord, name_str, font=font)
    if est_champ == true_champ:
        draw.rectangle(bbox, fill=WIN_COLOR)
    elif est_champ != true_champ and true_champ > 0:
        draw.rectangle(bbox, fill=LOST_COLOR)
    draw.text(champ_coord, name_str, fill=(0,0,0), font=font)

    # save the picture
    img.save(args.filename, 'PNG')