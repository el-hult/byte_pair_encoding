use std::{collections::HashMap, fs::read_to_string};

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
    forwards: Vec<Vec<u8>>,
}

impl Dictionary {
    fn decode<'a>(&'a self, token: Token) -> &'a Vec<u8> {
        &self.forwards[token]
    }
    // TODO when working with bytes, we can use some bit manipulation to make this nicer
    fn add_char_token(&mut self, char: char) -> Token {
        self.forwards.push(char.to_string().as_bytes().to_vec());
        self.forwards.len() - 1
    }
    fn add_pair_rule(&mut self, (fst, snd): (&Token, &Token)) -> Token {
        // for new token maps by concatenating the bytes for the two tokens
        let new_token = {
            let mut v = self.forwards[*fst].clone();
            v.extend(&self.forwards[*snd]);
            v
        };
        self.forwards.push(new_token);
        self.forwards.len() - 1
    }
    // TODO this is a linear search, but it should be fine for now
    fn get_token_for_char(&self, c: &char) -> Option<Token> {
        self.forwards
            .iter()
            .position(|x| x == &c.to_string().as_bytes().to_vec())
    }
}

fn decode<const COLOR: bool>(v: &TokenizedString, dict: &Dictionary) -> String {
    let mut output_s = String::new();
    const RESET: &str = "\x1b[0m";
    const YELLOW: &str = "\x1b[93m";
    const RED: &str = "\x1b[91m";
    let mut curr_color = RESET;
    let mut it = v.iter();
    while let Some(token) = it.next() {
        // try to decode the token
        let mut decoded = dict.decode(*token).clone();
        let mut maybe_str = std::str::from_utf8(&decoded);
        while maybe_str.is_err() {
            // token did not decode to a valid utf8 sequence, add another decoded token another token
            let next_token = it
                .next()
                .expect("previous did not end on a utf8 boundary, so there must be more tokens");
            decoded.extend(dict.decode(*next_token));
            maybe_str = std::str::from_utf8(&decoded);
            if COLOR {
                curr_color = RED;
            }
        }
        let str_to_add = maybe_str.unwrap();
        if COLOR {
            output_s.push_str(curr_color);
            curr_color = if curr_color == RESET { YELLOW } else { RESET };
        }
        output_s.push_str(str_to_add);
    }
    if COLOR {
        output_s.push_str(RESET);
    }
    output_s
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
        return (v.clone(), 0);
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
    let foo = std::str::from_utf8(foo).expect("all tokens should be valid utf8");
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

    (out, count)
}

fn main() -> () {
    let s = read_to_string("input.txt").expect("file is there");
    let (mut v, mut table) = encode(&s);

    // create new tokens until the usage count for new tokens is below 100
    let mut times_used = 99999;
    while times_used > 100 {
        (v, times_used) = prune_round(&v, &mut table);
    }
    let s2 = decode::<true>(&v, &table);
    println!("{}", s2);
    ()
}
