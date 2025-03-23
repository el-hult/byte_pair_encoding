use std::fs::{read_to_string, write, read};
use byte_pair_encoding::BytePairEncodingTokenizer;

fn main() {
    // Train a BPE from the input.txt
    let s = read_to_string("input.txt").expect("file is there");
    let (tokenizer, v) = BytePairEncodingTokenizer::from_corpus(&s, 100);
    let s2 = tokenizer.decode::<true>(&v);
    println!("{}", s2);

    // Write the BPE to a file
    let serialized = tokenizer.to_bytes();
    write("tokenizer.bpe", serialized).expect("Failed to write tokenizer to file");

    // Load the BPE from a file
    let serialized = read("tokenizer.bpe").expect("Failed to read tokenizer file");
    let tokenizer = BytePairEncodingTokenizer::from_bytes(&serialized);

    // Apply the BPE to s again, and see it is the same as v
    let v2 = tokenizer.encode(&s);
    assert_eq!(v, v2, "Re-encoded tokens do not match the original tokens");

    // Load a different file, and apply the BPE to it, printing the colorized output
    let test_s = read_to_string("input2.txt").expect("Did not find second file");
    let test_s2 = tokenizer.decode::<true>(&tokenizer.encode(&test_s));
    println!("{}", test_s2);
}
