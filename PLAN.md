# QuicVid project plan

## Purpose of this document

This file describes how the project was organized, tracked, validated, and
brought to completion.

Technical usage belongs in `README.md` and `docs/`. Detailed work discussions
belong in GitHub issues. This plan provides the project-management view:

- project objective and scope;
- milestone and epic structure;
- delivered work and acceptance gates;
- risks, decisions, and dependencies;
- remaining submission work.

## Project context

- **Project:** QuicVid
- **Student:** Tong Ki Chun
- **Advisor:** Pasi Sarolahti
- **Institution:** Aalto University, School of Electrical Engineering
- **Project type:** Bachelor's final project

The project began with a broader proposal involving quiche, Qt, FFmpeg,
mobile-style handover, and wider application comparisons. Early experiments
showed that this scope was too broad for the available project time and that a
smaller, measurable prototype would provide a stronger engineering result.

The completed project therefore focuses on a Quinn-based media prototype,
controlled dual-path Mininet experiments, proactive migration and reconnect,
and a reproducible evaluation pipeline.

Earlier planning material is retained as project history, not as the final
delivery contract.

## Estimated workload

The course workload target is approximately **270 hours**.

The project board records the detailed milestone and epic estimates. The
following table provides the planning summary.

| Work package | Existing board estimate | Status |
|---|---:|---|
| Milestone 1 — Fake-video QUIC baseline | 65 h | Complete |
| Milestone 2 — Visible JPEG transport | 43 h | Complete |
| Milestone 3 — Controlled migration | 30 h | Complete |
| Milestone 4 — Automatic migration | 39 h | Complete |
| Epic 5.1 — Continuous media timeline | 8 h | Complete |
| Epic 5.2 — Proactive reconnect baseline | 12 h | Complete |
| Epic 5.3 — Measurement and automation | 20 h | Complete |
| **Documented subtotal** | **217 h** |  |
| Pre-milestone discovery and early prototypes | **30 h** | Complete |
| Epic 5.4 — Final analysis and delivery | **10 h** | Complete |
| **Currently allocated total** | **257 h** |  |
| **Remaining toward course target** | **13 h** | Final report, meetings, validation, and submission |
| **Course target** | **270 h** |  |

The pre-milestone estimate covers:

- advisor meetings;
- project-definition and scope discussions;
- quiche and Quinn exploration;
- feasibility validation;
- the early `quinn-ping` and `mouse-coordinates` prototypes.

Epic 5.4 delivered:

- aggregate comparison statistics;
- report-ready figures;
- interpretation and limitations;
- a preview-mode demonstration clip;
- advisor-facing repository documentation;
- final documentation consistency work.

The detailed time log remains the authoritative record for final course
reporting.

## Project objective

Demonstrate and evaluate media continuity during a controlled network-path
failure.

The comparison uses one shared proactive health detector and two recovery
actions:

```text
Healthy -> Suspect -> Challenging
                         |
                         +-- migrate
                         |      preserve connection and session
                         |      rebind existing Quinn endpoint
                         |
                         +-- reconnect
                                create replacement connection and session
                                preserve logical media run and frame timeline
```

The project answers:

1. Can the media run complete after the active path fails?
2. Can migration preserve the existing QUIC connection and application
   session?
3. Can reconnect preserve the logical media timeline despite replacing the
   connection and session?
4. What receiver-visible interruption and frame loss does each strategy
   produce?
5. What trade-off appears between transport continuity and recovery behavior?

## Scope

### Included

- Quinn client and server;
- generated JPEG test-pattern media;
- QUIC DATAGRAM media transport;
- QUIC stream control messages;
- transport-independent `MediaRun`;
- controlled and automatic migration;
- proactive reconnect;
- dual-path Mininet topology;
- sustained Path A failure;
- structured logs and automated verification;
- repeated comparative experiments;
- aggregate statistics and plots;
- report-ready results and demonstration evidence.

### Excluded

- physical Wi-Fi handover;
- NetworkManager-triggered recovery;
- public-Internet NAT traversal;
- TCP baseline;
- timeout-triggered reconnect;
- repeated reconnects in one media run;
- adaptive media behavior;
- multi-party conferencing;
- production retry and backoff;
- a polished consumer-facing UI;
- broad commercial application benchmarking;
- user studies.

