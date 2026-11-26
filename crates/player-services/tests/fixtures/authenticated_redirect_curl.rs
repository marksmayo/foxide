use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut config = String::new();
    std::io::stdin().read_to_string(&mut config).unwrap();

    // Model curl's documented behavior: custom headers are reused on redirect
    // when --location is enabled. Surface any forwarded header in stderr so
    // the caller can assert no credential reaches the redirect target.
    if args.iter().any(|arg| arg == "--location") {
        eprintln!("redirect target received request headers: {config}");
    }
    eprintln!("redirect response rejected");
    std::process::exit(22);
}

