# Video orientation and cadence regression

The desktop preview passed `MPV_RENDER_PARAM_FLIP_Y=1` when rendering into a texture that Slint interpreted with its top-left origin. This introduced an extra vertical inversion. The corrected handoff renders normally (`FLIP_Y=0`) and explicitly selects Slint's top-left texture origin.

The portable regression runs through the same C ABI parameter construction as production. A test render function supplies four differently coloured rows and applies libmpv's documented flip semantics. The test then applies the configured Slint origin and checks that the rows retain their source order. It failed with the original flip and passes with the correction. It also checks that the render call retains nonblocking target-time behavior. Run it with:

```sh
cargo test -p player-render --lib --locked
```

An additional opt-in integration test exercises real libmpv decoding, the production OpenGL texture handoff and Slint compositing. Its fixture has a red top, blue bottom and moving middle. The test reads the completed window backbuffer, checks the orientation, and requires at least 75 distinct composited frames in the five-second observation interval of a 30 fps source. This detects frozen or severely stalled video independently of audio. It is a regression threshold, not a comparative FPS benchmark.

The `Video compositor regression` Actions workflow generates the fixture and runs under Xvfb with Mesa software OpenGL. It uploads the test log. To run on a prepared Linux test desktop:

```sh
export FOXIDE_RENDER_FIXTURE=/absolute/path/to/orientation.mkv
xvfb-run -a cargo test -p player-render --features gpu-integration --locked -- --ignored --nocapture --test-threads=1
```

Frame readback is confined to the opt-in test. Production playback still shares an OpenGL texture without application-side pixel readback. Backbuffer observations do not prove physical scanout, native hardware decoding or battery efficiency. Windows hardware playback and the reported stutter still need runtime confirmation; the orientation correction alone is not a verified fix for slow frame delivery.
