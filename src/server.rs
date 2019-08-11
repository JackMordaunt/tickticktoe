use laminar::{Socket};
use std::thread;

fn main() {
    let mut s = Socket::bind("127.0.0.1:8080").unwrap();
    thread::spawn(move || s.start_polling());
}


