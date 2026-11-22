# Isolated Windows GPU comparison

The local desktop is excluded from this workflow. Use **Isolated Windows GPU comparison** (`.github/workflows/gpu-comparison.yml`) with a configured Windows GPU runner label, a trusted absolute ANGLE runtime directory and an explicit libmpv ABI2 DLL path. The runner needs FFmpeg/ffprobe and an interactive desktop capable of D3D/OpenGL presentation. Builds and fixture generation finish before measurement; no compiler runs alongside the trials.

GitHub's GPU larger runners are available to Team/Enterprise organizations and billed separately; this personal repository cannot assume access to one. A dedicated remote Windows GPU machine registered as a repository runner is another option. No paid runner is provisioned or dispatched automatically.

The workflow builds one candidate executable and compares its default and ANGLE modes on the same machine. It runs three compositor pixel checks per mode, rejects software GPU renderers and ANGLE copy-back fallback during the pixel check, then collects five randomized30-second HEVC trials per mode after5-second warmup. It retains runtime/binary/fixture hashes, driver inventory, commands, raw process samples, engine events, logs and a summary artifact.

The same executable starts at Foxide's default window dimensions on the same desktop/DPI. No user app activity should share the dedicated runner. GitHub VM scheduling noise can still exist. Pixel tests inspect composited frames; the performance collector observes engine counters and process CPU/RSS. It does not measure physical scanout or power, and different delivered frame rates can invalidate a direct CPU comparison. These experimental results are excluded from the main medal scoreboard.

After the workflow exists on the default branch, select **Actions → Isolated Windows GPU comparison → Run workflow**, choose the reviewed source branch, and supply the configured label and runtime paths. Do not substitute the default Windows hosted label and interpret a software GPU fallback as hardware evidence.

References: [GitHub larger runner eligibility](https://docs.github.com/en/actions/concepts/runners/larger-runners), [GPU specifications](https://docs.github.com/en/actions/reference/runners/larger-runners).

Current blocker: no eligible Windows GPU runner label or dedicated remote machine has been provided. Standard Actions already validate portable code/builds and software compositor behavior; they do not reproduce the Windows hardware regression.
