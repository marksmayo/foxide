# Candidate-by-candidate implementation ledger

All 200 original entries were reviewed. They were proposals, not proven wins: some overlapped, described work already optimized, assumed nonexistent allocations, or changed evidence/cleanup semantics. This ledger does **not** claim that 200 optimizations shipped.

**94 implemented**, **7 already satisfied**, **99 not adopted**. Consolidated entries share code and must not be summed as independent savings. No measured player speedup is claimed from these counts.

New behavior was developed through failing→passing tests (event tailing, final partial records, cache validation, README values). Existing public-interface tests characterized pure refactors; those tests were green before and after. Local Rust regression tests, Clippy, coverage and an exact replay of all 579 archived measurement rows validate the changed seams. Raw logs are under results/local/perf-*. The GitHub run supplies the three-platform acceptance and refreshed scoreboards.

| Original rank | Candidate | Outcome | Decision / evidence |
|---:|---|---|---|
| 1 | Cache the pinned mpv download by archive SHA, then reverify before use | implemented | Windows workflows cache only the archive directory; install() still verifies SHA before extraction. |
| 2 | Cache seed media by recipe, encoder build, flags and platform; verify restored hashes | implemented | Exact encoder-build/platform/recipe cache; every restored seed passes hash and stream validation. |
| 3 | Cache extended media by seed hashes and extension recipe; reverify restored files | implemented | Extended-media cache shares the exact recipe key and verifies seed identity and content hashes. |
| 4 | Cache Cargo registry sources using lockfile and toolchain keys | implemented | rust-cache handles registry/dependency reuse with lockfile and compiler identity. |
| 5 | Cache release build outputs under exact target/toolchain/flags keys | implemented | Release dependency outputs are cached; workspace binaries are rebuilt from source. |
| 6 | Cache normal debug dependency outputs separately from coverage outputs | implemented | Normal and llvm-cov target directories stay separate inside the cached build tree. |
| 7 | Cache instrumented dependencies separately by coverage flags and toolchain | not adopted | Covered in part by dependency caching, but no separate coverage-only cache entry; keeping one checks cache avoids duplicate large archives. |
| 8 | Cache the verified cargo-llvm-cov tool installation by version and platform | not adopted | Not adopted: installer action already manages its tool bootstrap; duplicating that cache requires evidence. |
| 9 | Cache supported Homebrew download artifacts, retaining version receipts | not adopted | Not adopted: additional workflow topology/cache maintenance lacks measured benefit beyond the implemented caches; preserve full platform validation and raw artifacts. |
| 10 | Use setup-python pip caching keyed by the coverage version | implemented | setup-python pip cache is keyed through requirements-ci.txt. |
| 11 | Let the coverage invocation supply the equivalent test pass after checking identical test discovery | implemented | Equivalent workspace tests execute through llvm-cov; removed the preceding duplicate pass. |
| 12 | Let branch-coverage unittest discovery supply the equivalent Python pass | implemented | Equivalent unittest discovery executes through coverage; removed the preceding duplicate pass. |
| 13 | Separate a selected-player benchmark dispatch from the full default field | not adopted | Not adopted: additional workflow topology/cache maintenance lacks measured benefit beyond the implemented caches; preserve full platform validation and raw artifacts. |
| 14 | Add report-only dispatch that reuses immutable raw artifacts and preserves their original provenance | not adopted | Not adopted: additional workflow topology/cache maintenance lacks measured benefit beyond the implemented caches; preserve full platform validation and raw artifacts. |
| 15 | Allow combined-only rendering when per-platform HTML artifacts are not requested | not adopted | Not adopted: additional workflow topology/cache maintenance lacks measured benefit beyond the implemented caches; preserve full platform validation and raw artifacts. |
| 16 | Skip compilation jobs on documentation-only commits while retaining a documentation check | not adopted | Not adopted: skipping required check jobs can leave branch-protection checks pending; keep the checks workflow authoritative. |
| 17 | Scope automatic smoke dispatch to playback/harness changes; retain release/manual triggers | already satisfied | Already satisfied by existing smoke path filters; documentation-only paths do not trigger it. |
| 18 | Download only session/measurement inputs into a report-only artifact variant | not adopted | Not adopted: additional workflow topology/cache maintenance lacks measured benefit beyond the implemented caches; preserve full platform validation and raw artifacts. |
| 19 | Avoid checkout in a separate artifact-only publishing job if it uses a verified report bundle | not adopted | Rejected: README regeneration and provenance require checked-out source; this was not a valid independent low-risk candidate. |
| 20 | Reuse exact-source, same-platform release binaries from a verified build artifact | not adopted | Not adopted: additional workflow topology/cache maintenance lacks measured benefit beyond the implemented caches; preserve full platform validation and raw artifacts. |
| 21 | Build the Windows inventory once per session instead of once per player | implemented | One Windows inventory per event session. |
| 22 | Pass the session-verified fixture digest into each trial | implemented | Verified fixture digests are passed to trials; metadata changes before/after trials reject mutation. |
| 23 | Compute the ASS digest once per immutable session | implemented | One external subtitle digest per session. |
| 24 | Reuse the immutable cohort profile across identical repeats | not adopted | Not adopted: only a small settings JSON is hashed per case; media and executable reads were the actual repeated large work. |
| 25 | Memoize executable hashes by resolved identity during one inventory build | implemented | One digest/size tuple per resolved executable per fingerprinted inventory. |
| 26 | Offer path-only discovery for callers that do not consume binary fingerprints | implemented | Path-only discovery for event lookup and package detection; default fingerprinted inventory preserved. |
| 27 | Cache known-folder resolutions for the duration of one inventory | implemented | Known-folder fallback results retained within a single inventory. |
| 28 | Cache failed candidate resolutions within a discovery session | implemented | Failed candidate paths memoized only during one inventory. |
| 29 | Cache successful candidate resolutions within a discovery session | implemented | Successful candidate paths memoized only during one inventory. |
| 30 | Compile the placeholder regular expression once | already satisfied | Already effectively cached by Python re; no explicit compiled-pattern object without evidence of a benefit. |
| 31 | Retain one catalog parse in a reusable discovery context | implemented | Consolidated with #21: catalog parsed once per session inventory. |
| 32 | Retain executable stat results with the discovered path | not adopted | Not adopted: Path.is_file does not expose a retained stat object; introducing a new discovery record for one extra size query is not justified. |
| 33 | Create a player-to-path dictionary from the shared inventory | implemented | Session-local player-to-executable lookup map. |
| 34 | Prebuild macOS application search prefixes once | not adopted | Not adopted: no repeated expensive work remains on this path after session inventory reuse, or the proposed cache would need new invalidation behavior. |
| 35 | Prebuild the macOS application executable map once | not adopted | Not adopted: no repeated expensive work remains on this path after session inventory reuse, or the proposed cache would need new invalidation behavior. |
| 36 | Accept a verified in-process inventory from a coordinating installer | not adopted | Not applicable: installation and event collection are separate command invocations, not one in-process coordinator. |
| 37 | Cache identical WinGet list results only within a no-install phase | not adopted | Not applicable: configured installer package IDs are unique; no duplicate list calls within a no-install phase. |
| 38 | Use a seen-ID set to skip duplicate album entries | implemented | Seen-ID set selects the first FLAC album track, retaining prior behavior. |
| 39 | Reuse an exact verified seed artifact for verify-only consumers | implemented | Consolidated with #2: every cached seed is verified before use. |
| 40 | Share resolved executable paths across cases in one hosted run | not adopted | Not adopted: no repeated expensive work remains on this path after session inventory reuse, or the proposed cache would need new invalidation behavior. |
| 41 | Build a parent-to-children adjacency map and traverse once | implemented | Iterative adjacency traversal handles long chains, cycles, absent roots and unrelated processes. |
| 42 | Cache SC_PAGE_SIZE once outside the process loop | implemented | One page-size query per Linux snapshot instead of per process. |
| 43 | Cache SC_CLK_TCK once outside the process loop | implemented | One clock-tick query per Linux snapshot instead of per process. |
| 44 | Use os.scandir for numeric /proc entries | not adopted | Not adopted: native sampling/record-flush semantics would change or the remaining micro-optimization has no measured observer benefit. |
| 45 | Parse parent IDs first and materialize RSS/CPU records only for selected descendants | not adopted | Not adopted: native sampling/record-flush semantics would change or the remaining micro-optimization has no measured observer benefit. |
| 46 | Build tree membership before converting RSS and CPU strings | implemented | macOS converts RSS and CPU values only for selected descendants. |
| 47 | Reuse Entry storage within a sampler instance | already satisfied | Already allocated once per snapshot, not once per queried process. Cross-snapshot reuse is too small to prioritize. |
| 48 | Reuse Memory storage for sequential process reads | implemented | Memory ctypes storage reused across sequential queries in one snapshot. |
| 49 | Reuse FILETIME storage for sequential process reads | implemented | FILETIME ctypes storage reused across sequential queries in one snapshot. |
| 50 | Bind frequently used FFI functions to local sampler variables | not adopted | Not adopted: native sampling/record-flush semantics would change or the remaining micro-optimization has no measured observer benefit. |
| 51 | Retain the initialized Windows sampler across trials | implemented | Windows sampler adapter is initialized once per event session. |
| 52 | Use built-in open on a prepared stat path in the hot scan | not adopted | Not adopted: native sampling/record-flush semantics would change or the remaining micro-optimization has no measured observer benefit. |
| 53 | Use partition-based parsing where field boundaries are fixed | not adopted | Not adopted: native sampling/record-flush semantics would change or the remaining micro-optimization has no measured observer benefit. |
| 54 | Fast-path the ordinary no-day CPU-time format | not adopted | Not adopted: cpu_seconds already handles the no-day path without a day-time loop; real ps subprocess work dominates. |
| 55 | Compute the CPU common-PID sum with a generator | implemented | CPU delta common-PID sequence is a generator. |
| 56 | Bind constant interval seconds outside the sampling loop | implemented | Sampling interval conversion moved outside the loop. |
| 57 | Reuse a configured compact JSONEncoder for sample records | implemented | One configured JSONEncoder per telemetry process. |
| 58 | Use compact separators for JSONL telemetry | implemented | Telemetry JSONL uses compact separators; parsed content unchanged. |
| 59 | Add bounded buffering after the ready handshake with flush on shutdown | not adopted | Rejected: buffering would reduce crash-time evidence visibility; keep per-sample flushes. |
| 60 | Bind the selected JSON encoder and write functions outside emit | not adopted | Not adopted: native sampling/record-flush semantics would change or the remaining micro-optimization has no measured observer benefit. |
| 61 | Tail new complete event lines using a retained cursor | implemented | Incremental EventTail retains complete records and partial bytes; final partial evidence rejects verification. |
| 62 | Share one event snapshot between startup and verification checks in each tick | implemented | Startup and readiness checks reuse accumulated events, not newly parsed copies. |
| 63 | Maintain an incremental verification summary while retaining raw events | not adopted | Not adopted: extra persistent verification state would duplicate subtle wrong-speed/hardware/error rules; incremental I/O removes the large repeated work. |
| 64 | Schedule log checks only when new bytes are available | implemented | Only newly appended bytes are decoded; unchanged EOF returns existing events. |
| 65 | Precompute the VLC Basic authorization header once per case | implemented | VLC authorization header is encoded once per case. |
| 66 | Precompute the VLC status URL once per case | implemented | VLC status URL is formatted once per case. |
| 67 | Reuse a case-local loopback HTTP connection with reconnect fallback | not adopted | Not adopted: HTTP keepalive changes collector timing and failure behavior; first profile reconnect overhead with the existing cadence. |
| 68 | Create the Windows snapshot adapter once per session | implemented | Consolidated with #51: one session Windows sampler. |
| 69 | Only reserve a loopback port for the VLC adapter | implemented | Only VLC reserves a status port. |
| 70 | Only generate an HTTP password for VLC | implemented | Only VLC generates an HTTP token; other command arguments are not accidentally redacted. |
| 71 | Resolve event/stdout/stderr paths once before the polling loop | implemented | Event path retained outside the polling loop; stdout/stderr were already opened once. |
| 72 | Use a single current events value for final readiness evaluation | implemented | Final evidence drains the existing tail after owned processes stop, without reparsing old lines. |
| 73 | Aggregate CPU deltas directly without a common-PID list | implemented | Event runner computes CPU common-PID deltas with a generator. |
| 74 | Create a static per-case command prefix before starting the clock | already satisfied | Already satisfied: command_for assembles arguments before the process-spawn timestamp. |
| 75 | Resolve each fixture path once per session | implemented | Fixture paths resolved once per session. |
| 76 | Resolve the shared subtitle path once per session | implemented | Subtitle path resolved once per session. |
| 77 | Convert requested player IDs to a set after validation | implemented | Requested-player set reused after validation. |
| 78 | Preselect eligible players before entering trial execution | not adopted | Not adopted: player applicability is checked once per catalog entry already, outside trial loops. |
| 79 | Reuse a prepared scenario-to-fixture lookup | already satisfied | Already satisfied: SCENARIOS and media are prepared lookup dictionaries. |
| 80 | Take one current process-tree snapshot for Unix cleanup, then recheck identities before signals | not adopted | Rejected: reducing fresh snapshots during child cleanup can weaken PID-reuse safety. |
| 81 | Append measurement rows to a journal and compact once at completion | not adopted | Not adopted: changing the raw evidence schema/checkpoint/retention contract is not a low-risk performance refactor; remaining format tweaks need data-volume evidence. |
| 82 | Remove the final duplicate measurements write when no data changed | not adopted | Not adopted: final snapshot guarantees an output for zero-case and plan-only sessions; skip-write state adds complexity for one final write. |
| 83 | Store compact measurement JSON and keep pretty summaries separately | implemented | Machine-consumed measurements use compact JSON; readable case records remain pretty. |
| 84 | Serialize case records directly into an open output stream | implemented | Case JSON streams to its output file rather than a full intermediate string. |
| 85 | Serialize session metadata directly into an output stream | implemented | Session JSON streams to its output file rather than a full intermediate string. |
| 86 | Store raw cases once and reference them from the session with a compatibility loader | not adopted | Not adopted: changing the raw evidence schema/checkpoint/retention contract is not a low-risk performance refactor; remaining format tweaks need data-volume evidence. |
| 87 | Offer a summary-only scores JSON beside the full archival bundle | not adopted | Not adopted: changing the raw evidence schema/checkpoint/retention contract is not a low-risk performance refactor; remaining format tweaks need data-volume evidence. |
| 88 | Stream large medals.json serialization to the file | implemented | Medals JSON streams to its output file. |
| 89 | Load only the session fields consumed by scoreboards | not adopted | Not adopted: changing the raw evidence schema/checkpoint/retention contract is not a low-risk performance refactor; remaining format tweaks need data-volume evidence. |
| 90 | Skip writes when generated bytes are unchanged | not adopted | Not adopted: changing the raw evidence schema/checkpoint/retention contract is not a low-risk performance refactor; remaining format tweaks need data-volume evidence. |
| 91 | Prepare only requested fixture IDs for deliberately scoped sessions | not adopted | Not adopted: changing the raw evidence schema/checkpoint/retention contract is not a low-risk performance refactor; remaining format tweaks need data-volume evidence. |
| 92 | Probe only required duration fields when validating extension output | implemented | Extension ffprobe requests only duration. |
| 93 | Request only fields needed by the contract plus preserved provenance fields | not adopted | Not adopted: complete ffprobe metadata is retained as provenance and useful for future contract checks. |
| 94 | Iterate matching run.json paths with a parser iterator before grouping | not adopted | Not adopted: changing the raw evidence schema/checkpoint/retention contract is not a low-risk performance refactor; remaining format tweaks need data-volume evidence. |
| 95 | Stream stderr through the evidence parser rather than read all text at completion | not adopted | Not adopted: changing the raw evidence schema/checkpoint/retention contract is not a low-risk performance refactor; remaining format tweaks need data-volume evidence. |
| 96 | Stream stdout through the evidence parser rather than read all text at completion | not adopted | Not adopted: changing the raw evidence schema/checkpoint/retention contract is not a low-risk performance refactor; remaining format tweaks need data-volume evidence. |
| 97 | Stream event JSONL records through evidence accumulation | not adopted | Consolidated conceptually with incremental event collection, but the legacy comparison parser is unchanged to preserve its separate evidence rules. |
| 98 | Write incremental hosted status through a journal with a final snapshot | not adopted | Not adopted: changing the raw evidence schema/checkpoint/retention contract is not a low-risk performance refactor; remaining format tweaks need data-volume evidence. |
| 99 | Stream the machine-readable summary JSON output | not adopted | Not adopted: changing the raw evidence schema/checkpoint/retention contract is not a low-risk performance refactor; remaining format tweaks need data-volume evidence. |
| 100 | Spool unusually large installer output to bounded disk logs with retained references | not adopted | Not adopted: changing the raw evidence schema/checkpoint/retention contract is not a low-risk performance refactor; remaining format tweaks need data-volume evidence. |
| 101 | Index measurements by platform, event and profile once | implemented | Measurements indexed once by platform/event/profile. |
| 102 | Index claims by feature once | implemented | Claims indexed once by feature. |
| 103 | Index session cases by platform and player once | already satisfied | Already grouped in one session pass per platform in the current small three-platform model; no cross-player rescans of session cases. |
| 104 | Index measurements by platform and player for runtime summaries | implemented | Runtime evidence indexed by platform/player. |
| 105 | Prefilter eligible platform measurements once | implemented | Cohort-local filtering replaces full-array filtering. |
| 106 | Cache event-to-profile sets for each platform | implemented | Profile sets indexed once by platform/event. |
| 107 | Memoize source-URL validity within a report build | implemented | Source-URL validation memoized within a scoreboard build. |
| 108 | Parse each URL once rather than separately for scheme and hostname | implemented | Each distinct source URL is parsed once per validity check. |
| 109 | Precompute player media-type sets | implemented | Player media-type sets built once per board. |
| 110 | Precompute feature media-type sets once per feature | implemented | Feature media-type set built once per feature. |
| 111 | Represent applicable player membership as a set during claim filtering | implemented | Claim applicability uses set membership; ordered applicable output stays unchanged. |
| 112 | Build trial uniqueness and validity in one pass | not adopted | Not adopted: existing separated validity checks are clearer and cheap after cohort indexing; no worthwhile saving established. |
| 113 | Collect valid values during validation | not adopted | Not adopted: current indexed cohorts are small; extra aggregate state/cache complexity is unproven and must not alter invalid-trial or applicability semantics. |
| 114 | Reuse profile/player trial-key sets for runtime verified counts | not adopted | Not adopted: current indexed cohorts are small; extra aggregate state/cache complexity is unproven and must not alter invalid-trial or applicability semantics. |
| 115 | Calculate verified counts during session-case indexing | not adopted | Not adopted: current indexed cohorts are small; extra aggregate state/cache complexity is unproven and must not alter invalid-trial or applicability semantics. |
| 116 | Calculate distinct runtime statuses during case indexing | not adopted | Not adopted: current indexed cohorts are small; extra aggregate state/cache complexity is unproven and must not alter invalid-trial or applicability semantics. |
| 117 | Reuse immutable player platform lists across board construction | not adopted | Not adopted: current indexed cohorts are small; extra aggregate state/cache complexity is unproven and must not alter invalid-trial or applicability semantics. |
| 118 | Avoid constructing missing-runtime fallback strings until needed | not adopted | Not adopted: current indexed cohorts are small; extra aggregate state/cache complexity is unproven and must not alter invalid-trial or applicability semantics. |
| 119 | Hoist the fixed rank-to-medal mapping outside the entrant loop | implemented | Rank-to-medal dictionary constructed once per ranking call. |
| 120 | Reuse feature award results across platforms when applicability and claim platform sets match exactly | not adopted | Rejected for this pass: platform applicability and platform-specific claims must remain explicit; cross-platform result sharing can hide differences. |
| 121 | Check filename extension before requesting entry.file_type for clearly unsupported names | implemented | Unsupported extensions rejected before file_type query. |
| 122 | Compare extensions with eq_ignore_ascii_case instead of allocating lowercase strings | implemented | Case-insensitive extension comparison avoids lowercase String allocation. |
| 123 | Classify entries before constructing a full PathBuf when possible | implemented | Full entry paths constructed only for selected media. |
| 124 | Accumulate significant numeric digits without first owning the zero-prefixed chunk | not adopted | Not adopted: natural-sort representation changes need a measured large-folder bottleneck and broader Unicode/leading-zero differential tests; retain deterministic cached keys. |
| 125 | Use chunk ranges into a retained lowercase filename key | not adopted | Not adopted: natural-sort representation changes need a measured large-folder bottleneck and broader Unicode/leading-zero differential tests; retain deterministic cached keys. |
| 126 | Reserve chunk capacity from a small measured filename heuristic | not adopted | Not adopted: there is no measured filename-chunk heuristic; the proposal assumed data that was never collected. |
| 127 | Add a semantics-equivalent ASCII path that avoids Unicode lowercase allocation where possible | not adopted | Not adopted: natural-sort representation changes need a measured large-folder bottleneck and broader Unicode/leading-zero differential tests; retain deterministic cached keys. |
| 128 | Borrow a digit-only chunk after trimming rather than clone it into another temporary | not adopted | Not adopted: natural-sort representation changes need a measured large-folder bottleneck and broader Unicode/leading-zero differential tests; retain deterministic cached keys. |
| 129 | Represent deterministic path tie-breaks as borrowed paths or stable indices in decorated entries | not adopted | Not adopted: natural-sort representation changes need a measured large-folder bottleneck and broader Unicode/leading-zero differential tests; retain deterministic cached keys. |
| 130 | Reserve the selected-files vector when a trustworthy local size hint exists | not adopted | Not applicable: read_dir supplies no trustworthy selected-file count; a counting pass would add I/O. |
| 131 | Use an extension dispatch by length before case-insensitive comparison | not adopted | Not adopted: natural-sort representation changes need a measured large-folder bottleneck and broader Unicode/leading-zero differential tests; retain deterministic cached keys. |
| 132 | Reserve the temporary lowercase-key buffer to filename byte length | not adopted | Not adopted: natural-sort representation changes need a measured large-folder bottleneck and broader Unicode/leading-zero differential tests; retain deterministic cached keys. |
| 133 | Store numeric chunk length in compact key metadata without extra text ownership | not adopted | Not adopted: natural-sort representation changes need a measured large-folder bottleneck and broader Unicode/leading-zero differential tests; retain deterministic cached keys. |
| 134 | Retain whether the canonical input was a directory | implemented | Directory state retained across folder and start logic. |
| 135 | Retain whether the canonical input was a file when autoqueue is active | not adopted | Not adopted: natural-sort representation changes need a measured large-folder bottleneck and broader Unicode/leading-zero differential tests; retain deterministic cached keys. |
| 136 | Reserve the media argument vector using the known selected queue length | implemented | Media vector reserves selected folder size. |
| 137 | Reserve final engine argument capacity before extending all media paths | implemented | Final engine argument vector reserves media capacity. |
| 138 | Use a compact enum for numeric/text sort-key chunks | not adopted | Not adopted: natural-sort representation changes need a measured large-folder bottleneck and broader Unicode/leading-zero differential tests; retain deterministic cached keys. |
| 139 | Avoid lossy filename ownership for already-valid UTF-8 names | already satisfied | Already satisfied: to_string_lossy returns a borrowed Cow for valid UTF-8. |
| 140 | Fast-path lowercase filenames containing no ASCII digits as one text chunk | not adopted | Not adopted: natural-sort representation changes need a measured large-folder bottleneck and broader Unicode/leading-zero differential tests; retain deterministic cached keys. |
| 141 | Consume the parsed sample Value instead of cloning it into from_value | implemented | Parsed sample JSON Value consumed rather than deep-cloned. |
| 142 | Read JSONL with BufReader lines instead of whole-file read_to_string | implemented | Telemetry JSONL parsed through BufReader lines. |
| 143 | Stream events with BufReader instead of loading the whole log | implemented | Engine JSONL parsed through BufReader lines. |
| 144 | Accumulate steady RSS sum/count directly without a steady-reference vector | implemented | Steady RSS accumulated without a reference Vec. |
| 145 | Accumulate CPU sum/count without collecting CPU values | implemented | CPU sum/count accumulated without a values Vec. |
| 146 | Combine peak, steady RSS and CPU accumulation into one ordered pass | not adopted | Not adopted: peak remains a separate short iterator to keep summary behavior clear; two Vec allocations were removed without further accumulator state. |
| 147 | Sort finite restart values once for both p50 and p95 | not adopted | Alternative to #148: single-quantile selection was chosen; avoid adding a separate sorted-percentile interface for tiny groups. |
| 148 | Use select_nth_unstable_by for a single nearest-rank quantile | implemented | Single nearest-rank quantile uses selection rather than full sorting. |
| 149 | Use directory-entry file_type before redundant Path.is_dir probes | not adopted | Not adopted: archive format/grouping or ownership changes add complexity without a demonstrated current bottleneck; keep cohort fields and malformed-record rejection. |
| 150 | Allow a manifest of run paths to bypass recursive archive discovery | not adopted | Not adopted: archive format/grouping or ownership changes add complexity without a demonstrated current bottleneck; keep cohort fields and malformed-record rejection. |
| 151 | Accumulate RSS means directly rather than collect group RSS vectors | not adopted | Not adopted: archive format/grouping or ownership changes add complexity without a demonstrated current bottleneck; keep cohort fields and malformed-record rejection. |
| 152 | Accumulate CPU means directly rather than collect group CPU vectors | not adopted | Not adopted: archive format/grouping or ownership changes add complexity without a demonstrated current bottleneck; keep cohort fields and malformed-record rejection. |
| 153 | Use write! into the report string instead of push_str(format!(...)) | not adopted | Not adopted: archive format/grouping or ownership changes add complexity without a demonstrated current bottleneck; keep cohort fields and malformed-record rejection. |
| 154 | Escape pipes and newlines in one pass | not adopted | Not adopted: archive format/grouping or ownership changes add complexity without a demonstrated current bottleneck; keep cohort fields and malformed-record rejection. |
| 155 | Use serde_json::to_writer_pretty with a buffered file | implemented | Run JSON streams through BufWriter with explicit flush error propagation. |
| 156 | Retain the event-log PathBuf outside the polling loop | implemented | Startup event-log PathBuf retained outside polling loop. |
| 157 | Reserve pending_line capacity from observed typical event line lengths | implemented | Implemented a small 256-byte initial line buffer, not a measured optimum. No runtime performance claim for this micro-change. |
| 158 | Reserve sample capacity from duration and interval | not adopted | Not adopted: archive format/grouping or ownership changes add complexity without a demonstrated current bottleneck; keep cohort fields and malformed-record rejection. |
| 159 | Replace serialized JSON cohort keys with a typed ordered key preserving every field | not adopted | Not adopted: archive format/grouping or ownership changes add complexity without a demonstrated current bottleneck; keep cohort fields and malformed-record rejection. |
| 160 | Share immutable diagnostic note text before final serialization | not adopted | Not adopted: archive format/grouping or ownership changes add complexity without a demonstrated current bottleneck; keep cohort fields and malformed-record rejection. |
| 161 | Deduplicate candidate roots before probing tools/mpv | implemented | Duplicate root probes filtered before filesystem checks. |
| 162 | Probe candidate roots lazily rather than first collect them all | not adopted | Not adopted: materializing at most four roots enables simple duplicate filtering; lazy root enumeration is not an established bottleneck. |
| 163 | Join the constant tools/mpv/filename suffix once | implemented | Engine discovery suffix constructed once. |
| 164 | Move subtitle_files into canonicalization instead of borrow then allocate another owned list | implemented | Subtitle input vector consumed during canonicalization. |
| 165 | Reserve capacity from fixed arguments plus selected controls and subtitles | not adopted | Not adopted: Vec literal already reserves exactly its initial fixed arguments; further capacity guesses require allocation measurements. |
| 166 | Borrow subtitle paths when converting external spelling | not adopted | Not adopted: current launch allocates only a few small buffers; additional caching/ownership interfaces have no established startup benefit and can change path semantics. |
| 167 | Borrow the resume directory while building its argument | not adopted | Not adopted: current launch allocates only a few small buffers; additional caching/ownership interfaces have no established startup benefit and can change path semantics. |
| 168 | Provide a borrowed-or-owned path spelling helper for internal argument construction | not adopted | Not adopted: current launch allocates only a few small buffers; additional caching/ownership interfaces have no established startup benefit and can change path semantics. |
| 169 | Reserve final media arguments using explicit media count | implemented | Media vector initially reserves explicit input count. |
| 170 | Precompute URL-scheme recognition without constructing helper iterators per input | not adopted | Not adopted: the literal scheme array and iterator are stack-based and optimized by Rust; this proposal assumed an allocation that is not present. |
| 171 | Share canonical paths for repeated identical explicit inputs within one launch | not adopted | Rejected: memoizing path canonicalization can change symlink/error timing and duplicate-file semantics for negligible likely benefit. |
| 172 | Share canonical paths for repeated identical subtitle inputs within one launch | not adopted | Rejected: subtitle canonicalization caching has the same path invalidation issue as #171. |
| 173 | Stream print-command JSON to locked stdout | not adopted | Not adopted: current launch allocates only a few small buffers; additional caching/ownership interfaces have no established startup benefit and can change path semantics. |
| 174 | Serialize borrowed lossy-display values in print-command mode | not adopted | Not adopted: diagnostic-only lossy output is intentionally owned for serialization; no playback benefit. |
| 175 | Append formatted numeric flags directly into their final argument buffers | not adopted | Not adopted: current launch allocates only a few small buffers; additional caching/ownership interfaces have no established startup benefit and can change path semantics. |
| 176 | Pre-size long subtitle option buffers from prefix and path lengths | not adopted | Not adopted: current launch allocates only a few small buffers; additional caching/ownership interfaces have no established startup benefit and can change path semantics. |
| 177 | Pre-size the resume-directory option buffer | not adopted | Not adopted: current launch allocates only a few small buffers; additional caching/ownership interfaces have no established startup benefit and can change path semantics. |
| 178 | Extend media into an already-sized final vector rather than grow twice | implemented | Consolidated with #137: final argument vector reserves before extension. |
| 179 | Pre-size the benchmark script option buffer from prefix and canonical path | not adopted | Not adopted: current launch allocates only a few small buffers; additional caching/ownership interfaces have no established startup benefit and can change path semantics. |
| 180 | Reserve the candidate-root vector for its maximum four roots | implemented | Candidate-root vector reserves its bounded four-entry capacity. |
| 181 | Build a player-ID-to-name map once per board | implemented | Player names indexed once per HTML board. |
| 182 | Index performance cohorts by event ID before rendering | implemented | Performance cohorts indexed once by event before HTML rendering. |
| 183 | Reuse escaped player labels between table headers and performance bars | not adopted | Not adopted: remaining work is tiny at 18 players; extra caches/event scheduling lack evidence and could complicate accessibility or display state. |
| 184 | Cache validated source-link fragments within a render | not adopted | Not adopted: remaining work is tiny at 18 players; extra caches/event scheduling lack evidence and could complicate accessibility or display state. |
| 185 | Append SVG fragments to a list and join once | implemented | SVG fragments appended and joined once. |
| 186 | Append markdown rows to a list and join once | implemented | Markdown fragments appended and joined once. |
| 187 | Precompute escaped event names and search keys | not adopted | Not adopted: remaining work is tiny at 18 players; extra caches/event scheduling lack evidence and could complicate accessibility or display state. |
| 188 | Reuse static feature heading fragments across platforms | not adopted | Not adopted: remaining work is tiny at 18 players; extra caches/event scheduling lack evidence and could complicate accessibility or display state. |
| 189 | Precompute feature-count scale once per chart | not adopted | Not adopted: remaining work is tiny at 18 players; extra caches/event scheduling lack evidence and could complicate accessibility or display state. |
| 190 | Cache each player's total performance medals | not adopted | Not adopted: remaining work is tiny at 18 players; extra caches/event scheduling lack evidence and could complicate accessibility or display state. |
| 191 | Cache the search-row NodeList once per input element | implemented | Search rows cached per search input. |
| 192 | Compute the lowercase query once per input event | implemented | Search query lowercased once per input event. |
| 193 | Skip DOM hidden assignment when row visibility is already correct | implemented | Unchanged row visibility is not assigned again. |
| 194 | Skip work when the normalized search query has not changed | implemented | Repeated normalized search query returns early. |
| 195 | Cache platform button and section NodeLists once | implemented | Platform button and section arrays cached. |
| 196 | Return early when the already-active platform tab is clicked | implemented | Clicking the already-active tab returns early. |
| 197 | Only toggle visibility on the old and new active sections | not adopted | Alternative covered by #193/#195: code compares current hidden state before writes, so only changed sections receive visibility writes. |
| 198 | Use input-value normalization outside requestAnimationFrame batching | not adopted | Rejected: asynchronous filtering is unnecessary for 30 rows and complicates deterministic search state. |
| 199 | Prebuild escaped claim notes for reused source tooltips | not adopted | Not adopted: remaining work is tiny at 18 players; extra caches/event scheduling lack evidence and could complicate accessibility or display state. |
| 200 | Reuse chart output for standalone SVG and embedded HTML | implemented | One SVG render per platform shared by standalone and inline outputs. |
