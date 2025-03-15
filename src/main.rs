use std::{collections::HashMap, fs::read_to_string};


type Token = u16; // must be smaller than usize, since I upcast from token to usize
type TokenizedString = Vec<Token>;


struct Dictionary {
    /// mapping from a token to the byte sequence it represents
    decoding_table: Vec<Vec<u8>>,
}

impl Dictionary {
    fn new() -> Self {
        // prepopulate the dictionary with all the raw bytes
        let mut forwards = vec![];
        for i in 0..256 {
            forwards.push(vec![i as u8]);
        }
        Self { decoding_table: forwards }
    }

    fn decode<'a>(&'a self, token: Token) -> &'a Vec<u8> {
        &self.decoding_table[token as usize]
    }
    fn add_byte_token(&mut self, b: u8) -> Token {
        // do nothing -- the byte is already in the dictionary
        b as Token
    }
    fn add_pair_rule(&mut self, (fst, snd): (Token, Token)) -> Token {
        // make sure there is room to add a new token -- that the storage is large enough
        assert!(self.decoding_table.len() < Token::MAX as usize);

        // for new token maps by concatenating the bytes for the two tokens
        let new_token = {
            let mut v = self.decoding_table[fst as usize].clone();
            v.extend(&self.decoding_table[snd as usize]);
            v
        };
        self.decoding_table.push(new_token);
        self.decoding_table.len() as Token - 1
    }
    fn get_token_for_byte(&self, b:u8) -> Option<Token> {
        Some(b as Token) // reserve the first 256 tokens for the raw bytes
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
    let mut dict = Dictionary::new();
    let mut v = vec![];
    for b in s.bytes() {
        let idx = dict.get_token_for_byte(b);
        match idx {
            Some(i) => v.push(i),
            None => {
                let i = dict.add_byte_token(b);
                v.push(i);
            }
        }
    }
    (v, dict)
}

struct TokenPairCounter {
    map: HashMap<(Token,Token), usize>,
}
impl TokenPairCounter {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    fn add_pair(&mut self, (fst, snd): (&Token, &Token)) {
        let key = (*fst, *snd);
        *self.map.entry(key).or_insert(0) += 1;
    }
    // sort first by count, then by fst, then by snd
    fn get_most_common_pair(&self) -> Option<((Token,Token), usize)> {
        self.map.iter().max_by_key(|(_, b)| *b).map(|(k,v)| {
            let fst = k.0;
            let snd = k.1;
            ((fst, snd), *v)
        })
    }
    fn process_string(&mut self, v: &TokenizedString) {
        self.map.iter_mut().for_each(|(_, v)| *v = 0); // TODO we don't want to invalidate all the work we have done really.
        let mut it = v.iter();
        let mut fst = it.next().unwrap();
        while let Some(snd) = it.next() {
            self.add_pair((fst, snd));
            fst = snd;
        }
    }
}

/// Find the most common token pair, replace it in the tokenized string
/// Return the new tokenized string, and the number of times the newly created token was used
/// PRECONDITION:
///     tpc has the current pair count for the tkn_str
/// TODO instead of re-counting every time, just modify the TokenPairCounter to keep track of the total count, as I do the substitutions
fn prune_round(tkn_str: &TokenizedString, dict: &mut Dictionary, tpc: &mut TokenPairCounter) -> (TokenizedString, usize) {
    if tkn_str.len() <= 1 {
        return (tkn_str.clone(), 0);
    }

    // count all token pairs and add the most common one to the dictionary
    let (max_pair, count) = tpc.get_most_common_pair().expect("no pairs found");
    let new_token_number = dict.add_pair_rule(max_pair);
    let foo = dict.decode(new_token_number);
    let foo = std::str::from_utf8(foo).unwrap_or( "INVALID UTF8");
    println!("{} ({})", foo, count);

    // replace all occurances of the 'max' combination with a new token!
    let mut out = vec![];
    let mut it = tkn_str.iter();
    let mut fst = it.next().unwrap();
    let mut rest_token = true;
    while let Some(snd) = it.next() {
        let pair = (*fst, *snd);
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
    let (mut v, mut dict) = encode(&s);

    // create new tokens until the usage count for new tokens is below 100
    let mut times_used = 99999;
    let mut hm = TokenPairCounter::new();
    while times_used > 100 {
        hm.process_string(&v);
        (v, times_used) = prune_round(&v, &mut dict, &mut hm);
    }
    let s2 = decode::<true>(&v, &dict);
    println!("{}", s2);
    ()
}
