# GitHub Actions runtime pins

The JavaScript actions below use Node.js 24, verified from action.yml at the pinned release commit on 2026-11-16. Workflows use immutable commit IDs and leave Dependabot enabled for updates.

| Action | Release | Commit |
|---|---|---|
| [actions/checkout](https://github.com/actions/checkout/releases/tag/v7.0.1) | v7.0.1 | 3d3c42e5aac5ba805825da76410c181273ba90b1 |
| [actions/cache](https://github.com/actions/cache/releases/tag/v6.1.0) | v6.1.0 | 55cc8345863c7cc4c66a329aec7e433d2d1c52a9 |
| [actions/setup-python](https://github.com/actions/setup-python/releases/tag/v7.1.0) | v7.1.0 | 6d4fd22b20465ccc0f7354476588e8df7c9ca0e0 |
| [actions/upload-artifact](https://github.com/actions/upload-artifact/releases/tag/v7.0.2) | v7.0.2 | cf430e030ddbb5b0abf93d22962f4752f3646cd9 |
| [actions/download-artifact](https://github.com/actions/download-artifact/releases/tag/v8.0.2) | v8.0.2 | 9000827ccba6bdab643e8b6fd33ac0654aef8333 |

Hosted runners in the benchmark run report Actions Runner 2.338.2, above the Node.js 24 minimum (2.327.1). Checkout's additional container credential requirement is 2.329.0; these jobs run directly on the host. Self-hosted UI runners should use a current runner version.

The selected action inputs were checked against the pinned manifests. Cache paths, Python versions, artifact names and normal archive behavior are unchanged. Checkout's protection against unsafe privileged PR checkout stays enabled; no workflow uses pull_request_target or opts out of that protection.
