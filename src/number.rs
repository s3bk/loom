const superscipt_numbers = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
const subscript_number_offset: char = '₀';

fn subscript_number(n: u8) -> char {
    (subscript_number_offset as u32 + n as u32) as char
}
fn superscript_number(n: u8) -> char {
    superscipt_numbers[n as usize]
}
fn fraction(nom: usize, denom: usize) {
    ⁄
}
