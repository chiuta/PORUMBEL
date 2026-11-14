// Prints the SVG of a QR code for the text given as the first argument
// (optional second argument: force mask 0-7). Used by tools/check-qr.py.
fn main() {
    let mut a = std::env::args().skip(1);
    let t = a
        .next()
        .unwrap_or_else(|| "http://192.168.1.10:8750/?t=abc".into());
    let m = a.next().and_then(|m| m.parse().ok());
    print!(
        "{}",
        porumbel::qr::Qr::encode_with_mask(t.as_bytes(), m)
            .expect("encode")
            .to_svg(4)
    );
}
