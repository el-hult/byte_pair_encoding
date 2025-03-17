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
        Self {
            decoding_table: forwards,
        }
    }

    fn decode(&self, token: Token) -> &Vec<u8> {
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
    fn get_token_for_byte(&self, b: u8) -> Option<Token> {
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
    map: HashMap<(Token, Token), usize>,
}
impl TokenPairCounter {
    fn new(v: &TokenizedString) -> Self {
        let mut it = v.iter();
        let mut fst = it.next().unwrap();
        let mut s = Self {
            map: HashMap::new(),
        };
        for snd in it {
            s.inc((*fst, *snd));
            fst = snd;
        }
        s
    }
    fn inc(&mut self, key: (Token, Token)) {
        //eprintln!("+1 on {:?}",key);
        *self.map.entry(key).or_insert(0) += 1;
    }
    // sort first by count, then by fst, then by snd
    fn get_most_common_pair(&self) -> Option<((Token, Token), usize)> {
        self.map.iter().max_by_key(|(_, b)| *b).map(|(k, v)| {
            let fst = k.0;
            let snd = k.1;
            ((fst, snd), *v)
        })
    }

    /// PRECONDITION: the pair must be present
    fn dec_unsafe(&mut self, key: &(Token, Token)) {
        //eprintln!("-1 on {:?}",key);
        *self.map.get_mut(key).unwrap() -= 1;
    }

    fn unsafe_get_pair_count(&self, key: &(Token, Token)) -> &usize {
        self.map.get(key).unwrap()
    }
}

/// Find the most common token pair, replace it in the tokenized string
/// Return the new tokenized string, and the number of times the newly created token was used
/// PRECONDITION:
///     tpc has the current pair count for the tkn_str
/// TODO instead of re-counting every time, just modify the TokenPairCounter to keep track of the total count, as I do the substitutions
fn prune_round(
    tkn_str: &TokenizedString,
    dict: &mut Dictionary,
    tpc: &mut TokenPairCounter,
) -> (TokenizedString, usize) {
    //eprintln!("prune_round");
    assert!(!tkn_str.is_empty());
    if tkn_str.len() == 1 {
        return (tkn_str.clone(), 0);
    }

    // okay, we have at least two tokens.
    // which token pair should we replace?
    // finde out which token pair is most common.
    // then add the most common one to the dictionary
    let (max_pair, count) = tpc.get_most_common_pair().expect("no pairs found");
    let new_token = dict.add_pair_rule(max_pair);
    let decoded_token = dict.decode(new_token);
    let decoded_token_as_string = std::str::from_utf8(decoded_token).unwrap_or("INVALID UTF8");
    println!("{} => {}", decoded_token_as_string, count);

    // replace all occurances of the 'max' combination with a new token!
    let mut out = vec![];
    let mut j = 0;
    let mut left_to_replace = *tpc.unsafe_get_pair_count(&max_pair);
    assert_eq!(left_to_replace, count);
    while j < tkn_str.len() {
        let this_tkn = tkn_str[j];
        //eprintln!("This token={}",this_tkn);

        // try to take next token. if there is some
        let next_tkn = tkn_str.get(j + 1);
        if next_tkn.is_none() {
            out.push(this_tkn);
            break;
        }
        let next_tkn = *next_tkn.unwrap();

        // should we do a replacement?
        let this_pair = (this_tkn, next_tkn);
        if (left_to_replace > 0) && (this_pair == max_pair) {
            // with no replacement, I would have a count (pre_tkn,this_tkn), but replacement changes it into (pre_tkn,new_token)
            if let Some(&pre_tkn) = out.last() {
                let pair_to_remove = (pre_tkn, this_tkn);
                tpc.dec_unsafe(&pair_to_remove);
                tpc.inc((pre_tkn, new_token));
            }

            // same, but at the end of the replacement
            if let Some(&post_tkn) = tkn_str.get(j + 2) {
                let pair_to_add = (new_token, post_tkn);
                tpc.dec_unsafe(&(next_tkn, post_tkn));
                tpc.inc(pair_to_add);
            }

            // push the new token, remvoe the deleted pair, and see if we are done
            out.push(new_token);
            tpc.dec_unsafe(&this_pair);

            // done?
            left_to_replace = *tpc.unsafe_get_pair_count(&max_pair);
            j += 2; // skip two tokens, since we combined tokens
            continue;
        } else {
            // no replacement
            //eprintln!("(j={}) no sub",j);
            out.push(this_tkn);
            j += 1;
            continue;
        }
    }

    (out, count)
}

fn main() {
    let s = read_to_string("input.txt").expect("file is there");
    let (mut v, mut dict) = encode(&s);

    // create new tokens until the usage count for new tokens is below 100
    let mut times_used = 99999;
    let mut hm = TokenPairCounter::new(&v);
    while times_used > 10 {
        (v, times_used) = prune_round(&v, &mut dict, &mut hm);
    }
    let s2 = decode::<true>(&v, &dict);
    println!("{}", s2);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn process_not() {
        let s = "la la".to_owned();
        let (v, dict) = encode(&s);
        let s2 = decode::<false>(&v, &dict);
        assert_eq!(s, s2)
    }
    #[test]
    fn process_once() {
        let s = "la la".to_owned();
        let (mut v, mut dict) = encode(&s);
        let mut hm = TokenPairCounter::new(&v);
        (v, _) = prune_round(&v, &mut dict, &mut hm);
        let s2 = decode::<false>(&v, &dict);
        assert_eq!(s, s2)
    }
    #[test]
    fn process_twice() {
        let s = "la la".to_owned();
        let (mut v, mut dict) = encode(&s);
        let mut hm = TokenPairCounter::new(&v);
        (v, _) = prune_round(&v, &mut dict, &mut hm);
        (v, _) = prune_round(&v, &mut dict, &mut hm);
        let s2 = decode::<false>(&v, &dict);
        assert_eq!(s, s2)
    }

    fn check_tpc_handling(s: String) {
        let (v, mut dict) = encode(&s);
        let mut hm = TokenPairCounter::new(&v);
        let (v2, _) = prune_round(&v, &mut dict, &mut hm);
        let mut hm2 = TokenPairCounter::new(&v2);
        //eprintln!("{:?}",v2);
        hm.map.retain(|_, v| *v > 0);
        hm2.map.retain(|_, v| *v > 0);
        assert_eq!(hm.map, hm2.map);
    }

    #[test]
    fn test_update_is_correct_1() {
        check_tpc_handling("abab".to_owned());
    }
    #[test]
    fn test_update_is_correct_2() {
        check_tpc_handling("ab ab".to_owned());
    }
    #[test]
    fn test_update_is_correct_3() {
        check_tpc_handling("ababc".to_owned());
    }
}
