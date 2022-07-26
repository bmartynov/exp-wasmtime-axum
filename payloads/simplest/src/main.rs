extern crate wee_alloc;

use std::io;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn main() {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    io::copy(&mut stdin, &mut stdout).expect("ok");
}