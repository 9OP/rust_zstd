extern crate net7212;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <command>", args[0]);
        return;
    }

    let command = &args[1];

    match command.as_str() {
        "run" => {
            net7212::do_something();
        }
        _ => {
            println!("Unknown command: {}", command);
        }
    }
}