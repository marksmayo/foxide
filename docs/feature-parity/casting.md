# Chromecast implementation checkpoint

Chromecast output remains **unaccepted** and receives no feature medal. The isolated player-cast crate now has pinned control transport, receiver-restricted file serving and owned playback operations. Desktop discovery/pairing, its session worker and physical acceptance remain.

## Transport and trust

Cast V2.1 envelopes preserve routing, JSON/binary payloads and Unicode with a 64 KiB frame cap. Declared oversize, ambiguous, malformed and truncated messages are rejected before use. Rustls with the ring provider verifies TLS 1.2/1.3 handshake signatures.

Certificate inspection returns a presented leaf-certificate SHA-256, then aborts before application commands. A control connection requires an explicitly trusted fingerprint and rejects a changed certificate. This is pinning / trust on first use, not Google's device-authentication authority validation. No verification-disabled application connection is provided.

Literal local-network addresses avoid an unbounded DNS lookup. Network operations have a total deadline, including fragmented TLS/frame reads. Request IDs start at a random nonzero value and increase within JavaScript's exact integer range. Root/application responses are correlated; unrelated frames have a bounded skip count. Transport-level heartbeats are handled before application routing, including the transport-0 endpoint.

No discovery, network request or new dependency loading is added to desktop startup. The desktop does not depend on this crate yet.

## Owned media operations

CastPlayback starts the default media receiver and loads an explicitly selected local file. Existing applications are refused by default, except those explicitly marked as an idle screen. Replacing a foreground application requires an explicit option. Paused startup is also explicit.

An initial IDLE/LOADING acknowledgement does not count as ready playback. Startup waits for PLAYING or PAUSED within its network deadline. Receiver errors, invalid clocks and failed/ended loads propagate as failures.

Pause, play, seek and stop check the captured application session and media session. A provided content ID must still match the private source URL. Optional media-information updates preserve known duration/position; they do not falsely imply lost ownership. Seek inputs reject nonfinite, negative and out-of-duration values.

Stop targets only owned media, not a receiver-wide application STOP. Dropping revokes file serving and closes control transport. The future desktop worker must maintain the connection and perform best-effort owned stop on shutdown; leaving an idle default receiver open is not a completed disconnect experience.

## Restricted local-media origin

The origin selects a local interface, binds an ephemeral port and creates a 256-bit random URL. It admits only the selected receiver IP and retains one open regular-file handle. Replacing its pathname cannot expose another file. There is no directory mapping or filename in the URL.

GET, HEAD, preflight and single byte ranges support fetching and seeking. Headers have an 8 KiB cap and three-second total deadline. At most four connection workers run. File transfers use bounded buffers, write timeouts and shutdown checks. Native socket readiness and a Mio waker avoid listener polling and allow cancellation after a network interface disappears.

Drop signals cleanup without waiting on HTTP clients; explicit shutdown waits for workers/file handles. This is LAN HTTP with a private URL and source-IP restriction. It does not encrypt the media stream or transcode unsupported codecs/containers.

TransferStats reports accepted connections, selected-peer connections, valid file requests and media bytes queued through completed writes. Those counters prove neither decoding nor displayed frames.

## Verification

Forty-five automatic cases span the envelope codec, real loopback TLS, file HTTP serving and owned playback. The playback fixture fetches actual selected-file bytes through the restricted origin before confirming its simulated player state.

Red/green cases include missing protocol/pinning APIs, interim load acknowledgement before readiness, transport heartbeat routing, optional media updates, invalid clocks and explicit receiver errors. Other cases cover changed certificates, inspection without application commands, unrelated/excessive responses, overall deadlines under trickled input, byte ranges, wrong peers/hosts/tokens, path replacement, concurrent seeks and shutdown during a slow transfer. Foreground takeover defaults and lost application/media ownership have negative tests.

The earlier transport checkpoint de1a4c3052e87baa43bd7ceefcfc648082bb0663 passed the [combined desktop/GPU coverage gate](https://github.com/marksmayo/foxide/actions/runs/44080423510) at **91.05%** (4,427/4,862 lines). That measured checkpoint predates the owned-session additions; their current CI results must be reviewed separately.

## Physical receiver acceptance: pending

Read-only status succeeded against two privately configured owner receivers. Subsequent silent-video attempts received an intermediate load acknowledgement but did not reach confirmed playback. File-server counters recorded **zero incoming connections and zero media bytes served**. Windows has enabled TCP/UDP block rules for the exact test executable, and the test process is not elevated.

A local, ignored helper was prepared for a temporary receiver-only firewall exception. It keeps other source addresses blocked, changes only the prepared test executable's TCP access, restores its original blocks afterward, and runs no player/test process as administrator. Administrator action is pending. No firewall setting was changed by the agent.

These failed runs are not a successful feature acceptance or a performance measurement. Device names, addresses, fingerprints, raw diagnostics and the helper stay in ignored local data. Display confirmation and audio acceptance remain separate.

The opt-in device test requires the device-integration feature plus explicit FOXIDE_CAST_TARGET, FOXIDE_CAST_PIN and FOXIDE_CAST_MEDIA values. It stays ignored in ordinary/offline/hosted tests. Use a short silent fixture and an idle receiver; it never forces foreground takeover. Optional FOXIDE_CAST_TRACE output uses fixed protocol labels and routing-match flags, not device identifiers, URLs or tokens.

## Engineering commands

```text
cargo run -p player-cast --example receiver-status -- inspect IP:PORT
cargo run -p player-cast --example receiver-status -- status IP:PORT SHA256_PIN
cargo test -p player-cast --features device-integration --test device_probe -- --ignored --nocapture
```

Inspection does not grant trust. The read-only diagnostic does not launch applications. The device probe does attempt media load and owned playback controls.

## Remaining work

Desktop discovery, a visible trust decision, cancellable worker/session ownership, integration with playback controls, complete disconnect/failure cleanup, receiver codec compatibility and successful physical display/audio acceptance remain. The feature total stays at **8/49 accepted**, with **41 gaps remaining**.

References: [Chromium envelope definition](https://chromium.googlesource.com/chromium/src.git/+/4d6bb36e3fcfb9742b6428e43a8e5df12681d0d0/components/cast_channel/proto/cast_channel.proto), [Rustls verifier contract](https://docs.rs/rustls/0.23.46/rustls/client/danger/trait.ServerCertVerifier.html), [Google media messages](https://developers.google.com/cast/docs/media/messages), and [Chromium transport-endpoint fix](https://chromium.googlesource.com/openscreen/+/8ef06d904f52e5595f0fbfd80ae93c123b3a35d5).
