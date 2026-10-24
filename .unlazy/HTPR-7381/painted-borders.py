import sys
from pathlib import Path
from PIL import Image

base = Path('/home/valentin/projects/hypertask-desktop-wt-public/internal')
round_name = Path('.unlazy/HTPR-7381/current-round').read_text().strip()
mode = sys.argv[1]
samples = ([(screen, 1500, 100) for screen in ('inbox', 'board', 'task')]
           if mode == 'panel' else [('task', 270, 200), ('task', 966, 200)])
for screen, x, y in samples:
    def pixel(folder):
        return Image.open(base / folder / f'{screen}.png').convert('RGB').getpixel((x, y))
    live, native = pixel('live-graphite-1920'), pixel(round_name)
    print(f'{screen} ({x},{y}) live={live} native={native}')
    assert native == live, f'painted {mode} border differs'
print(f'BORDER_PIXELS_MATCH {mode} {round_name}')
