use std::{collections::HashMap, fs::read_to_string};
const ANSI_RESET: &str = "\x1b[0m";
const ANSI_YELLOW_TEXT: &str = "\x1b[93m";


type TokenizedString = Vec<usize>;

// TODO tokenize on the byte level, not on the unicode character level, to enable BPE encoding unseen unicode characters (but the basic dictionary is that all bytes are kept as they are anyways)
// the TokenMap should have all reductions from (Token,Token) -> Token
// so the TokenMap must have that mapping for encoding, and may have a reverse mapping for decoding that goes Token -> Vec<Token> directly, expanding all steps in one go
// in the first step, use raw bytes casted to u16 as tokens, and then do BPE on that level
// all the non-byte-tokens are from \x0100 and upwards wo we are guaranteed no ambiguity
// a token is a u16 = (u8,u8)
// and so the mapping table is HashMap<(u16,u16),u16>
// and the reverse mapping table is HashMap<u16,Vec<u8>>
// a hashmap is probably wasteful, but it is simple to implement and may be fast enough? since the key is really a u32 and all elements in range of u8 are invalid as keys, we could use a vector and just subtract \xFF from the key to get an index? should be super mega fast. but less readable and more code to write?
type TokenMap = Vec<String>;



// TODO after doing BPE on byte level
// if a token don't align with a unicode character boundary, we cannot set the color correctly
// therefore, do two variants in decoding
// -> decoding a token produces a valid unicode string -- set a color that alternates (white/yellow?)
// -> decoding a token produces an invalid unicode string -- decode another token and see if the combined byte sequence is valid,
//   if it is, set the color to red, otherwise add another token in decoding, and try again
fn decode<const COLOR: bool>(v: &TokenizedString, mapping_table: &TokenMap) -> String {
    let mut s = vec![];
    let mut is_red = false;
    for token in v {
        if COLOR && is_red {
            s.push(ANSI_YELLOW_TEXT.to_string());
        }
        s.push(mapping_table[*token].clone());
        if COLOR && is_red {
            s.push(ANSI_RESET.to_string());
        }
        if COLOR {
            is_red = !is_red;
        }
    }
    s.into_iter().collect::<String>()
}

fn encode(s: &str) -> (TokenizedString, TokenMap) {
    let mut mapping_table: Vec<String> = vec![];
    let mut v = vec![];
    for c in s.chars() {
        // TODO use a map. this became slow after the number of tokens grew large enough for the vector to not fit in cache properly
        let idx = mapping_table
            .iter()
            .position(|a| a.chars().next().unwrap() == c);
        match idx {
            Some(i) => v.push(i),
            None => {
                println!("{} => {}", c, mapping_table.len());
                let i = mapping_table.len();
                v.push(i);
                mapping_table.push(c.to_string());
            }
        }
    }
    (v, mapping_table)
}

/// Find the most common token pair, replace it in the tokenized string
fn prune_round(v: &TokenizedString, tkn_map: &mut TokenMap) -> TokenizedString {
    //println!("PRUNE");
    if v.len() <= 1 {
        return v.clone();
    }

    let mut hm = HashMap::new();
    let mut it = v.iter();
    let mut fst = it.next().unwrap();
    while let Some(snd) = it.next() {
        let pair = (fst, snd);
        *hm.entry(pair).or_insert(0) += 1;
        fst = snd;
    }

    // decide what the new token to remove is
    let (max, count) = hm
        .into_iter()
        .max_by_key(|(_, b)| *b)
        .expect("There should be at least one pair in the iteration before");
    let new_token_number = tkn_map.len();
    //println!("{}", new_token_number);
    let new_token = tkn_map[*max.0].clone() + &tkn_map[*max.1];
    println!("{} => {}", new_token, count);
    tkn_map.push(new_token);

    // replace all occurances of the 'max' combination with a new token!
    let mut out = vec![];
    let mut it = v.iter();
    let mut fst = it.next().unwrap();
    let mut rest_token = true;
    while let Some(snd) = it.next() {
        let pair = (fst, snd);
        if pair == max {
            out.push(new_token_number);
            let maybe_fst = it.next();
            match maybe_fst {
                None => {
                    rest_token = false;
                    break;
                }
                Some(q) => fst = q,
            }
        } else {
            out.push(*fst);
            fst = snd;
        }
    }
    if rest_token {
        out.push(*fst);
    }

    out
}

fn main() -> () {
    let s = read_to_string("input.txt").expect("file is there");
    let (mut v, mut table) = encode(&s);
    
    // TODO tokenize until the least common token is used less than 10 times. this means we need to return an optional or a Result or something...
    for _ in 0..200 {
        v = prune_round(&v, &mut table);
    }
    let s2 = decode::<true>(&v, &table);
    println!("{}", s2);
    println!("{table:?}");
    ()
}
