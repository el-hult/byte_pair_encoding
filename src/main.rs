use std::fs::{read_to_string, write, read};
use byte_pair_encoding::BytePairEncodingTokenizer;

fn main() {
    // Train a BPE from the input.txt
    let s = read_to_string("input.txt").expect("file is there");
    let (tokenizer, encoded1) = BytePairEncodingTokenizer::from_corpus(&s, 10);
    let s2 = tokenizer.decode::<true>(&encoded1);
    println!("{}", s2);
    println!("There are {} tokens in the BPE", tokenizer.vocab_size());

    // Write the BPE to a file
    let serialized = tokenizer.to_bytes();
    write("tokenizer.bpe", serialized).expect("Failed to write tokenizer to file");

    // Load the BPE from a file
    let serialized = read("tokenizer.bpe").expect("Failed to read tokenizer file");
    let tokenizer2 = BytePairEncodingTokenizer::from_bytes(&serialized);

    // Illustrate that the tokenizer2 is the same as the original tokenizer
    assert_eq!(tokenizer, tokenizer2, "Loaded tokenizer is not the same as the original tokenizer");

    // Apply the BPE to s again, and see it is the same as v
    let encoded2 = tokenizer2.encode(&s);
    // HEISENBUG! This assert fails on my example input, and I don't know why (yet)
    assert_eq!(encoded1, encoded2, "Re-encoded tokens do not match the original tokens");

    // Load a different file, and apply the BPE to it, printing the colorized output
    let test_s = read_to_string("input2.txt").expect("Did not find second file");
    let test_s2 = tokenizer.decode::<true>(&tokenizer.encode(&test_s));
    println!("{}", test_s2);
}
