use std::collections::HashMap;

const HIGH: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', 'É', 'æ', 'Æ',
    'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ', 'á', 'í', 'ó', 'ú', 'ñ', 'Ñ',
    'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»', '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕',
    '╣', '║', '╗', '╝', '╜', '╛', '┐', '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦',
    '╠', '═', '╬', '╧', '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐',
    '▀', 'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩', '≡', '±',
    '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', ' ',
];

pub fn decode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| match b {
            0x00..=0x7f => b as char,
            _ => HIGH[(b - 0x80) as usize],
        })
        .collect()
}

pub fn encode(text: &str) -> (Vec<u8>, usize) {
    let reverse: HashMap<char, u8> = HIGH
        .iter()
        .enumerate()
        .map(|(index, &ch)| (ch, index as u8 + 0x80))
        .collect();
    let mut replacements = 0;
    let bytes = text
        .chars()
        .map(|ch| {
            if ch as u32 <= 0x7f {
                ch as u8
            } else if let Some(byte) = reverse.get(&ch) {
                *byte
            } else {
                replacements += 1;
                b'?'
            }
        })
        .collect();
    (bytes, replacements)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_all_bytes() {
        let source: Vec<u8> = (0..=255).collect();
        let decoded = decode(&source);
        let (encoded, replacements) = encode(&decoded);
        assert_eq!(replacements, 0);
        assert_eq!(encoded, source);
    }
}
