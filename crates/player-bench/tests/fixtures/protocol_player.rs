// A test-only process at the media-engine seam. It simulates the event protocol,
// never decoded frames. The harness must keep displayed TTFF and power unknown.
use std::{fs::File, io::Write, thread, time::Duration};
fn main() {
    let mut file = File::create(std::env::var_os("FOXIDE_BENCH_EVENTS").unwrap()).unwrap();
    writeln!(file, "{{\"kind\":\"first-playback-restart-proxy\"}}").unwrap();
    writeln!(file, "{{\"kind\":\"stats\",\"hwdec\":\"no\"}}").unwrap();
    if std::env::var("FOXIDE_BENCH_SEEKS").as_deref() == Ok("1") {
        writeln!(file, "{{\"kind\":\"seeks-complete\"}}").unwrap();
    }
    file.flush().unwrap();
    thread::sleep(Duration::from_secs(30));
}
