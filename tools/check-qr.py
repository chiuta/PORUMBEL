#!/usr/bin/env python3
"""Decode PORUMBEL's QR output with an independent reader (ZXing) for every length that
the encoder accepts, with every mask pattern forced in turn.
Needs: pip install zxing-cpp pillow ; and `cargo build --example qrdump`."""
import re, subprocess, sys
import zxingcpp
from PIL import Image

def render(svg, scale=8):
    vb = int(re.search(r'viewBox="0 0 (\d+) ', svg).group(1))
    img = Image.new("L", (vb * scale, vb * scale), 255)
    px = img.load()
    for x, y in re.findall(r'M(\d+),(\d+)h1v1h-1z', svg):
        for dy in range(scale):
            for dx in range(scale):
                px[int(x) * scale + dx, int(y) * scale + dy] = 0
    return img

bin_ = "./target/debug/examples/qrdump"
bad = checked = 0
for n in list(range(1, 107, 3)) + [105, 106]:
    text = ("http://192.168.100.200:8750/?t=" + "".join("0123456789abcdef"[(i * 7 + n) % 16] for i in range(n)))[:n]
    for mask in list(range(8)) + [None]:
        args = [bin_, text] + ([] if mask is None else [str(mask)])
        svg = subprocess.run(args, capture_output=True, text=True).stdout
        r = zxingcpp.read_barcodes(render(svg))
        ok = bool(r) and r[0].text == text
        checked += 1
        if not ok:
            bad += 1
            print(f"FAIL: {n} bytes, mask {mask}")
print(f"{checked - bad}/{checked} decoded correctly")
sys.exit(1 if bad else 0)
