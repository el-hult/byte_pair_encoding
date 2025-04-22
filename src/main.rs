use std::fs::{read_to_string, write, read};
use byte_pair_encoding::BytePairEncodingTokenizer;

fn main() {
    // Train a BPE from the input.txt
    let s = read_to_string("train.txt").expect("file is there");
    let (tokenizer, encoded1) = BytePairEncodingTokenizer::from_corpus(&s, 3);
    let s2 = tokenizer.decode_color(&encoded1);
    println!("{}", s2);
    println!("There are {} tokens in the BPE ({} tokens except the raw bytes)", tokenizer.vocab_size(), tokenizer.vocab_size()-256);

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
    assert_eq!(encoded1, encoded2, "Re-encoded tokens do not match the original tokens");

    // Load a different file, and apply the BPE to it, printing the colorized output
    let test_s = read_to_string("test.txt").expect("Did not find second file");
    let test_s2 = tokenizer.decode_color(&tokenizer.encode(&test_s));
    println!("{}", test_s2);
}
