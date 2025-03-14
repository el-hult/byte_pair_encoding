use std::fs::read_to_string;
const ANSI_RESET: &str = "\x1b[0m";
const ANSI_YELLOW_TEXT: &str = "\x1b[93m";
//const ANSI_YELLOW_BACKGROUND: &str = "\x1b[43m";
type TokenizedString = Vec<usize>;

fn decode<const COLOR: bool>(v: TokenizedString, mapping_table: Vec<char>) -> String {
    let mut s = vec![];
    let mut is_red = false;
    for token in v {
        if COLOR && is_red {
            ANSI_YELLOW_TEXT.chars().for_each(|c| s.push(c));
        }
        s.push(mapping_table[token]);
        if COLOR && is_red {
            ANSI_RESET.chars().for_each(|c| s.push(c));
        }
        if COLOR {
            is_red = !is_red;
        }
    }
    s.into_iter().collect::<String>()
}

fn encode(s: &str) -> (TokenizedString, Vec<char>) {
    let mut mapping_table: Vec<char> = vec![];
    let mut v = vec![];
    for c in s.chars() {
        let idx = mapping_table.iter().position(|&a| a == c);
        match idx {
            Some(i) => v.push(i),
            None => {
                let i = mapping_table.len();
                v.push(i);
                mapping_table.push(c);
            }
        }
    }
    (v, mapping_table)
}

fn main() -> () {
    let s = read_to_string("input.txt").expect("file is there");
    let (v, mapping_table) = encode(&s);
    let s2 = decode::<true>(v, mapping_table);
    println!("{}", s2);
    ()
}
