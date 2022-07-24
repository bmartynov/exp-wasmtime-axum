extern crate wee_alloc;

use std::io::{Read, Write};

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn main() {
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();

    let mut s = String::new();

    stdin.read_to_string(&mut s).expect("ok");

    stdout.write(s.as_bytes()).expect("ok");
}