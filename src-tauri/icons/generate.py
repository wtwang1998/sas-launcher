"""Generate the launcher icon with Pillow."""
from pathlib import Path
from PIL import Image, ImageDraw

image = Image.new("RGBA", (256, 256))
draw = ImageDraw.Draw(image)
draw.rounded_rectangle((8, 8, 248, 248), radius=56, fill="#155b53")
draw.rounded_rectangle((56, 56, 200, 200), radius=24, outline="#ecfff8", width=13)
draw.line(((84, 103), (108, 128), (84, 153)), fill="#ecfff8", width=14, joint="curve")
draw.rounded_rectangle((123, 149, 172, 161), radius=6, fill="#8de1b9")
image.save(Path(__file__).with_name("icon.ico"), sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (256, 256)])
