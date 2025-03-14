use std::{collections::HashMap, fs::read_to_string, iter::zip};

type CharPair = (char,char);

fn collect_counts(s:&str)->HashMap<CharPair,u64> {
    let mut hm = HashMap::new();
    let left = s.chars();
    let right = s.chars().skip(1);
    let pairs = zip(left,right);
    for pair in pairs {
        *hm.entry(pair).or_insert(0) += 1;
    }
    hm
}
fn main() -> () {
    let s = read_to_string("input.txt").expect("file is there").to_lowercase();
    let hm = collect_counts(&s);
    let (most_common_pair,_) = hm.iter().max_by_key(
        |(_,&c)| c
    ).expect("There should be at least one pair");
    print!("{:?}",most_common_pair);
    ()
}