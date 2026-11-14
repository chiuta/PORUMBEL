//! Minimal QR Code encoder (byte mode, error correction M, versions 1–6: up to 106 bytes), std only.
//! Used to show the phone address as a code on the panel and in the terminal.

const ECC_PER_BLOCK_M: [usize; 10] = [0, 10, 16, 26, 18, 24, 16, 18, 22, 22];
const BLOCKS_M: [usize; 10] = [0, 1, 1, 1, 2, 2, 4, 4, 4, 5];
const ECC_FORMAT_M: u32 = 0;

pub struct Qr {
    pub size: usize,
    modules: Vec<bool>,
}

fn raw_data_modules(ver: usize) -> usize {
    let mut r = (16 * ver + 128) * ver + 64;
    if ver >= 2 {
        let n = ver / 7 + 2;
        r -= (25 * n - 10) * n - 55;
    }
    r
}

fn data_codewords(ver: usize) -> usize {
    raw_data_modules(ver) / 8 - ECC_PER_BLOCK_M[ver] * BLOCKS_M[ver]
}

fn gf_mul(x: u8, y: u8) -> u8 {
    let mut z: u32 = 0;
    for i in (0..8).rev() {
        z = (z << 1) ^ ((z >> 7) * 0x11D);
        z ^= ((y as u32 >> i) & 1) * x as u32;
    }
    z as u8
}

fn rs_divisor(degree: usize) -> Vec<u8> {
    let mut result = vec![0u8; degree];
    result[degree - 1] = 1;
    let mut root = 1u8;
    for _ in 0..degree {
        for j in 0..degree {
            result[j] = gf_mul(result[j], root);
            if j + 1 < degree {
                result[j] ^= result[j + 1];
            }
        }
        root = gf_mul(root, 2);
    }
    result
}

fn rs_remainder(data: &[u8], divisor: &[u8]) -> Vec<u8> {
    let mut result = vec![0u8; divisor.len()];
    for &b in data {
        let factor = b ^ result.remove(0);
        result.push(0);
        for (x, &y) in result.iter_mut().zip(divisor) {
            *x ^= gf_mul(y, factor);
        }
    }
    result
}

struct Builder {
    size: usize,
    m: Vec<bool>,
    f: Vec<bool>, // function modules (not masked, not data)
}

impl Builder {
    fn set(&mut self, x: i32, y: i32, dark: bool) {
        let s = self.size as i32;
        if x >= 0 && y >= 0 && x < s && y < s {
            let i = y as usize * self.size + x as usize;
            self.m[i] = dark;
            self.f[i] = true;
        }
    }

    fn draw_function_patterns(&mut self, ver: usize) {
        let s = self.size as i32;
        for i in 0..s {
            self.set(6, i, i % 2 == 0);
            self.set(i, 6, i % 2 == 0);
        }
        for &(cx, cy) in &[(3, 3), (s - 4, 3), (3, s - 4)] {
            for dy in -4i32..=4 {
                for dx in -4i32..=4 {
                    let d = dx.abs().max(dy.abs());
                    self.set(cx + dx, cy + dy, d != 2 && d != 4);
                }
            }
        }
        if ver >= 2 {
            let n = ver / 7 + 2;
            let step = if ver == 32 {
                26
            } else {
                (ver * 8 + n * 3 + 5) / (n * 4 - 4) * 2
            };
            let mut pos = vec![6i32];
            let mut p = s - 7;
            for _ in 0..n - 1 {
                pos.insert(1, p);
                p -= step as i32;
            }
            let last = pos.len() - 1;
            for i in 0..pos.len() {
                for j in 0..pos.len() {
                    if (i == 0 && j == 0) || (i == 0 && j == last) || (i == last && j == 0) {
                        continue;
                    }
                    for dy in -2i32..=2 {
                        for dx in -2i32..=2 {
                            self.set(pos[i] + dx, pos[j] + dy, dx.abs().max(dy.abs()) != 1);
                        }
                    }
                }
            }
        }
        self.draw_format(0);
    }

    fn draw_format(&mut self, mask: u32) {
        let data = (ECC_FORMAT_M << 3) | mask;
        let mut rem = data;
        for _ in 0..10 {
            rem = (rem << 1) ^ ((rem >> 9) * 0x537);
        }
        let bits = ((data << 10) | rem) ^ 0x5412;
        let bit = |i: u32| (bits >> i) & 1 != 0;
        let s = self.size as i32;
        for i in 0..6 {
            self.set(8, i, bit(i as u32));
        }
        self.set(8, 7, bit(6));
        self.set(8, 8, bit(7));
        self.set(7, 8, bit(8));
        for i in 9..15 {
            self.set(14 - i, 8, bit(i as u32));
        }
        for i in 0..8 {
            self.set(s - 1 - i, 8, bit(i as u32));
        }
        for i in 8..15 {
            self.set(8, s - 15 + i, bit(i as u32));
        }
        self.set(8, s - 8, true);
    }

