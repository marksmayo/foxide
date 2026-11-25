use std::io::{Read, Write};
use std::time::Duration;
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args[0].as_str() {
        "echo" => println!("{}", args[1]),
        "stdin" => {
            let mut bytes = Vec::new();
            std::io::stdin().read_to_end(&mut bytes).unwrap();
            std::io::stdout().write_all(&bytes).unwrap();
        }
        "large" => { std::io::stdout().write_all(&vec![b'x'; 1024 * 1024]).unwrap(); }
        "fail" => { eprintln!("fixture failure"); std::process::exit(7); }
        "sleep" => {
            std::fs::write(&args[1], "ready").unwrap();
            std::thread::sleep(Duration::from_millis(1500));
            std::fs::write(&args[2], "survived").unwrap();
        }
        _ => panic!("unknown fixture mode"),
    }
}
