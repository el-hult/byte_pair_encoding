use std::{collections::HashMap, fs::read_to_string};
const ANSI_RESET: &str = "\x1b[0m";
const ANSI_YELLOW_TEXT: &str = "\x1b[93m";

type Token = usize;
type TokenizedString = Vec<Token>;

// TODO tokenize on the byte level, not on the unicode character level, to enable BPE encoding unseen unicode characters (but the basic dictionary is that all bytes are kept as they are anyways)
// the TokenMap should have all reductions from (Token,Token) -> Token
// so the TokenMap must have that mapping for encoding, and may have a reverse mapping for decoding that goes Token -> Vec<Token> directly, expanding all steps in one go
// in the first step, use raw bytes casted to u16 as tokens, and then do BPE on that level
// all the non-byte-tokens are from \x0100 and upwards wo we are guaranteed no ambiguity
// a token is a u16 = (u8,u8)
// and so the mapping table is HashMap<(u16,u16),u16> or maybe even HashMap<u32,u16> since we can just shift the u16s into a u32
// and the reverse mapping table is HashMap<u16,Vec<u8>>
// a hashmap is probably wasteful, but it is simple to implement and may be fast enough? since the key is really a u32 and all elements in range of u8 are invalid as keys, we could use a vector and just subtract \xFF from the key to get an index? should be super mega fast. but less readable and more code to write?
struct Dictionary {
    forwards: Vec<String>
}

impl Dictionary {
    fn decode(&self, token: Token) -> String {
        self.forwards[token].clone()
    }
    // TODO when working with bytes, we can use some bit manipulation to make this nicer
    fn add_char_token(&mut self, char: char) -> Token {
        self.forwards.push(char.to_string());
        self.forwards.len() - 1
    }
    fn add_pair_rule(&mut self, (fst,snd): (&Token,&Token)) -> Token {
        let new_token = self.decode(*fst) + &self.decode(*snd);
        self.forwards.push(new_token);
        self.forwards.len() - 1
    }
    // TODO this will be removed when I move to byte level encoding instead of characted level encoding
    // TODO use a hash map instead to speed up the lookup?
    fn get_token_for_char(&self, c: &char) -> Option<Token> {
        self.forwards.iter().position(|x| x == &c.to_string())
    }
}



// TODO after doing BPE on byte level
// if a token don't align with a unicode character boundary, we cannot set the color correctly
// therefore, do two variants in decoding
// -> decoding a token produces a valid unicode string -- set a color that alternates (white/yellow?)
// -> decoding a token produces an invalid unicode string -- decode another token and see if the combined byte sequence is valid,
//   if it is, set the color to red, otherwise add another token in decoding, and try again
fn decode<const COLOR: bool>(v: &TokenizedString, mapping_table: &Dictionary) -> String {
    let mut s = vec![];
    let mut is_red = false;
    for token in v {
        if COLOR && is_red {
            s.push(ANSI_YELLOW_TEXT.to_string());
        }
        s.push(mapping_table.decode(*token));
        if COLOR && is_red {
            s.push(ANSI_RESET.to_string());
        }
        if COLOR {
            is_red = !is_red;
        }
    }
    s.into_iter().collect::<String>()
}

fn encode(s: &str) -> (TokenizedString, Dictionary) {
    let mut dict = Dictionary { forwards: vec![] };
    let mut v = vec![];
    for c in s.chars() {
        let idx = dict.get_token_for_char(&c);
        match idx {
            Some(i) => v.push(i),
            None => {
                let i = dict.add_char_token(c);
                v.push(i);
            }
        }
    }
    (v, dict)
}

/// Find the most common token pair, replace it in the tokenized string
/// Return the new tokenized string, and the number of times the newly created token was used
fn prune_round(v: &TokenizedString, tkn_map: &mut Dictionary) -> (TokenizedString, usize) {
    if v.len() <= 1 {
        return (v.clone(),0);
    }

    // count all token pairs
    let mut hm: HashMap<(&Token, &Token), usize> = HashMap::new();
    let mut token_iterator = v.iter();
    let mut fst_token = token_iterator.next().unwrap();
    while let Some(snd_token) = token_iterator.next() {
        let token_pair = (fst_token, snd_token);
        *hm.entry(token_pair).or_insert(0) += 1;
        fst_token = snd_token;
    }

    // decide what the new token to remove is
    // TODO make sure this is deterministic, and I don't fall into some trap about iteration order when iterating a hashmap. split ties on the keys!
    let (max_pair, count) = hm
        .into_iter()
        .max_by_key(|(_, b)| *b)
        .expect("There should be at least one pair in the iteration before");
    let new_token_number = tkn_map.add_pair_rule(max_pair);
    let foo = tkn_map.decode(new_token_number);
    println!("{} ({})", foo, count);

    // replace all occurances of the 'max' combination with a new token!
    let mut out = vec![];
    let mut it = v.iter();
    let mut fst = it.next().unwrap();
    let mut rest_token = true;
    while let Some(snd) = it.next() {
        let pair = (fst, snd);
        if pair == max_pair {
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

    (out,count)
}

fn main() -> () {
    let s = read_to_string("input.txt").expect("file is there");
    let (mut v, mut table) = encode(&s);
    
    // create new tokens until the usage count for new tokens is below 100
    let mut times_used = 99999;
    while times_used > 100 {
        (v,times_used) = prune_round(&v, &mut table);
    }
    let s2 = decode::<true>(&v, &table);
    println!("{}", s2);
    println!("{:?}", table.forwards);
    ()
}
