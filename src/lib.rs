use std::collections::HashMap;

type Token = u32; // must be smaller than usize, since I upcast from Token to usize
type TokenizedString = Vec<Token>;

#[derive(Debug, PartialEq, Eq)]
pub struct BytePairEncodingTokenizer {
    /// mapping from a token to the byte sequence it represents
    decoding_table: Vec<Vec<u8>>,
    /// ordered collection of token pairs and their corresponding tokens
    encoding_table: Vec<((Token, Token), Token)>,
}

impl Default for BytePairEncodingTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl BytePairEncodingTokenizer {
    pub fn new() -> Self {
        Self {
            decoding_table: (0..=255).map(|b| vec![b as u8]).collect(),
            encoding_table: Vec::new(),
        }
    }

    pub fn vocab_size(&self) -> usize {
        self.decoding_table.len()
    }

    fn decode_token(&self, token: Token) -> &Vec<u8> {
        &self.decoding_table[token as usize]
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
        let new_token_num = self.decoding_table.len() as Token - 1;
        self.encoding_table.push(((fst, snd), new_token_num));
        new_token_num
    }

    /// Encode string into tokens
    /// First cast to bytes, then iteratively use the encoding table to 
    /// replace token pairs with new tokens
    pub fn encode(&self, v: &str) -> Vec<Token> {
        let mut v1 = v.bytes().map(|b| b.into()).collect::<Vec<Token>>();
        let mut v2 = Vec::with_capacity(v1.len());

        for ((fst, snd), new_tkn) in &self.encoding_table {
            let mut i = 0;
            while i < v1.len() {
                if i + 1 < v1.len() && v1[i] == *fst && v1[i + 1] == *snd {
                    v2.push(*new_tkn);
                    i += 2;
                } else {
                    v2.push(v1[i]);
                    i += 1;
                }
            }

            std::mem::swap(&mut v1, &mut v2);
            v2.clear();
        }
        v1
    }