These exclusions are intentional scope controls rather than unfinished
requirements.

## Work-management model

The project was managed through:

```text
Milestone
    -> Epic issue
        -> small implementation commits
            -> focused tests
            -> real-run validation
            -> issue progress comments
```

Working rules:

1. Keep commits small and single-purpose.
2. Add or update tests with analysis and automation changes.
3. Validate behavior with real migration and reconnect runs.
4. Record notable implementation evidence in the relevant issue.
5. Commit compact structured results, not large reproducible raw logs.
6. Update documentation when the implemented workflow changes.

## Milestone status

### M1 — Fake-video QUIC baseline: complete

Delivered:

- Quinn endpoint setup;
- control-session handshake;
- generated frame timeline;
- DATAGRAM delivery;
- run summaries.

Acceptance met:

- client and server complete a media run without recovery.

### M2 — Visible JPEG transport: complete

Delivered:

- JPEG test-pattern generation;
- DATAGRAM fragmentation and reassembly;
- complete-frame validation;
- receiver preview.

Acceptance met:

- receiver displays and validates generated frames.

### M3 — Controlled migration: complete

Delivered:

- dual-path Mininet topology;
- Path A and Path B addressing;
- controlled `Endpoint::rebind()`;
- same connection and session after rebind;
- continued media delivery.

Acceptance met:

- controlled Path A to Path B migration completes without creating another
  application session.

### M4 — Automatic migration: complete

Delivered:

- `Healthy`, `Suspect`, and `Challenging` health states;
- ACK-progress health signal;
- deterministic alternate-address discovery;
- automatic endpoint rebind;
- resumed-progress confirmation.

Acceptance met:

- sustained Path A failure triggers automatic migration and the run completes
  through Path B.

### M5 — Comparative evaluation: complete

Delivered through Epics 5.1–5.4:

- continuous media timeline above transport sessions;
- proactive reconnect using the same detector and decision point;
- schema-versioned per-run JSON;
- cross-session frame aggregation;
- receiver-visible continuity metrics;
- noninteractive repeated experiments;
- 10 migration and 10 reconnect trials;
- aggregate statistics and final figures;
- interpreted findings and limitations;
- preview-mode demonstration evidence;
- advisor-facing documentation.

Acceptance met:

- both strategies completed all committed runs;
- migration used one connection and session;
- reconnect used two connections and sessions;
- both preserved one logical media run and continuous frame timeline;
- results are reproducible from committed structured records.

## Epic status

### Epic 5.1 — Continuous media timeline: complete

Delivered:

- transport-independent `MediaRun`;
- global frame IDs derived from run time;
- `media_run_id` and `session_id` lifecycle;
- completion through the active session;
- migration preserves run/session identity;
- reconnect preserves the run but replaces the session.

### Epic 5.2 — Proactive reconnect baseline: complete

Delivered:

- shared health detector and `Challenging` decision point;
- configurable recovery strategy;
- replacement endpoint, connection, and session;
- second HELLO for the same media run;
- resumption from the live timeline;
- reconnect-specific verification.

### Epic 5.3 — Measurement and experiment automation: complete

Delivered:

- structured log parser;
- run, session, connection, HELLO, and address identity extraction;
- cross-session frame aggregation;
- global missing, duplicate, and received-frame metrics;
- strategy-specific action timing;
- receiver-side continuity metrics;
- shared receiver clock across reconnect sessions;
- schema-versioned per-run JSON;
- flat CSV export;
- noninteractive Mininet launcher;
- interleaved repeated-run driver;
- timeout and failure handling;
- focused Python tests;
- committed 10-migration and 10-reconnect dataset.

### Epic 5.4 — Comparative results and final demo: complete

Delivered:

- aggregate descriptive statistics;
- report-ready result tables;
- receive-gap histogram;
- missing-frame frequency plot;
- interpretation of receiver interruption and frame preservation;
- transport/session identity comparison;
- outlier discussion;
- limitations and threats to validity;
- preview-mode demonstration clip;
- advisor-facing repository entry point.

Remaining work belongs to final course submission rather than implementation:

- final report editing;
- advisor walkthrough;
- final clean-checkout validation;
- submission packaging.

## Final baseline experiment

Configuration:

