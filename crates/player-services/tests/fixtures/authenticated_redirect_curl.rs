use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut config = String::new();
    std::io::stdin().read_to_string(&mut config).unwrap();

    if args.iter().any(|arg| arg.contains("size-limit-test")) {
        let limit = args
            .windows(2)
            .find(|pair| pair[0] == "--max-filesize")
            .and_then(|pair| pair[1].parse::<usize>().ok())
            .expect("curl was given a response size limit");
        std::io::Write::write_all(&mut std::io::stdout(), &vec![b'x'; limit + 1]).unwrap();
        return;
    }

    // Model curl's documented behavior: custom headers are reused on redirect
    // when --location is enabled. Surface any forwarded header in stderr so
    // the caller can assert no credential reaches the redirect target.
    if args.iter().any(|arg| arg == "--location") {
        eprintln!("redirect target received request headers: {config}");
    }
    eprintln!("redirect response rejected");
    std::process::exit(22);
}

