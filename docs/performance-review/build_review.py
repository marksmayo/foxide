"""Render the manually reviewed, ranked optimization candidate inventory."""
from pathlib import Path
import csv
import html
import json

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
# Each line is a separate candidate, not an implemented or measured improvement.
# source | anchor | action | expected work avoided
GROUPS = [
('CI', 'High potential; cache-dependent', '''
.github/workflows/player-games.yml|Install the Windows field|Cache the pinned mpv download by archive SHA, then reverify before use|Repeated download of the full engine archive
.github/workflows/player-games.yml|Generate and verify deterministic seed corpus|Cache seed media by recipe, encoder build, flags and platform; verify restored hashes|Repeated AVC, HEVC and AV1 seed encodes on exact cache hits
.github/workflows/player-games.yml|Extend matched bitstreams|Cache extended media by seed hashes and extension recipe; reverify restored files|Repeated 90-second remuxes and FLAC encoding
.github/workflows/ci.yml|rust-and-sampler|Cache Cargo registry sources using lockfile and toolchain keys|Dependency downloads across three platform jobs
.github/workflows/ci.yml|rust-and-sampler|Cache release build outputs under exact target/toolchain/flags keys|Repeated dependency compilation for release builds
.github/workflows/ci.yml|rust-and-sampler|Cache normal debug dependency outputs separately from coverage outputs|Repeated clippy and test dependency compilation
.github/workflows/ci.yml|Enforce Rust workspace line coverage|Cache instrumented dependencies separately by coverage flags and toolchain|Repeated compilation of coverage dependencies
.github/workflows/ci.yml|taiki-e/install-action|Cache the verified cargo-llvm-cov tool installation by version and platform|Tool bootstrap work across runs
.github/workflows/player-games.yml|Install the macOS field|Cache supported Homebrew download artifacts, retaining version receipts|Repeated formula and cask downloads
.github/workflows/ci.yml|Enforce medal policy and report coverage|Use setup-python pip caching keyed by the coverage version|Repeated coverage wheel downloads
.github/workflows/ci.yml|cargo test --workspace|Let the coverage invocation supply the equivalent test pass after checking identical test discovery|One duplicate Rust test execution per platform; retain fmt and clippy
.github/workflows/ci.yml|python -m unittest|Let branch-coverage unittest discovery supply the equivalent Python pass|One duplicate Python test execution per platform
.github/workflows/player-games.yml|Install the Windows field|Separate a selected-player benchmark dispatch from the full default field|Optional installation work for deliberately scoped reruns; full release runs unchanged
.github/workflows/player-games.yml|report:|Add report-only dispatch that reuses immutable raw artifacts and preserves their original provenance|All player installs and trials when only presentation changes
.github/workflows/player-games.yml|Render this platform report|Allow combined-only rendering when per-platform HTML artifacts are not requested|Three redundant platform HTML renders; retain raw results
.github/workflows/ci.yml|on:|Skip compilation jobs on documentation-only commits while retaining a documentation check|Three platform rebuilds on pure prose or generated-score changes
.github/workflows/hosted-benchmarks.yml|name:|Scope automatic smoke dispatch to playback/harness changes; retain release/manual triggers|Smoke installs and runs on unrelated documentation edits
.github/workflows/player-games.yml|report:|Download only session/measurement inputs into a report-only artifact variant|Transfer and unpacking of logs not consumed by aggregation
.github/workflows/ci.yml|uses: actions/checkout|Avoid checkout in a separate artifact-only publishing job if it uses a verified report bundle|Repository checkout for artifact-only operations; requires separating the job
.github/workflows/player-games.yml|cargo build --release|Reuse exact-source, same-platform release binaries from a verified build artifact|Duplicate release builds across workflows; exclude instrumented binaries
'''),
('Discovery and hashing', 'Substantial on large binaries/media', '''
scripts/run_event_benchmarks.py|locate|Build the Windows inventory once per session instead of once per player|P full inventory scans become one; each currently hashes every discovered executable
scripts/run_event_benchmarks.py|collect|Pass the session-verified fixture digest into each trial|One full media hash per trial after the initial verification is already complete
scripts/run_event_benchmarks.py|collect|Compute the ASS digest once per immutable session|Repeated subtitle hashing across players and repeats
scripts/run_event_benchmarks.py|collect|Reuse the immutable cohort profile across identical repeats|Repeated sorted JSON encoding and SHA calculation of identical cohort settings
scripts/compare_players.py|inventory|Memoize executable hashes by resolved identity during one inventory build|Duplicate reads if catalog entries resolve to the same executable
scripts/compare_players.py|inventory|Offer path-only discovery for callers that do not consume binary fingerprints|All executable hashing in a path-only lookup; benchmark provenance still fingerprints once
scripts/compare_players.py|resolve|Cache known-folder resolutions for the duration of one inventory|Repeated SHGetFolderPathW calls for the same placeholders
scripts/compare_players.py|resolve|Cache failed candidate resolutions within a discovery session|Repeated file/PATH probes for identical absent candidates
scripts/compare_players.py|resolve|Cache successful candidate resolutions within a discovery session|Repeated path resolution and metadata probes for shared candidates
scripts/compare_players.py|resolve|Compile the placeholder regular expression once|Repeated pattern lookup at each candidate; small relative to hashing
scripts/compare_players.py|inventory|Retain one catalog parse in a reusable discovery context|Repeated JSON parsing when multiple discovery consumers share one process
scripts/compare_players.py|inventory|Retain executable stat results with the discovered path|An additional stat for executable_bytes after file discovery
scripts/run_event_benchmarks.py|locate|Create a player-to-path dictionary from the shared inventory|Linear player lookup on every locate call
scripts/run_event_benchmarks.py|locate|Prebuild macOS application search prefixes once|Repeated home-directory and prefix construction per player
scripts/run_event_benchmarks.py|locate|Prebuild the macOS application executable map once|Recreating the same map on every lookup
scripts/install-players.py|main|Accept a verified in-process inventory from a coordinating installer|A second full inventory when installation and benchmarking share orchestration
scripts/install-players.py|install_packages|Cache identical WinGet list results only within a no-install phase|Repeated package-list subprocesses for duplicated requested IDs
scripts/prepare-event-media.py|main|Use a seen-ID set to skip duplicate album entries|Repeated linear scans of already-prepared fixture IDs
scripts/generate-media.py|main|Reuse an exact verified seed artifact for verify-only consumers|Accidental regeneration in consumers that only need validation
scripts/hosted-benchmarks.py|locate|Share resolved executable paths across cases in one hosted run|Repeated filesystem/PATH discovery across case construction
'''),
('Process telemetry', 'Moderate observer savings; host-dependent', '''
scripts/telemetry.py|descendants|Build a parent-to-children adjacency map and traverse once|Repeated whole-process-map scans for each process-tree depth
scripts/telemetry.py|linux_snapshot|Cache SC_PAGE_SIZE once outside the process loop|One sysconf lookup per process per sample
scripts/telemetry.py|linux_snapshot|Cache SC_CLK_TCK once outside the process loop|A second sysconf lookup per process per sample
scripts/telemetry.py|linux_snapshot|Use os.scandir for numeric /proc entries|Path objects for every process-directory entry
scripts/telemetry.py|linux_snapshot|Parse parent IDs first and materialize RSS/CPU records only for selected descendants|Numeric conversions and tuple allocations for unrelated processes
scripts/telemetry.py|mac_snapshot|Build tree membership before converting RSS and CPU strings|CPU-time parsing and memory conversions for unrelated processes
scripts/telemetry.py|windows_snapshot|Reuse Entry storage within a sampler instance|Repeated ctypes allocation per snapshot; preserve size initialization
scripts/telemetry.py|windows_snapshot|Reuse Memory storage for sequential process reads|One ctypes structure allocation per selected process per sample
scripts/telemetry.py|windows_snapshot|Reuse FILETIME storage for sequential process reads|Four ctypes allocations per selected process per sample
scripts/telemetry.py|windows_snapshot|Bind frequently used FFI functions to local sampler variables|Attribute resolution in every process query
scripts/telemetry.py|windows_snapshot|Retain the initialized Windows sampler across trials|Repeated DLL binding and ctypes class setup in collect
scripts/telemetry.py|linux_snapshot|Use built-in open on a prepared stat path in the hot scan|Path/read_text helper overhead per process; benchmark before retaining
scripts/telemetry.py|mac_snapshot|Use partition-based parsing where field boundaries are fixed|Temporary token lists per ps row; preserve lstart identity verbatim
scripts/telemetry.py|cpu_seconds|Fast-path the ordinary no-day CPU-time format|Day-prefix parsing on every ordinary process record
scripts/telemetry.py|main|Compute the CPU common-PID sum with a generator|The temporary common-PID list per sample
scripts/telemetry.py|main|Bind constant interval seconds outside the sampling loop|Repeated division of interval-ms every tick
scripts/telemetry.py|main|Reuse a configured compact JSONEncoder for sample records|Repeated encoder setup per emitted sample
scripts/telemetry.py|main|Use compact separators for JSONL telemetry|Whitespace bytes in every raw telemetry record
scripts/telemetry.py|main|Add bounded buffering after the ready handshake with flush on shutdown|Per-sample flush calls for an archival-only consumer; not the readiness/event channel
scripts/telemetry.py|main|Bind the selected JSON encoder and write functions outside emit|Repeated method and global lookup for every telemetry record
'''),
('Event collection', 'Moderate; scales with trials and log size', '''
scripts/run_event_benchmarks.py|read_events|Tail new complete event lines using a retained cursor|Rereading the entire event file on each readiness poll
scripts/run_event_benchmarks.py|collect|Share one event snapshot between startup and verification checks in each tick|Two read_events calls in ticks where both checks run
scripts/run_event_benchmarks.py|collect|Maintain an incremental verification summary while retaining raw events|Repeated scans of all old stats on readiness checks
scripts/run_event_benchmarks.py|collect|Schedule log checks only when new bytes are available|Repeated JSON parsing when the event log is unchanged
scripts/run_event_benchmarks.py|collect|Precompute the VLC Basic authorization header once per case|Base64 encoding on every HTTP poll
scripts/run_event_benchmarks.py|collect|Precompute the VLC status URL once per case|Repeated URL string formatting for HTTP polls
scripts/run_event_benchmarks.py|collect|Reuse a case-local loopback HTTP connection with reconnect fallback|Repeated connection setup per VLC status poll; preserve timeout and authentication
scripts/run_event_benchmarks.py|collect|Create the Windows snapshot adapter once per session|Repeated Windows FFI initialization across every trial
scripts/run_event_benchmarks.py|collect|Only reserve a loopback port for the VLC adapter|Socket bind/close for mpv, Foxide and frontend trials
scripts/run_event_benchmarks.py|collect|Only generate an HTTP password for VLC|Cryptographic random-token generation for non-HTTP adapters
scripts/run_event_benchmarks.py|collect|Resolve event/stdout/stderr paths once before the polling loop|Repeated Path joins on the timed path
scripts/run_event_benchmarks.py|collect|Use a single current events value for final readiness evaluation|Extra final event-file read when already drained after process cleanup
scripts/run_event_benchmarks.py|collect|Aggregate CPU deltas directly without a common-PID list|One list allocation per process-tree sample
scripts/run_event_benchmarks.py|collect|Create a static per-case command prefix before starting the clock|Repeated static argument assembly in future repeated launches
scripts/run_event_benchmarks.py|main|Resolve each fixture path once per session|Repeated resolve calls for the same file on every trial
scripts/run_event_benchmarks.py|main|Resolve the shared subtitle path once per session|Repeated resolve calls even in scenarios without subtitles
scripts/run_event_benchmarks.py|main|Convert requested player IDs to a set after validation|Linear selection-list membership for each catalog player
scripts/run_event_benchmarks.py|main|Preselect eligible players before entering trial execution|Repeated platform and selection predicates in orchestration extensions
scripts/run_event_benchmarks.py|main|Reuse a prepared scenario-to-fixture lookup|Repeated dictionary chains and path assembly per trial
scripts/run_event_benchmarks.py|stop_owned|Take one current process-tree snapshot for Unix cleanup, then recheck identities before signals|Full-system snapshots repeated for each owned child; identity checks must remain fresh
'''),
('Result I/O and preparation', 'Moderate to low; data-volume dependent', '''
scripts/run_event_benchmarks.py|main|Append measurement rows to a journal and compact once at completion|Rewriting the entire growing measurements array after every case
scripts/run_event_benchmarks.py|main|Remove the final duplicate measurements write when no data changed|One complete extra serialization and file write
scripts/run_event_benchmarks.py|main|Store compact measurement JSON and keep pretty summaries separately|Indentation bytes in machine-consumed measurements
scripts/run_event_benchmarks.py|collect|Serialize case records directly into an open output stream|The complete intermediate pretty JSON string per case
scripts/run_event_benchmarks.py|main|Serialize session metadata directly into an output stream|The complete intermediate session JSON string
scripts/run_event_benchmarks.py|main|Store raw cases once and reference them from the session with a compatibility loader|Duplicate sample/event/status payloads in session and case files
scripts/build_medal_report.py|write_report|Offer a summary-only scores JSON beside the full archival bundle|Transfer and parsing of raw measurements for summary consumers
scripts/build_medal_report.py|write_report|Stream large medals.json serialization to the file|The complete intermediate serialized JSON string
scripts/build_medal_report.py|main|Load only the session fields consumed by scoreboards|Retaining bulky samples and raw event arrays in report memory
scripts/build_medal_report.py|write_report|Skip writes when generated bytes are unchanged|Filesystem writes and downstream uploads for identical generated files
scripts/prepare-event-media.py|main|Prepare only requested fixture IDs for deliberately scoped sessions|Remuxing/encoding unused media; default full suite remains unchanged
scripts/prepare-event-media.py|main|Probe only required duration fields when validating extension output|Unneeded format metadata in ffprobe stdout
scripts/generate-media.py|probe|Request only fields needed by the contract plus preserved provenance fields|Large ffprobe output where extra tags are not consumed
scripts/hosted-benchmarks.py|build_reports|Iterate matching run.json paths with a parser iterator before grouping|An intermediate list of fully parsed runs before aggregation; preserve all summaries
scripts/compare_players.py|one_trial|Stream stderr through the evidence parser rather than read all text at completion|Large log strings; preserve the raw log file
scripts/compare_players.py|one_trial|Stream stdout through the evidence parser rather than read all text at completion|Large stdout strings; preserve the raw log file
scripts/compare_players.py|one_trial|Stream event JSONL records through evidence accumulation|Whole event-file text plus split-lines list
scripts/hosted-benchmarks.py|main|Write incremental hosted status through a journal with a final snapshot|Repeated whole-record serialization as cases accumulate
scripts/build_comparison_report.py|main|Stream the machine-readable summary JSON output|An intermediate serialized summary string for large fields
scripts/install-players.py|run|Spool unusually large installer output to bounded disk logs with retained references|Unbounded captured installer stdout/stderr in memory; preserve audit evidence
'''),
('Medal computation', 'Low now; moderate for larger history', '''
scripts/medal_events.py|scoreboards|Index measurements by platform, event and profile once|Full measurement scans for every event on every platform
scripts/medal_events.py|scoreboards|Index claims by feature once|Full claim scans across all feature events
scripts/medal_events.py|scoreboards|Index session cases by platform and player once|Repeated evidence filtering while constructing runtime outcomes
scripts/medal_events.py|scoreboards|Index measurements by platform and player for runtime summaries|A complete measurement scan for every player
scripts/medal_events.py|scoreboards|Prefilter eligible platform measurements once|Rebuilding the player-filtered measurement list per performance cohort
scripts/medal_events.py|scoreboards|Cache event-to-profile sets for each platform|Repeated scans merely to discover cohort names
scripts/medal_events.py|feature_medals|Memoize source-URL validity within a report build|Repeated urlparse calls for shared official documentation links
scripts/medal_events.py|feature_medals|Parse each URL once rather than separately for scheme and hostname|Two urlparse calls for each accepted source
scripts/medal_events.py|scoreboards|Precompute player media-type sets|Repeated set creation for every feature-player pair
scripts/medal_events.py|scoreboards|Precompute feature media-type sets once per feature|Repeated construction inside the player applicability comprehension
scripts/medal_events.py|scoreboards|Represent applicable player membership as a set during claim filtering|Linear list membership checks per claim
scripts/medal_events.py|performance_medals|Build trial uniqueness and validity in one pass|Separate trial-set and validity passes; preserve all-invalid rejection rules
scripts/medal_events.py|performance_medals|Collect valid values during validation|A second value extraction traversal before median calculation
scripts/medal_events.py|scoreboards|Reuse profile/player trial-key sets for runtime verified counts|Repeated tuple/set construction for the same case rows
scripts/medal_events.py|scoreboards|Calculate verified counts during session-case indexing|An additional full case pass per player
scripts/medal_events.py|scoreboards|Calculate distinct runtime statuses during case indexing|A later set comprehension over each player's cases
scripts/medal_events.py|scoreboards|Reuse immutable player platform lists across board construction|Repeated membership work across board consumers
scripts/medal_events.py|scoreboards|Avoid constructing missing-runtime fallback strings until needed|Fallback formatting that session inventory will immediately replace
scripts/medal_events.py|performance_medals|Hoist the fixed rank-to-medal mapping outside the entrant loop|One dictionary construction per ranked entrant
scripts/medal_events.py|scoreboards|Reuse feature award results across platforms when applicability and claim platform sets match exactly|Duplicate URL validation and award dictionaries; never merge differing applicability
'''),
('Folder playback', 'Low; strongest on large folders', '''
crates/player-core/src/queue.rs|folder_queue|Check filename extension before requesting entry.file_type for clearly unsupported names|File-type queries for non-media directory entries
crates/player-core/src/queue.rs|folder_queue|Compare extensions with eq_ignore_ascii_case instead of allocating lowercase strings|One lowercase String allocation per candidate file
crates/player-core/src/queue.rs|folder_queue|Classify entries before constructing a full PathBuf when possible|Full paths for rejected folder entries
crates/player-core/src/queue.rs|natural_key|Accumulate significant numeric digits without first owning the zero-prefixed chunk|A temporary numeric chunk allocation; preserve leading-zero tie behavior
crates/player-core/src/queue.rs|natural_key|Use chunk ranges into a retained lowercase filename key|One String allocation per sort-key chunk
crates/player-core/src/queue.rs|natural_key|Reserve chunk capacity from a small measured filename heuristic|Repeated Vec growth for episode-like names
crates/player-core/src/queue.rs|natural_key|Add a semantics-equivalent ASCII path that avoids Unicode lowercase allocation where possible|Unicode conversion machinery for ordinary ASCII episode names
crates/player-core/src/queue.rs|natural_key|Borrow a digit-only chunk after trimming rather than clone it into another temporary|Numeric key string copying; depends on retained-key storage
crates/player-core/src/queue.rs|folder_queue|Represent deterministic path tie-breaks as borrowed paths or stable indices in decorated entries|Cloning each full path into its cached sort key
crates/player-core/src/queue.rs|folder_queue|Reserve the selected-files vector when a trustworthy local size hint exists|Vector reallocations in large directories; do not pre-scan to count
crates/player-core/src/queue.rs|folder_queue|Use an extension dispatch by length before case-insensitive comparison|Unnecessary comparisons against impossible extension lengths
crates/player-core/src/queue.rs|natural_key|Reserve the temporary lowercase-key buffer to filename byte length|Growth copies while constructing owned sort-key storage
crates/player-core/src/queue.rs|natural_key|Store numeric chunk length in compact key metadata without extra text ownership|Per-chunk heap metadata; validate arbitrary long digit sequences
crates/player-cli/src/main.rs|main|Retain whether the canonical input was a directory|Repeated is_dir checks while selecting folder and start logic
crates/player-cli/src/main.rs|main|Retain whether the canonical input was a file when autoqueue is active|Repeated metadata lookup before locating the selected episode
crates/player-cli/src/main.rs|main|Reserve the media argument vector using the known selected queue length|Repeated vector growth while extending a large queue
crates/player-cli/src/main.rs|main|Reserve final engine argument capacity before extending all media paths|Large argument-vector growth when opening whole folders
crates/player-core/src/queue.rs|folder_queue|Use a compact enum for numeric/text sort-key chunks|Repeated bool/usize layout overhead in large key collections
crates/player-core/src/queue.rs|folder_queue|Avoid lossy filename ownership for already-valid UTF-8 names|Temporary name conversions; retain non-Unicode ordering behavior
crates/player-core/src/queue.rs|natural_key|Fast-path lowercase filenames containing no ASCII digits as one text chunk|Per-character chunk assembly for plain alphabetic filenames
'''),
('Rust benchmark and summaries', 'Low; large archives benefit most', '''
crates/player-bench/src/runner.rs|finish_samples|Consume the parsed sample Value instead of cloning it into from_value|A deep JSON Value clone for every telemetry sample
crates/player-bench/src/runner.rs|finish_samples|Read JSONL with BufReader lines instead of whole-file read_to_string|The full telemetry text allocation
crates/player-bench/src/runner.rs|events|Stream events with BufReader instead of loading the whole log|The full event-log text allocation
crates/player-bench/src/model.rs|summarize|Accumulate steady RSS sum/count directly without a steady-reference vector|One Vec of sample references per run summary
crates/player-bench/src/model.rs|summarize|Accumulate CPU sum/count without collecting CPU values|One numeric Vec per run summary
crates/player-bench/src/model.rs|summarize|Combine peak, steady RSS and CPU accumulation into one ordered pass|Multiple complete sample traversals
crates/player-bench/src/report.rs|render|Sort finite restart values once for both p50 and p95|Two percentile vector copies and sorts per report group
crates/player-bench/src/model.rs|percentile|Use select_nth_unstable_by for a single nearest-rank quantile|Full sort for callers needing only one percentile; preserve finite filtering
crates/player-bench/src/report.rs|collect|Use directory-entry file_type before redundant Path.is_dir probes|Additional metadata requests during archive recursion
crates/player-bench/src/report.rs|collect|Allow a manifest of run paths to bypass recursive archive discovery|Traversing unrelated artifact directories on repeated report runs
crates/player-bench/src/report.rs|render|Accumulate RSS means directly rather than collect group RSS vectors|One temporary Vec per cohort
crates/player-bench/src/report.rs|render|Accumulate CPU means directly rather than collect group CPU vectors|A second temporary Vec per cohort
crates/player-bench/src/report.rs|render|Use write! into the report string instead of push_str(format!(...))|One temporary formatted row String per cohort
crates/player-bench/src/report.rs|escape|Escape pipes and newlines in one pass|Two successive string replacement allocations
crates/player-bench/src/runner.rs|save|Use serde_json::to_writer_pretty with a buffered file|Whole serialized run Vec allocation before write
crates/player-bench/src/runner.rs|one|Retain the event-log PathBuf outside the polling loop|Repeated path joins during startup observation
crates/player-bench/src/runner.rs|one|Reserve pending_line capacity from observed typical event line lengths|Repeated buffer growth during the first startup event read
crates/player-bench/src/runner.rs|finish_samples|Reserve sample capacity from duration and interval|Repeated sample-vector growth when loading long telemetry
crates/player-bench/src/report.rs|render|Replace serialized JSON cohort keys with a typed ordered key preserving every field|JSON serialization and allocated key text per run
crates/player-bench/src/runner.rs|base_run|Share immutable diagnostic note text before final serialization|Repeated note String construction across many short trials; needs ownership design
'''),
('Launcher and argument policy', 'Tiny; do not expect playback CPU gains', '''
crates/player-cli/src/main.rs|mpv_path|Deduplicate candidate roots before probing tools/mpv|Duplicate filesystem lookups when cwd and executable ancestors coincide
crates/player-cli/src/main.rs|mpv_path|Probe candidate roots lazily rather than first collect them all|Unused PathBuf allocations when the first candidate exists
crates/player-cli/src/main.rs|mpv_path|Join the constant tools/mpv/filename suffix once|Repeated construction of the same suffix per root
crates/player-cli/src/main.rs|main|Move subtitle_files into canonicalization instead of borrow then allocate another owned list|Retaining the original subtitle PathBuf list during canonicalization
crates/player-core/src/playback.rs|mpv_args|Reserve capacity from fixed arguments plus selected controls and subtitles|Several small Vec reallocations per launch
crates/player-core/src/playback.rs|mpv_args|Borrow subtitle paths when converting external spelling|Cloning PathBufs that do not need Windows prefix conversion
crates/player-core/src/playback.rs|mpv_args|Borrow the resume directory while building its argument|An unnecessary PathBuf clone before external spelling conversion
crates/player-core/src/lib.rs|external_path|Provide a borrowed-or-owned path spelling helper for internal argument construction|Whole path clones where the spelling is already accepted
crates/player-cli/src/main.rs|main|Reserve final media arguments using explicit media count|Vector growth for a multi-file open
crates/player-cli/src/main.rs|main|Precompute URL-scheme recognition without constructing helper iterators per input|Tiny per-input dispatch overhead; preserve exact accepted schemes
crates/player-cli/src/main.rs|main|Share canonical paths for repeated identical explicit inputs within one launch|Repeated filesystem canonicalization; keep duplicate playlist entries intact
crates/player-cli/src/main.rs|main|Share canonical paths for repeated identical subtitle inputs within one launch|Repeated subtitle filesystem resolution; keep duplicate options intact
crates/player-cli/src/main.rs|main|Stream print-command JSON to locked stdout|The complete pretty JSON String allocation in diagnostic mode
crates/player-cli/src/main.rs|main|Serialize borrowed lossy-display values in print-command mode|Owned Strings for every already-valid Unicode argument
crates/player-core/src/playback.rs|mpv_args|Append formatted numeric flags directly into their final argument buffers|Temporary formatting buffers where profiling shows duplicate copies
crates/player-core/src/playback.rs|mpv_args|Pre-size long subtitle option buffers from prefix and path lengths|Growth copies while appending large subtitle paths
crates/player-core/src/playback.rs|mpv_args|Pre-size the resume-directory option buffer|A buffer growth when appending a long watch-later path
crates/player-cli/src/main.rs|main|Extend media into an already-sized final vector rather than grow twice|Transient backing-storage copies across media and engine argument vectors
crates/player-cli/src/main.rs|main|Pre-size the benchmark script option buffer from prefix and canonical path|Buffer growth when appending a long diagnostic script path
crates/player-cli/src/main.rs|mpv_path|Reserve the candidate-root vector for its maximum four roots|Small vector reallocations when building discovery roots
'''),
('HTML rendering and interaction', 'Tiny at current 18-player scale', '''
scripts/build_medal_report.py|render|Build a player-ID-to-name map once per board|Linear player searches for every measured performance bar
scripts/build_medal_report.py|render|Index performance cohorts by event ID before rendering|Repeated scans of all board performance entries
scripts/build_medal_report.py|render|Reuse escaped player labels between table headers and performance bars|Repeated HTML escaping of identical names
scripts/build_medal_report.py|source_link|Cache validated source-link fragments within a render|Repeated URL parsing and escaping for shared links
scripts/build_medal_report.py|chart|Append SVG fragments to a list and join once|Repeated growing-string concatenation for chart markup
scripts/build_medal_report.py|markdown|Append markdown rows to a list and join once|Repeated growing markdown-string concatenation
scripts/build_medal_report.py|render|Precompute escaped event names and search keys|Repeated lowercasing/escaping across platform panels
scripts/build_medal_report.py|render|Reuse static feature heading fragments across platforms|Repeated formatting of identical feature names and tier labels
scripts/build_medal_report.py|chart|Precompute feature-count scale once per chart|Repeated max/count/division setup for every player bar
scripts/build_medal_report.py|chart|Cache each player's total performance medals|Repeated summation for max scaling and labels
scripts/build_medal_report.py|render|Cache the search-row NodeList once per input element|DOM query on every search keystroke
scripts/build_medal_report.py|render|Compute the lowercase query once per input event|Repeated lowercasing for every feature/performance row
scripts/build_medal_report.py|render|Skip DOM hidden assignment when row visibility is already correct|Redundant DOM writes during filtering
scripts/build_medal_report.py|render|Skip work when the normalized search query has not changed|A full row-filter pass for equivalent input values
scripts/build_medal_report.py|render|Cache platform button and section NodeLists once|Repeated querySelectorAll calls on every tab change
scripts/build_medal_report.py|render|Return early when the already-active platform tab is clicked|Repeated aria and hidden writes for the same platform
scripts/build_medal_report.py|render|Only toggle visibility on the old and new active sections|Writes to every platform section on each tab change
scripts/build_medal_report.py|render|Use input-value normalization outside requestAnimationFrame batching|Multiple filter passes within one paint for rapid synthetic input; preserve final input state
scripts/build_medal_report.py|render|Prebuild escaped claim notes for reused source tooltips|Repeated escaping of the same notes in shared-gold cells
scripts/build_medal_report.py|write_report|Reuse chart output for standalone SVG and embedded HTML|Rendering all three charts twice during one write_report call
'''),
]

