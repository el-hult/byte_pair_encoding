use std::fs::read_to_string;

fn decode(v:Vec<usize>,mapping_table:Vec<char>) -> String {
    let mut s = vec![];
    for token in v{
        s.push(mapping_table[token]);
    }
    s.into_iter().collect::<String>()
}

fn encode(s: &str) -> (Vec<usize>,Vec<char>){
    let mut mapping_table = vec![];
    let mut v = vec![];
    for c in s.chars(){
        let idx =  mapping_table.iter().position(|&a|a==c);
        match idx {
            Some(i)=> v.push(i),
            None=> {
                let i = mapping_table.len();
                v.push(i);
                mapping_table.push(c);
            }
        }
    }
    (v,mapping_table)
}

fn main() -> () {
    let s = read_to_string("input.txt").expect("file is there");
    let (v,mapping_table) = encode(&s);
    let s2 = decode(v, mapping_table);
    println!("{}",s);
    println!("{}",s2);
    ()
}