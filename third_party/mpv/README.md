# Scaler lookup-table padding

The Linux compositor regression uses mpv 0.40.0, commit
`e48ac7ce08462f5e33af6ef9deeac6fa87eef01e`, built from upstream source with
[scaler-lut-padding.patch](scaler-lut-padding.patch). The patch changes one
allocation from `talloc_array` to `talloc_zero_array`. It preserves computed
filter coefficients and initializes unused RGBA lookup-table channels.

On the hosted Mesa 25.2.8 llvmpipe renderer, the distro libmpv 0.37 control
produced black video while its decoded Y/U/V textures contained the expected
image. Failed LUTs had finite computed coefficients and 156–166 NaNs in unused
padding channels. Filling the allocation with NaNs before upstream computes
its valid coefficients reproduced the failure in three independent SDL/OpenGL
trials; zero-initializing the allocation passed three trials with 153 distinct
frames each. These results are in [the independent control run](https://github.com/marksmayo/foxide/actions/runs/43733627114).

The permanent control workflow repeats the padded build twelve times. Its
negative control accepts exit status 2 only after at least 75 actual render
calls, finite computed LUT coefficients, NaNs in unused channels, no colored
video frames and no OpenGL readback error. Setup/build failures are failures,
not successful negative controls. The full Foxide rounded-layer compositor
still requires at least 75 distinct upright frames in five seconds and zero
inverted observations, including a separate instrumented pass.

This is a CI dependency patch, not a claim that downloaded Windows/macOS or
system Linux libmpv binaries have been patched. Those runtimes require their
own acceptance testing; an affected unpatched runtime can still show this
upstream rendering problem. No scaler-quality downgrade, CPU pixel readback
or per-frame GPU wait is added to production playback. Baseline-player
benchmark packages are unchanged and retain their recorded provenance.

Upstream source and licensing: [mpv 0.40.0](https://github.com/mpv-player/mpv/tree/e48ac7ce08462f5e33af6ef9deeac6fa87eef01e),
[scaler allocation](https://github.com/mpv-player/mpv/blob/e48ac7ce08462f5e33af6ef9deeac6fa87eef01e/video/out/gpu/video.c),
[filter coefficient generation](https://github.com/mpv-player/mpv/blob/e48ac7ce08462f5e33af6ef9deeac6fa87eef01e/video/out/filter_kernels.c),
and [LGPL license](https://github.com/mpv-player/mpv/blob/e48ac7ce08462f5e33af6ef9deeac6fa87eef01e/LICENSE.LGPL).
The patch follows the upstream file's LGPL terms. CI uses the source tree's
default GPL-enabled build and publishes test logs/coverage, not this library
as an application release artifact.