    fn draw_codewords(&mut self, data: &[u8]) {
        let s = self.size as i32;
        let mut i = 0usize;
        let mut right = s - 1;
        while right >= 1 {
            if right == 6 {
                right = 5;
            }
            for vert in 0..s {
                for j in 0..2 {
                    let x = right - j;
                    let upward = ((right + 1) & 2) == 0;
                    let y = if upward { s - 1 - vert } else { vert };
                    let idx = y as usize * self.size + x as usize;
                    if !self.f[idx] && i < data.len() * 8 {
                        self.m[idx] = (data[i >> 3] >> (7 - (i & 7))) & 1 != 0;
                        i += 1;
                    }
                }
            }
            right -= 2;
        }
    }

    fn apply_mask(&mut self, mask: u32) {
        for y in 0..self.size {
            for x in 0..self.size {
                let (xu, yu) = (x as u32, y as u32);
                let inv = match mask {
                    0 => (xu + yu) % 2 == 0,
                    1 => yu % 2 == 0,
                    2 => xu % 3 == 0,
                    3 => (xu + yu) % 3 == 0,
                    4 => (xu / 3 + yu / 2) % 2 == 0,
                    5 => xu * yu % 2 + xu * yu % 3 == 0,
                    6 => (xu * yu % 2 + xu * yu % 3) % 2 == 0,
                    _ => ((xu + yu) % 2 + xu * yu % 3) % 2 == 0,
                };
                let i = y * self.size + x;
                if inv && !self.f[i] {
                    self.m[i] = !self.m[i];
                }
            }
        }
    }

    fn penalty(&self) -> i64 {
        let n = self.size;
        let g = |x: usize, y: usize| self.m[y * n + x];
        let mut p = 0i64;
        // runs in rows and columns (N1) and finder-like patterns (N3)
        for horiz in [true, false] {
            for a in 0..n {
                let mut run = 1;
                let line: Vec<bool> = (0..n)
                    .map(|b| if horiz { g(b, a) } else { g(a, b) })
                    .collect();
                for b in 1..n {
                    if line[b] == line[b - 1] {
                        run += 1;
                        if run == 5 {
                            p += 3;
                        } else if run > 5 {
                            p += 1;
                        }
                    } else {
                        run = 1;
                    }
                }
                // finder-like 1:1:3:1:1 with 4 light modules on a side
                let pat = [true, false, true, true, true, false, true];
                for b in 0..n.saturating_sub(6) {
                    if (0..7).all(|k| line[b + k] == pat[k]) {
                        let before = b >= 4 && (b - 4..b).all(|k| !line[k]);
                        let after = b + 11 <= n && (b + 7..b + 11).all(|k| !line[k]);
                        if before || after {
                            p += 40;
                        }
                    }
                }
            }
        }
        // 2x2 blocks (N2)
        for y in 0..n - 1 {
            for x in 0..n - 1 {
                let c = g(x, y);
                if c == g(x + 1, y) && c == g(x, y + 1) && c == g(x + 1, y + 1) {
                    p += 3;
                }
            }
        }
        // balance (N4)
        let dark = self.m.iter().filter(|b| **b).count() as i64;
        let total = (n * n) as i64;
        let k = ((dark * 20 - total * 10).abs() + total - 1) / total - 1;
        p + k.max(0) * 10
    }
}

impl Qr {
    pub fn encode(data: &[u8]) -> Result<Qr, String> {
        Self::encode_with_mask(data, None)
    }

