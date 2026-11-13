# Foxide test media

Generated artifacts live under `media/generated/` and are ignored by Git. Run:

```powershell
python scripts/generate-media.py --profile smoke --output media/generated/smoke
python scripts/generate-media.py --profile full --output media/generated/full
python scripts/generate-media.py --verify-only --output media/generated/full
```

The default smoke profile is three seconds at the specified resolutions/codecs. It checks the pipeline; it cannot validate seeks or battery life. Full fixtures are 660 seconds so a 30-second warmup and 600-second measurement finish before EOF. A and B together require roughly 8.3 GB, plus AV1, FLAC, temporary encoder space and traces. Encoding 4K in software can take hours. `--files A` or `--seconds 45 --profile smoke` lets you exercise a smaller subset. Output directories are immutable once their manifest exists.

| ID | Contents | Intended use |
|---|---|---|
| A | 1920×1080/30 AVC, 8-bit 4:2:0, CBR target 20 Mbps, AAC 192 kbps, MP4 | Startup, RAM, standard playback |
| B | 3840×2160/60 HEVC Main10, CBR target 80 Mbps, BT.2020/PQ HDR10 metadata, AC3 640 kbps, ASS and SRT tracks, MKV | Decode, subtitle and pacing stress |
| C | 3840×2160/60 AV1 10-bit, Opus 192 kbps, MKV | Codec coverage and fallback |
| D | Three FLAC tracks, 48 kHz lossless synthetic audio, embedded 600×600 PNG and album/track tags | Audio/cover/queue behavior |

AC3 is the standard B audio branch; add a separately hashed, legally sourced TrueHD variant for TrueHD/passthrough acceptance. These are synthetic assets, not film clips. B converts an SDR test signal to a PQ signal with 100-nit content; it exercises HDR signaling but does not establish high-luminance tone-map quality. Add a licensed, independently validated HDR reference with >1000-nit highlights, mastering metadata, gradients and skin tones before claiming correct HDR reproduction.

The manifest saves exact commands, encoder versions/configuration, probes, byte sizes and SHA-256 values. The input signals and recipes are deterministic; encoded bytes are only comparable within a pinned FFmpeg/encoder build. The same source recipe with a new toolchain is a new suite revision. Validate codec, pixel format, transfer, tracks and duration automatically; review actual bitrate from file/probe, including short-clip rate-control deviations, before approving the suite. Media verification reads the full files and warms their cache, so run it before reboot for cold trials.