    pub fn decode<const COLOR: bool>(&self, v: &TokenizedString) -> String {
        let mut output_s = String::new();
        const RESET: &str = "\x1b[0m";
        const YELLOW: &str = "\x1b[93m";
        const RED: &str = "\x1b[91m";
        let mut curr_color = RESET;
        let mut it = v.iter();
        while let Some(token) = it.next() {
            // try to decode the token
            let mut decoded = self.decode_token(*token).clone();
            let mut maybe_str = std::str::from_utf8(&decoded);
            while maybe_str.is_err() {
                // token did not decode to a valid utf8 sequence, add another decoded token another token
                let next_token = it.next().expect(
                    "previous did not end on a utf8 boundary, so there must be more tokens",
                );
                decoded.extend(self.decode_token(*next_token));
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

    /// Serialize the tokenizer into bytes that can be stored on disk
    /// The format is:
    ///  - number of tokens (u32)
    ///    for each token:
    ///    - length of the token (u32)
    ///    - the token itself (bytes)
    ///  - number of pairs (u32)
    ///    for each pair:
    ///    - the first token (u32)
    ///    - the second token (u32)
    ///    - the new token (u32)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(&(self.decoding_table.len() as u32).to_le_bytes());
        for token in &self.decoding_table {
            out.extend(&(token.len() as u32).to_le_bytes());
            out.extend(token);
        }
        out.extend(&(self.encoding_table.len() as u32).to_le_bytes());
        for ((fst, snd), new_tkn) in &self.encoding_table {
            out.extend(&fst.to_le_bytes());
            out.extend(&snd.to_le_bytes());
            out.extend(&new_tkn.to_le_bytes());
        }
        out
    }

    /// Deserialize the tokenizer from bytes that were stored on disk
    pub fn from_bytes(bs: &[u8]) -> Self {
        let mut it = bs.iter();

        let num_tokens = take_u32_le(&mut it) as usize;
        let mut decoding_table = Vec::with_capacity(num_tokens);
        for _ in 0..num_tokens {
            let token_len = take_u32_le(&mut it) as usize;
            let token = it.by_ref().take(token_len).cloned().collect();
            decoding_table.push(token);
        }

        let num_pairs = take_u32_le(&mut it) as usize;
        let mut encoding_table = Vec::with_capacity(num_pairs);
        for _ in 0..num_pairs {
            let fst = take_u32_le(&mut it);
            let snd = take_u32_le(&mut it);
            let new_tkn = take_u32_le(&mut it);
            encoding_table.push(((fst, snd), new_tkn));
        }

        Self {
            decoding_table,
            encoding_table,
        }
    }

    /// Create a tokenizer from a corpus, training it until the usage count for new tokens is below `min_usage_count`.
    pub fn from_corpus(corpus: &str, min_usage_count: usize) -> (Self, TokenizedString) {
        let mut v = corpus.bytes().map(|b| b.into()).collect::<Vec<Token>>();
        let mut tokenizer = BytePairEncodingTokenizer::new();
        let mut times_used = usize::MAX;
        let mut tpc = TokenPairCounter::new(&v);

        while times_used > min_usage_count {
            (v, times_used) = train_step::<false>(&v, &mut tokenizer, &mut tpc);
        }

        (tokenizer, v)
    }
}

struct TokenPairCounter {
    map: HashMap<(Token, Token), usize>,
}
impl TokenPairCounter {
    fn new(v: &TokenizedString) -> Self {
        assert!(v.len() > 1);
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
    // sort first by count, then split ties by the first token, then by the second token
    fn get_most_common_pair(&self) -> Option<((Token, Token), usize)> {
        self.map.iter().max_by_key(
            |((fst, snd), count)| {
                (*count, *fst, *snd)
            },
        )
            .map(|(k, v)| {
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

    /// PRECONDITION: the pair must be present
    fn get_pair_count_unsafe(&self, key: &(Token, Token)) -> &usize {
        self.map.get(key).unwrap()
    }
}

/// Find the most common token pair, replace it in the tokenized string
/// Return the new tokenized string, and the number of times the newly created token was used
/// PRECONDITION:
///     tpc has the current pair count for the tkn_str
fn train_step<const DEBUG: bool>(
    tkn_str: &TokenizedString,
    tokenizer: &mut BytePairEncodingTokenizer,
    tpc: &mut TokenPairCounter,
) -> (TokenizedString, usize) {
    //eprintln!("prune_round");
    assert!(!tkn_str.is_empty());
    if tkn_str.len() == 1 {
        return (tkn_str.clone(), 0);
    }

    // okay, we have at least two tokens.
    // which token pair should we replace?
    // find out which token pair is most common.
    // then add the most common one to the tokenizer
    let (max_pair, count) = tpc.get_most_common_pair().expect("no pairs found");
    let new_token = tokenizer.add_pair_rule(max_pair);
    if DEBUG {
        let decoded_token = tokenizer.decode_token(new_token);
        let decoded_token_as_string = std::str::from_utf8(decoded_token).unwrap_or("INVALID UTF8");
        println!("{} => {}", decoded_token_as_string, count);
    }
    // replace all occurances of the 'max' combination with a new token!
    let mut out = Vec::with_capacity(tkn_str.len());
    let mut j = 0;
    let mut left_to_replace = *tpc.get_pair_count_unsafe(&max_pair);
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
            left_to_replace -= 1;
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

/// Helper to read the next 4 bytes from an iterator and return a u32
fn take_u32_le(it: &mut std::slice::Iter<u8>) -> u32 {
    let mut bs = [0u8; 4];
    bs.copy_from_slice(&it.by_ref().take(4).cloned().collect::<Vec<u8>>());
    u32::from_le_bytes(bs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init(s: &str) -> (TokenizedString, BytePairEncodingTokenizer, TokenPairCounter) {
        let v = s.bytes().map(|b| b.into()).collect::<Vec<Token>>();
        let tokenizer = BytePairEncodingTokenizer::new();
        let tpc = TokenPairCounter::new(&v);
        (v, tokenizer, tpc)
    }

    #[test]
    fn process_not() {
        let s = "la la".to_owned();
        let (v, tokenizer, _) = init(&s);
        let s2 = tokenizer.decode::<false>(&v);
        assert_eq!(s, s2)
    }
    #[test]
    fn process_once() {
        let s = "la la".to_owned();
        let (mut v, mut tokenizer, mut tpc) = init(&s);
        (v, _) = train_step::<false>(&v, &mut tokenizer, &mut tpc);
        let s2 = tokenizer.decode::<false>(&v);
        assert_eq!(s, s2)
    }
    #[test]
    fn process_twice() {
        let s = "la la".to_owned();
        let (mut v, mut tokenizer, mut tpc) = init(&s);
        (v, _) = train_step::<false>(&v, &mut tokenizer, &mut tpc);
        (v, _) = train_step::<false>(&v, &mut tokenizer, &mut tpc);
        let s2 = tokenizer.decode::<false>(&v);
        assert_eq!(s, s2)
    }

    fn check_tpc_handling(s: String) {
        let (v, mut tokenizer, mut tpc) = init(&s);
        let (v2, _) = train_step::<false>(&v, &mut tokenizer, &mut tpc);
        let mut tpc2 = TokenPairCounter::new(&v2);
        tpc.map.retain(|_, v| *v > 0);
        tpc2.map.retain(|_, v| *v > 0);
        assert_eq!(tpc.map, tpc2.map);
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

        /// Can train a tokenizer on one string, and then round trip another string with compression
        #[test]
        fn test_troundtrip() {
            let s1 = "abab".to_owned();
    
            // Train a tokenizer on s1
            let (tokenizer,train_tokenized) = BytePairEncodingTokenizer::from_corpus(&s1, 2);
            let train_roundtrip = tokenizer.decode::<false>(&train_tokenized);
            assert_eq!(s1, train_roundtrip);

            // Did it actually compress? Fewer tokens than bytes?
            assert!(train_tokenized.len() < s1.bytes().len());
        }

    /// Can train a tokenizer on one string, and then round trip another string with compression
    #[test]
    fn test_roundtrip_other() {
        let s1 = "abab".to_owned();
        let s2 = "ab ab".to_owned();

        // Train a tokenizer on s1
        let (tokenizer,_) = BytePairEncodingTokenizer::from_corpus(&s1, 2);

        // should only have 1 rule
        assert_eq!(tokenizer.encoding_table.len(), 1);

        // the rule should be (a,b) -> 256
        assert_eq!(tokenizer.encoding_table[0], ((97, 98), 256));

        // Roundtrips on s2
        let v2 = tokenizer.encode(&s2);
        // is the tokenization correct?
        assert_eq!(v2, vec![256, 32, 256]);


        let s2_decoded = tokenizer.decode::<false>(&v2);
        assert_eq!(s2, s2_decoded);

        // And did it actually compress? Fewer tokens than bytes?
        assert!(v2.len() < s2.bytes().len());
    }

    #[test]
    fn test_to_bytes_and_from_bytes() {
        let mut tokenizer = BytePairEncodingTokenizer::new();
        tokenizer.add_pair_rule((0, 1));
        tokenizer.add_pair_rule((2, 3));

        let serialized = tokenizer.to_bytes();
        let deserialized = BytePairEncodingTokenizer::from_bytes(&serialized);

        // Check decoding table
        assert_eq!(tokenizer.decoding_table, deserialized.decoding_table);

        // Check encoding table
        assert_eq!(tokenizer.encoding_table, deserialized.encoding_table);
    }

    #[test]
    fn test_from_bytes_empty() {
        let tokenizer = BytePairEncodingTokenizer::new();
        let serialized = tokenizer.to_bytes();
        let deserialized = BytePairEncodingTokenizer::from_bytes(&serialized);

        // Check decoding table
        assert_eq!(tokenizer.decoding_table, deserialized.decoding_table);

        // Check encoding table
        assert_eq!(tokenizer.encoding_table, deserialized.encoding_table);
    }

    /// Applying a trained tokenizer to the corpus it was trained on produces the same TokenizedString
    /// as the TokenizedString that was returned in the training step
    #[test]
    fn test_tokenizer_on_corpus() {
        let s = "abab bcbc abc".to_owned();
        let (tokenizer, v) = BytePairEncodingTokenizer::from_corpus(&s,1);
        let v2 = tokenizer.encode(&s);
        assert_eq!(v, v2);
    }
}
