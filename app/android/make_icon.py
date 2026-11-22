"""Genera los iconos PNG del lanzador de Mi Garaje."""

import os
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
APP = os.path.dirname(HERE)
FONT = os.path.join(APP, "Assets", "Fonts", "Barlow-Bold.ttf")

BG = (11, 13, 16, 255)
RED = (229, 57, 53, 255)
RED_DARK = (156, 34, 32, 255)
WHITE = (245, 245, 245, 255)
MUTED = (146, 153, 163, 255)

S = 1024
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

# fondo con esquinas redondeadas
d.rounded_rectangle([0, 0, S - 1, S - 1], radius=220, fill=BG)

# halo rojo tenue
d.ellipse([150, 150, S - 150, S - 150], fill=(24, 18, 20, 255))

# aro del velocímetro
cx, cy, r = S // 2, S // 2 + 20, 300
d.arc([cx - r, cy - r, cx + r, cy + r], start=150, end=390, fill=RED, width=44)

# marcas
import math

for deg in range(160, 381, 22):
    a = math.radians(deg)
    r1, r2 = r - 60, r - 30
    x1, y1 = cx + r1 * math.cos(a), cy + r1 * math.sin(a)
    x2, y2 = cx + r2 * math.cos(a), cy + r2 * math.sin(a)
    d.line([x1, y1, x2, y2], fill=MUTED, width=9)

# aguja
a = math.radians(232)
d.line([cx, cy, cx + 235 * math.cos(a), cy + 235 * math.sin(a)], fill=WHITE, width=22)
d.ellipse([cx - 34, cy - 34, cx + 34, cy + 34], fill=RED)
d.ellipse([cx - 13, cy - 13, cx + 13, cy + 13], fill=BG)

# siglas
try:
    f = ImageFont.truetype(FONT, 128)
except OSError:
    f = ImageFont.load_default()
txt = "MG"
box = d.textbbox((0, 0), txt, font=f)
tw, th = box[2] - box[0], box[3] - box[1]
d.text((cx - tw / 2 - box[0], cy + 340 - th / 2 - box[1]), txt, font=f, fill=WHITE)

# línea roja inferior
d.rounded_rectangle([cx - 90, cy + 430, cx + 90, cy + 446], radius=8, fill=RED_DARK)

# densidades de Android
SIZES = {
    "mipmap-mdpi": 48,
    "mipmap-hdpi": 72,
    "mipmap-xhdpi": 96,
    "mipmap-xxhdpi": 144,
    "mipmap-xxxhdpi": 192,
}
for folder, size in SIZES.items():
    out = os.path.join(HERE, "res", folder)
    os.makedirs(out, exist_ok=True)
    img.resize((size, size), Image.LANCZOS).save(os.path.join(out, "ic_launcher.png"))

print("iconos generados")
