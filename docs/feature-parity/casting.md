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

## Still required

Discovery, a pairing/trust decision in the desktop, cancellable session ownership, bounded local-media HTTP serving, receiver launch/load/pause/seek/stop controls, cleanup after disconnect and physical playback/pixel/audio acceptance remain. Session operations must not stop an application owned by another sender. Receiver-specific codec compatibility and any firewall requirements must be documented when playback is implemented.

The [Chromium Cast envelope definition](https://chromium.googlesource.com/chromium/src.git/+/4d6bb36e3fcfb9742b6428e43a8e5df12681d0d0/components/cast_channel/proto/cast_channel.proto) defines framing fields. [Rustls verifier documentation](https://docs.rs/rustls/0.23.46/rustls/client/danger/trait.ServerCertVerifier.html) defines the certificate and handshake-signature verification boundary.
