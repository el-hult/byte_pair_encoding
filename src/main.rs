use std::fs::read_to_string;
fn main() -> () {
    let s = read_to_string("input.txt").expect("file is there");
    print!("{}",s);
    ()
}