def build():
    rows = []
    for scope, band, block in GROUPS:
        lines = [line for line in block.strip().splitlines() if line.strip()]
        assert len(lines) == 20, (scope, len(lines))
        for line in lines:
            path, anchor, action, saving = line.split('|')
            text = (ROOT / path).read_text('utf-8')
            location = next((i for i, s in enumerate(text.splitlines(), 1) if anchor in s), None)
            assert location, (path, anchor)
            prevention = 'no new saving today' in saving
            rows.append(dict(rank=len(rows)+1, scope=scope, expected_savings=band,
                             action=action, source=path, line=location, anchor=anchor,
                             saved_work=saving, risk='Existing guardrail' if prevention else 'Low-risk candidate; validate first',
                             evidence='Already implemented; retain' if prevention else 'Static inspection; unmeasured',
                             measured_saving=None))
    assert len(rows) == 200 and len({r['action'] for r in rows}) == 200
    (OUT/'candidates.json').write_text(json.dumps(rows, indent=2)+'\n', encoding='utf-8')
    with (OUT/'candidates.csv').open('w', newline='', encoding='utf-8-sig') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader(); writer.writerows(rows)
    intro = '''# Foxide performance review — 200 ranked candidates

Reviewed 2026-11-14 against the local source that produced the first Player Games report. Audit only: no production changes were made. These are **200 optimization candidates**. There are not 200 demonstrated low-risk playback wins in this small launcher. No wall-time, RAM, CPU, or battery saving below has been measured.

The rank is an ordinal implementation shortlist by expected saved work in the current repository: expensive repeated CI/encoding/download work first, repeated large-file reads next, observer overhead, then archive/report work and launcher micro-allocations. Items in the same scope/band are approximately tied, not precise predictions. Runner minutes, observer CPU and player startup are different objectives; filter by scope before choosing work. A low global rank does not mean an optimization helps playback. Savings overlap and must not be added together. Cache-dependent entries can save nothing on a miss; orchestration changes only apply to explicitly scoped runs. Individual micro-optimizations may be below noise or slower in practice.

**Risk:** zero risk cannot be promised. All new entries are candidates for low-risk implementation only with the stated behavior preserved and regression validation. Caching requires exact identities, immutable inputs, checksum verification and safe misses. A session-local digest cache requires fixtures to remain immutable throughout the session; detect modification and fail rather than silently accept stale provenance. Do not cache process identities across queries that authorize termination. Reporting/indexing must preserve unknowns, ties, invalid-trial rejection, platform applicability and cohort separation. Do not shorten trials, reduce repeats, weaken coverage, skip evidence, disable security checks, or change player settings to manufacture savings.

**Validation before adoption:** use red → green public-interface tests for each behavior change; retain ≥90% Rust production line coverage and the existing Python policy gate. For pure refactors compare old/new structured outputs over the archived three-platform sessions, including malformed and partial logs. For telemetry measure observer CPU, wall time, allocations and sample timing separately from player CPU, with process churn/PID reuse tests. For launcher/queue changes compare argument vectors and deterministic order (Unicode, leading zeros, UNC paths, symlinks and failures), then benchmark 1/100/10,000-entry folders. For CI compare cold/miss/hit runs with equivalent test discovery and artifacts. Reject changes whose benefit does not exceed measurement noise. Instrumentation changes create a new benchmark cohort; rerun all compared players. Keep checksums and cleanup identity validation intact.

**Best first batch:** ranks 21, 22, 41, 61, 65 and 200: one inventory, one verified digest per immutable fixture, linear process-tree traversal, event-log tailing, one authorization encoding, and one SVG render. Rank 1–12 caches may save more elapsed CI time but depend on hit rates. None establish a battery or displayed-TTFF improvement. Native libmpv/GPU integration, decoder changes, rendering quality reductions, larger demux caches, thread-count changes and frame dropping are excluded as architectural or behavioral changes with higher risk.

[Searchable HTML](report.html) · [Machine-readable JSON](candidates.json) · [CSV](candidates.csv)

| Rank | Scope / expected benefit | Candidate | Work avoided (unmeasured) | Source | Evidence |
|---:|---|---|---|---|---|
'''
    for r in rows:
        intro += f"| {r['rank']} | {r['scope']} / {r['expected_savings']} | {r['action']} | {r['saved_work']} | [{r['source']}:{r['line']}](../../{r['source']}) | {r['evidence']} |\n"
    (OUT/'README.md').write_text(intro, encoding='utf-8')
    esc = html.escape
    table = ''.join(f"<tr data-search='{esc((r['scope']+' '+r['action']+' '+r['saved_work']).lower(), quote=True)}'><td>{r['rank']}</td><td><b>{esc(r['action'])}</b><small>{esc(r['scope'])} · {esc(r['expected_savings'])}</small></td><td>{esc(r['saved_work'])}<small>{esc(r['evidence'])}</small></td><td><a href='../../{esc(r['source'])}'>{esc(r['source'])}:{r['line']}</a><small>{esc(r['risk'])}</small></td></tr>" for r in rows)
    page = '''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Foxide · Performance opportunities</title><style>body{margin:0;background:#0b1220;color:#e7edf7;font:16px/1.6 system-ui}main{max-width:1450px;margin:auto;padding:48px 4vw}h1{font-size:clamp(34px,5vw,64px);line-height:1.1}em{color:#ff985c;font-style:normal}p{max-width:1000px;color:#b4c2d8}a{color:#9bc9ff}aside{background:#19263b;border-left:4px solid #e8be65;padding:20px;max-width:1000px}input,select{font:inherit;padding:12px;background:#15233a;color:#fff;border:1px solid #425675;border-radius:8px;margin:8px 8px 25px 0;max-width:90%}table{border-collapse:collapse;width:100%;font-size:14px}td,th{text-align:left;padding:17px 12px;border-bottom:1px solid #28374f;vertical-align:top}td:nth-child(2){width:36%}small{display:block;color:#8fa5c3;font-size:12px;margin-top:6px}.scroll{overflow:auto}thead{color:#ffc183}b{font-weight:600}[hidden]{display:none}footer{color:#8fa5c3;margin-top:30px}</style><main><span>🦊 FOXIDE / ENGINEERING REVIEW</span><h1>Less repeated work.<br><em>200 ranked review entries.</em></h1><p>200 optimization candidates. Ranked by expected saved work across the current repository, from CI and large-file reads to tiny launcher allocations. Same-band ordering is approximate. These are proposals, not measured gains or 200 playback optimizations.</p><aside>No zero-risk promise. All savings are unmeasured and overlap. Scope matters: faster CI does not mean faster playback. Preserve hashes, process identity checks, sample schedules, repeated trials, coverage and medal eligibility. See the linked review for validation conditions.</aside><p><a href="README.md">Method and validation requirements</a> · <a href="candidates.json">JSON</a> · <a href="candidates.csv">CSV</a></p><input id="search" aria-label="Search candidates" placeholder="Search candidate or saved work"><select id="scope" aria-label="Filter scope"><option value="">All scopes</option>OPTIONS</select><span id="count">200 entries</span><div class="scroll"><table><thead><tr><th>Rank</th><th>Candidate / expected benefit</th><th>Expected work avoided</th><th>Source / risk</th></tr></thead><tbody>ROWS</tbody></table></div><footer>Static source review · 2026-11-14 · No application or benchmark settings changed</footer></main><script>const rows=[...document.querySelectorAll('tbody tr')],q=document.getElementById('search'),scope=document.getElementById('scope');function filter(){const term=q.value.toLowerCase(),selected=scope.value.toLowerCase();let n=0;for(const row of rows){row.hidden=!(row.dataset.search.includes(term)&&(!selected||row.dataset.search.startsWith(selected+' ')));if(!row.hidden)n++;}document.getElementById('count').textContent=n+' entries';}q.addEventListener('input',filter);scope.addEventListener('change',filter);</script></html>'''
    page=page.replace('OPTIONS',''.join(f'<option>{esc(g[0])}</option>' for g in GROUPS)).replace('ROWS',table)
    (OUT/'report.html').write_text(page, encoding='utf-8')
    print(f'{len(rows)} entries; {sum(r["evidence"].startswith("Already") for r in rows)} existing guardrails; sources validated')

if __name__ == '__main__':
    build()
