use std::fs::read_to_string;
use byte_pair_encoding::{BytePairEncodingTokenizer, Token};

fn main() {
    let s = read_to_string("input.txt").expect("file is there");
    let tokenizer = BytePairEncodingTokenizer::from_corpus(&s, 100);

    let v = s.bytes().map(|b| b.into()).collect::<Vec<Token>>();
    let s2 = tokenizer.decode::<true>(&v);
    println!("{}", s2);

    let test_s = read_to_string("input2.txt").expect("Did not find second file");
    let test_s2 = tokenizer.decode::<true>(&tokenizer.encode(&test_s));
    println!("{}", test_s2);
}