```text
frame rate:             10 fps
media duration:         6 s
expected frames:        60
initial path:           Path A / 10.0.1.2
alternate path:         Path B / 10.0.2.2
failure start:          2.0 s
failure duration:       sustained
suspect threshold:      250 ms
challenge threshold:    500 ms
trials per strategy:    10
execution order:        interleaved
```

Result:

| Metric | Migration | Reconnect |
|---|---:|---:|
| Successful runs | 10/10 | 10/10 |
| Median largest receive gap | 937.4 ms | 802.0 ms |
| Mean largest receive gap | 973.3 ms | 830.2 ms |
| Median missing frames | 2 | 7 |
| Mean missing frames | 2.4 | 7.6 |

Interpretation:

- reconnect restored receiver activity sooner;
- migration preserved more frames;
- migration retained one connection and session;
- reconnect created a replacement connection and session;
- both strategies completed every tested run.

The result shows a trade-off rather than one universally superior strategy.

## Deliverables

### Source deliverables

- active Rust prototype under `quic-vid/`;
- Mininet topology and launchers;
- recovery verification and analysis scripts;
- Python tests.

### Evidence deliverables

- committed per-run JSON records;
- flat trial CSV;
- aggregate statistics;
- final plots;
- preview-mode demonstration clip;
- Git revision and environment metadata.

### Documentation deliverables

- advisor-oriented project README;
- current-status document;
- Mininet workflow guide;
- recovery-analysis guide;
- result dataset README;
- final comparison analysis;
- final report.

## Quality gates

### Code gate

```bash
cargo fmt --manifest-path quic-vid/Cargo.toml --check
cargo test --manifest-path quic-vid/Cargo.toml
cargo clippy --manifest-path quic-vid/Cargo.toml   --all-targets --all-features -- -D warnings
cargo build --release --manifest-path quic-vid/Cargo.toml
```

### Python gate

```bash
python3 -m unittest discover -s tests -v
python3 -m py_compile scripts/mininet/*.py tests/*.py
```

### Experiment gate

A trial is accepted only when:

```text
scenario command completes
client.log and server.log exist
result extraction succeeds
analysis_errors is empty
media run reaches completion
identity matches the selected strategy
```

### Documentation gate

Before submission:

- no obsolete primary instructions;
- no local absolute paths in committed workflows;
- no credentials or private data;
- active and legacy directories are clearly distinguished;
- commands work from a clean checkout;
- generated artifacts and committed evidence are clearly separated.

## Reproducibility environment

The final recorded environment is:

```text
Git revision:       631d82d18d5cd4542f3132078a14fb6a7815fda6
Operating system:   Ubuntu 24.04.1 LTS
Kernel:             Linux 6.8.0-136-generic, aarch64
Python:             3.12.3
Pillow:             10.2.0
Rust compiler:      rustc 1.92.0
Cargo:              1.92.0
Open vSwitch:       3.3.4
iproute2:           6.1.0
```

## Decision record

Important project decisions:

1. **Quinn instead of the original quiche application stack**
   - reduced integration risk;
   - matched the working migration experiments.

2. **Generated JPEG workload instead of full camera/audio integration**
   - preserved visible media semantics;
   - enabled deterministic experiments.

3. **Mininet instead of physical Wi-Fi evaluation**
   - provided repeatable path control;
   - reduced environmental uncertainty.

4. **Proactive reconnect as the comparison baseline**
   - shares the same detector and decision point;
   - isolates the recovery-action difference.

5. **Continuous `MediaRun` above transport sessions**
   - makes reconnect comparable without restarting the logical call.

6. **Receiver gap as the primary interruption metric**
   - directly represents media delivery;
   - avoids comparing incompatible action-completion events.

7. **Compact structured evidence in Git**
   - improves reproducibility without committing bulky raw output.

## Completion criteria

Implementation and evaluation are complete when:

- both recovery strategies are implemented and verified;
- the continuous media timeline is demonstrated;
- repeated experiment results are reproducible;
- aggregate statistics and plots are generated;
- measured findings and limitations are documented;
- final demo evidence is available;
- the repository gives the advisor a clear evaluation path.

Final submission is complete when:

- the final report answers the engineering question;
- code, test, build, and documentation gates pass from a clean checkout;
- the repository contains no temporary or generated cache files;
- the submission package is clean and understandable.
