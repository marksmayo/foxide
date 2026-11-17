# Chromecast transport checkpoint

Chromecast output remains **unaccepted** and receives no feature medal. The current slice provides bounded protocol framing, certificate inspection and a pinned, read-only receiver-status query. It does not launch, stop or load receiver applications.

## Implemented boundary

- Cast V2.1 envelopes preserve JSON and binary payloads, routing and Unicode. A 64 KiB frame limit applies before allocation from a declared size. Invalid, ambiguous, truncated and unsupported envelopes are rejected.
- An isolated `player-cast` crate uses Rustls with the ring provider. Both TLS 1.2 and TLS 1.3 verify handshake signatures. There is no verification-disabled application connection.
- Inspection captures the presented leaf-certificate SHA-256 and aborts the handshake before application commands. It neither authenticates the device nor grants/saves trust.
- Status requires an explicitly supplied fingerprint. A changed certificate fails before application commands. This is explicit certificate pinning / trust on first use, **not Google's device-authentication authority validation**.
- Literal local-network addresses avoid an unbounded DNS lookup. The whole operation has a deadline, including fragmented TLS and frame reads. Heartbeats are answered while unrelated messages are skipped, with a 64-message response limit.
- No receiver discovery, network activity or dependency loading is added to application startup. The desktop does not depend on this crate yet.

## Verification

The protocol fixtures were written before the missing module/API and failed, then passed after implementation. The same red/green cycle covers certificate fingerprints, pinned status and non-trusting inspection. Tests use real generated certificates and loopback TLS rather than mocking TLS verification.

Acceptance cases include TLS 1.2/1.3, certificate changes before application data, inspection without application commands, heartbeats, unrelated/malformed/excessive responses, invalid addresses/timeouts and a trickled frame exceeding the overall deadline.

A private local diagnostic inspected two owner-specified receivers, then obtained read-only status using their observed fingerprints. No media was sent and no receiver application was launched or stopped. Device names, addresses, fingerprints and raw diagnostics remain in ignored local data; they are absent from public evidence. A first-contact fingerprint does not independently prove device identity.

## Engineering diagnostic

```text
cargo run -p player-cast --example receiver-status -- inspect IP:PORT
cargo run -p player-cast --example receiver-status -- status IP:PORT SHA256_PIN
```

Use only an explicitly trusted receiver fingerprint for the second command. Status output omits application/receiver names, identifiers and URLs. This diagnostic is not a completed casting frontend.

The transport checkpoint at de1a4c3052e87baa43bd7ceefcfc648082bb0663 passed the [full desktop/GPU coverage gate](https://github.com/marksmayo/foxide/actions/runs/44080423510) at **91.05%** (4,427/4,862 lines). This is suite coverage, not proof of feature parity.

## Local-media origin in progress

The isolated crate now also provides an opt-in, single-file HTTP origin. It selects the local interface for the receiver, binds an ephemeral port, creates a 256-bit random URL token and admits only that receiver IP. It retains the selected open file handle, so replacing a path cannot expose another file. There is no directory mapping or filename in the URL.

GET, HEAD, preflight and single byte ranges support receiver fetching and seeking. Request headers have an 8 KiB cap and a three-second total deadline. At most four connection workers run; writes have timeouts and observe shutdown. The listener waits on native socket readiness with a Mio waker for shutdown, rather than a polling timer or a self-connection. Dropping wakes its owner without waiting for HTTP clients; explicit shutdown waits for connection/file cleanup. This also allows shutdown after an advertised network interface disappears. This is LAN HTTP protected by a private URL and source-IP restriction, not encrypted media transport. It does not transcode containers or codecs.

Tests cover ranges, suffixes/clamping, malformed/oversized requests, duplicate headers, wrong peers/hosts/tokens, concurrent seeks, path replacement, multiple chunks and cancellation of a slow transfer. The origin is not connected to a desktop cast command yet, and no receiver has fetched media.

## Still required

Discovery, a pairing/trust decision in the desktop, cancellable session ownership, integration of the bounded local-media HTTP origin, receiver launch/load/pause/seek/stop controls, cleanup after disconnect and physical playback/pixel/audio acceptance remain. Session operations must not stop an application owned by another sender. Receiver-specific codec compatibility and any firewall requirements must be documented when playback is implemented.

The [Chromium Cast envelope definition](https://chromium.googlesource.com/chromium/src.git/+/4d6bb36e3fcfb9742b6428e43a8e5df12681d0d0/components/cast_channel/proto/cast_channel.proto) defines framing fields. [Rustls verifier documentation](https://docs.rs/rustls/0.23.46/rustls/client/danger/trait.ServerCertVerifier.html) defines the certificate and handshake-signature verification boundary.