    /// `mask` forces one of the eight mask patterns (testing); `None` picks the best.
    pub fn encode_with_mask(data: &[u8], force: Option<u32>) -> Result<Qr, String> {
        let ver = (1..=6)
            .find(|&v| {
                let need = 4 + if v < 10 { 8 } else { 16 } + data.len() * 8;
                need <= data_codewords(v) * 8
            })
            .ok_or_else(|| "address too long for the built-in QR encoder".to_string())?;

        // bit stream: mode 0100, length, bytes, terminator, padding
        let mut bits: Vec<bool> = Vec::new();
        fn push(bits: &mut Vec<bool>, val: u32, n: usize) {
            for i in (0..n).rev() {
                bits.push((val >> i) & 1 != 0);
            }
        }
        push(&mut bits, 0b0100, 4);
        push(&mut bits, data.len() as u32, 8);
        for &b in data {
            push(&mut bits, b as u32, 8);
        }
        let cap = data_codewords(ver) * 8;
        let term = (cap - bits.len()).min(4);
        push(&mut bits, 0, term);
        while bits.len() % 8 != 0 {
            bits.push(false);
        }
        let mut cw: Vec<u8> = bits
            .chunks(8)
            .map(|c| c.iter().fold(0u8, |a, &b| (a << 1) | b as u8))
            .collect();
        let mut pad = 0xECu8;
        while cw.len() < data_codewords(ver) {
            cw.push(pad);
            pad = if pad == 0xEC { 0x11 } else { 0xEC };
        }

        // split into blocks, add Reed–Solomon, interleave
        let nb = BLOCKS_M[ver];
        let ecc_len = ECC_PER_BLOCK_M[ver];
        let raw = raw_data_modules(ver) / 8;
        let n_short = nb - raw % nb;
        let short_len = raw / nb;
        let div = rs_divisor(ecc_len);
        let mut blocks: Vec<Vec<u8>> = Vec::new();
        let mut k = 0;
        for i in 0..nb {
            let dl = short_len - ecc_len + if i < n_short { 0 } else { 1 };
            let mut dat = cw[k..k + dl].to_vec();
            k += dl;
            let ecc = rs_remainder(&dat, &div);
            if i < n_short {
                dat.push(0);
            }
            dat.extend(ecc);
            blocks.push(dat);
        }
        let mut all: Vec<u8> = Vec::new();
        for i in 0..=short_len {
            for (j, b) in blocks.iter().enumerate() {
                if i != short_len - ecc_len || j >= n_short {
                    all.push(b[i]);
                }
            }
        }

        let size = ver * 4 + 17;
        let mut b = Builder {
            size,
            m: vec![false; size * size],
            f: vec![false; size * size],
        };
        b.draw_function_patterns(ver);
        b.draw_codewords(&all);
        let mut best = (i64::MAX, force.unwrap_or(0));
        for mask in (0..8).filter(|m| force.map_or(true, |f| f == *m)) {
            b.apply_mask(mask);
            b.draw_format(mask);
            let p = b.penalty();
            if p < best.0 {
                best = (p, mask);
            }
            b.apply_mask(mask); // undo
        }
        b.apply_mask(best.1);
        b.draw_format(best.1);
        Ok(Qr { size, modules: b.m })
    }

    pub fn get(&self, x: i32, y: i32) -> bool {
        x >= 0
            && y >= 0
            && (x as usize) < self.size
            && (y as usize) < self.size
            && self.modules[y as usize * self.size + x as usize]
    }

    pub fn to_svg(&self, border: i32) -> String {
        let dim = self.size as i32 + border * 2;
        let mut s = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {dim} {dim}\" shape-rendering=\"crispEdges\"><rect width=\"100%\" height=\"100%\" fill=\"#fff\"/><path fill=\"#000\" d=\""
        );
        for y in 0..self.size as i32 {
            for x in 0..self.size as i32 {
                if self.get(x, y) {
                    s.push_str(&format!("M{},{}h1v1h-1z", x + border, y + border));
                }
            }
        }
        s.push_str("\"/></svg>\n");
        s
    }

    /// Two rows of modules per text line, for the terminal.
    pub fn to_terminal(&self) -> String {
        let b = 2i32;
        let mut s = String::new();
        let mut y = -b;
        while y < self.size as i32 + b {
            for x in -b..self.size as i32 + b {
                s.push(match (self.get(x, y), self.get(x, y + 1)) {
                    (true, true) => '█',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    _ => ' ',
                });
            }
            s.push('\n');
            y += 2;
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_patterns() {
        let q = Qr::encode(b"hello").unwrap();
        assert_eq!(q.size, 21); // version 1
        for (x, y) in [(0, 0), (6, 0), (0, 6), (3, 3), (20, 0), (0, 20)] {
            assert!(q.get(x, y), "finder module {x},{y}");
        }
        assert!(!q.get(1, 1) || !q.get(1, 1));
        let url = b"http://192.168.100.200:8750/?t=0123456789abcdef0123456789abcdef";
        let q = Qr::encode(url).unwrap();
        assert!(q.size >= 33 && q.size <= 53);
        assert!(Qr::encode(&[b'a'; 107]).is_err());
        assert!(Qr::encode(&[b'a'; 106]).is_ok());
    }

    #[test]
    fn gf_and_rs() {
        assert_eq!(gf_mul(2, 0x80), 0x1D);
        // RS check: appending the remainder makes the polynomial divisible
        let d = rs_divisor(10);
        let data = b"PORUMBEL";
        let r = rs_remainder(data, &d);
        let mut all = data.to_vec();
        all.extend(&r);
        assert!(rs_remainder(&all, &d).iter().all(|b| *b == 0));
    }
}
