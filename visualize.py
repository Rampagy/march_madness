from PIL import Image, ImageDraw, ImageFont
import os
import json

VISUALIZE_BRACKET = '1 9 5 4 6 3 7 2 17 25 21 20 27 19 26 18 33 41 44 36 43 35 39 34 49 57 53 52 54 51 55 50;1 5 3 7 25 20 19 18 33 36 43 34 49 52 51 55;1 7 25 18 36 34 49 55;1 18 34 49;18 34;34'
BRACKET_KEYS_FILE = 'bracket_keys_2026.json'
SAVE_PATH = 'top_bracket.png'
WINNING_BRACKET_PATH = 'winning_bracket.txt'
HEIGHT = 720
WIDTH = 1280
FONT_HEIGHT = 12
LINE_LENGTH = 80

if __name__ == '__main__':
    bracket_keys = {}
    with open(BRACKET_KEYS_FILE, 'r') as file:
        bracket_keys = json.load(file)

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
    for round, winners in enumerate(VISUALIZE_BRACKET.split(';')):
        round_length = len(winners.split())
        for win_idx, winner in enumerate(winners.split()):
            name_str = ''
            if int(winner) < 17:
                name_str = str(int(winner) % 17) + ' ' + bracket_keys[winner]
            else:
                name_str = str(((int(winner) -1) % 16) + 1) + ' ' + bracket_keys[winner]

            text_coord = (0, 0)
            line_coord = (0, 0)
            if win_idx < (round_length / 2):
                text_coord = (next_round_coords[win_idx][0]+5, next_round_coords[win_idx][1] - FONT_HEIGHT)

                # line coordinates
                start = next_round_coords[win_idx]
                end = (next_round_coords[win_idx][0] + LINE_LENGTH, next_round_coords[win_idx][1])
                line_coord = (start, end)
            else:
                text_coord = (next_round_coords[win_idx][0]+5 - LINE_LENGTH, next_round_coords[win_idx][1] - FONT_HEIGHT)

                # line coordinates
                start = next_round_coords[win_idx]
                end = (next_round_coords[win_idx][0] - LINE_LENGTH, next_round_coords[win_idx][1])
                line_coord = (start, end)
            
            # drwaw text and line
            draw.text(text_coord, name_str, fill=(0,0,0), font=font)
            draw.line((start, end), fill='black', width = 1)
        break


    # save the picture
    img.save(SAVE_PATH, 'PNG')

'''
# Define image size and background color (white)
width, height = 200, 100
# Use 'RGBA' mode for potential transparency
img = Image.new('RGBA', (width, height), color=(255, 255, 255, 255))

# Get a drawing context
draw = ImageDraw.Draw(img)

# Define text and color (black)
text = "Hello, World!"
text_color = (0, 0, 0)

# (Optional) Specify font and size if you have a font file. 
# You might need to provide the full path to a font file on your system.
try:
    # Example font path (adjust for your OS)
    font = ImageFont.truetype("arial.ttf", 20) 
except IOError:
    # Fallback to default font if the specified font is not found
    font = ImageFont.load_default()
    print("Could not load arial.ttf, using default font.")

# Calculate text position (simple centering approximation)
text_width = draw.textlength(text, font=font)
text_height = font.getbbox()[3] if hasattr(font, 'getbbox') else font.getsize(text)[1] # Pillow version compatibility check
x = (width - text_width) / 2
y = (height - text_height) / 2

# Draw the text
draw.text((x, y), text, fill=text_color, font=font)

# Save the image as a PNG file
file_path = 'my_image.png'
img.save(file_path, 'PNG')

print(f"Image saved to {os.path.abspath(file_path)}")
'''