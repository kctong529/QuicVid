# QuicVid project plan

## Purpose of this document

This file describes how the project is organized, tracked, validated, and
brought to completion.

Technical usage belongs in `README.md` and `docs/`. Detailed work discussions
belong in GitHub issues. This plan provides the project-management view:

- project objective and scope;
- milestone and epic structure;
- current status;
- deliverables and acceptance gates;
- risks, decisions, and dependencies;
- remaining work toward submission.

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

The active plan therefore focuses on a Quinn-based media prototype, controlled
dual-path Mininet experiments, proactive migration and reconnect, and a
reproducible evaluation pipeline.

Earlier planning material is retained as project history, not as the current
delivery contract.


## Estimated workload

The course workload target is approximately **270 hours**.

The project board already records estimated hours for the milestone and epic
issues. Those existing estimates remain the primary planning figures.

| Work package | Existing board estimate | Planning status |
|---|---:|---|
| Milestone 1 — Fake-video QUIC baseline | 65 h | Documented |
| Milestone 2 — Visible JPEG transport | 43 h | Documented |
| Milestone 3 — Controlled migration | 30 h | Documented |
| Milestone 4 — Automatic migration | 39 h | Documented |
| Epic 5.1 — Continuous media timeline | 8 h | Documented |
| Epic 5.2 — Proactive reconnect baseline | 12 h | Documented |
| Epic 5.3 — Measurement and automation | 20 h | Documented |
| **Documented subtotal** | **217 h** |  |
| Pre-milestone discovery and early prototypes | **30 h** | Meetings, scope validation, technology exploration, and early prototypes |
| Epic 5.4 — Final analysis and delivery | **10 h** | Remaining analysis, report, demos, and submission work |
| **Currently allocated total** | **257 h** |  |
| **Remaining toward course target** | **13 h** | Unallocated reserve or additional documented work |
| **Course target** | **270 h** |  |

The pre-milestone estimate covers work completed before the milestone structure
was introduced:

- advisor meetings;
- project-definition and scope discussions;
- quiche and Quinn exploration;
- feasibility validation;
- the early `quinn-ping` and `mouse-coordinates` prototypes.

The 10-hour Epic 5.4 estimate covers the remaining final-analysis and delivery
work:

- aggregate statistics and plots;
- interpretation and limitations;
- final report updates;
- verified demonstration clips;
- advisor walkthrough;
- clean-checkout validation and submission packaging.

With these allocations, **257 hours** are currently accounted for. This is close
to the approximately **270-hour** course target and should be sufficient for
project-planning purposes. Any remaining difference can be covered by final
reporting, meetings, documentation, validation, and submission work.

This table is a project-planning summary. The detailed time log remains the
authoritative record for final course reporting.

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

The project should answer:

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

The project is managed through:

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
7. Move statistical interpretation and presentation work to Epic 5.4 rather
   than expanding Epic 5.3 indefinitely.

## Milestone structure

### M1 — Fake-video QUIC baseline: complete

Goal:

- establish a working Quinn client/server baseline;
- send generated media-like traffic over QUIC DATAGRAMs;
- record delivery behavior.

Delivered:

- Quinn endpoint setup;
- control-session handshake;
- generated frame timeline;
- DATAGRAM delivery;
- run summaries.

Exit gate:

- client and server complete a media run without recovery.

### M2 — Visible JPEG transport: complete

Goal:

- replace synthetic payload-only traffic with visible generated JPEG frames.

Delivered:

- JPEG test-pattern generation;
- DATAGRAM fragmentation and reassembly;
- complete-frame validation;
- receiver preview.

Exit gate:

- receiver displays and validates generated frames.

### M3 — Controlled migration: complete

Goal:

- demonstrate continuity when the endpoint is explicitly rebound.

Delivered:

- dual-path Mininet topology;
- Path A and Path B addressing;
- controlled `Endpoint::rebind()`;
- same connection and session after rebind;
- continued media delivery.

Exit gate:

- controlled Path A to Path B migration completes without creating another
  application session.

### M4 — Automatic migration: complete

Goal:

- detect a sustained loss of progress and trigger migration automatically.

Delivered:

- `Healthy`, `Suspect`, `Challenging`, and migration states;
- ACK-progress health signal;
- deterministic alternate-address discovery;
- automatic rebind;
- resumed-progress confirmation.

Exit gate:

- sustained Path A failure triggers automatic migration and the run completes
  through Path B.

### M5 — Comparative evaluation: active

Goal:

- compare migration with proactive reconnect under the same detector and
  failure scenario;
- produce reproducible results and submission-ready evidence.

M5 is split into four epics.

## Epic status

### Epic 5.1 — Continuous media timeline: complete

Delivered:

- transport-independent `MediaRun`;
- global frame IDs derived from run time;
- `media_run_id` and `session_id` lifecycle;
- completion through the active session;
- migration preserves run/session identity;
- reconnect preserves the run but replaces the session.

Acceptance:

- reconnect does not restart the timeline from frame zero.

### Epic 5.2 — Proactive reconnect baseline: complete

Delivered:

- shared health detector and `Challenging` decision point;
- configurable recovery strategy;
- replacement endpoint, connection, and session;
- second HELLO for the same media run;
- resumption from the live timeline;
- reconnect-specific verification.

Acceptance:

- migration uses one connection/session;
- reconnect uses two distinct connections/sessions;
- both complete the same logical media run.

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
- 63 focused Python tests;
- committed 10-migration and 10-reconnect dataset.

Committed evidence:

```text
results/recovery-experiment-01/
├── README.md
├── summary.csv
└── runs/
    ├── migrate-001.json
    ├── ...
    └── reconnect-010.json
```

Acceptance:

- all 20 committed runs complete;
- all results are analyzable;
- no analysis errors;
- migration and reconnect identity expectations are met;
- summary CSV is reproducible from per-run results.

### Epic 5.4 — Comparative results and final demo: active

Purpose:

- turn the verified Epic 5.3 dataset into the final engineering conclusion and
  submission package.

Remaining deliverables:

- aggregate statistics;
- report-ready comparison tables;
- plots with individual observations;
- interpretation of receiver gap versus frame preservation;
- limitations and threats to validity;
- verified migration and reconnect demonstration clips;
- advisor-facing walkthrough;
- final report updates;
- clean-repository and clean-checkout validation;
- submission packaging.

## Current baseline experiment

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

Current descriptive result:

| Metric | Migration | Reconnect |
|---|---:|---:|
| Successful runs | 10/10 | 10/10 |
| Median largest receive gap | 937.4 ms | 802.0 ms |
| Mean largest receive gap | 973.3 ms | 830.2 ms |
| Median missing frames | 2 | 7 |
| Mean missing frames | 2.4 | 7.6 |
| Median recovery-action duration | 156 ms | 1 ms |

Working interpretation:

- reconnect restored receiver activity sooner;
- migration preserved more frames;
- migration retained one connection and session;
- reconnect created a replacement connection and session;
- both strategies completed every tested run.

This interpretation remains preliminary until Epic 5.4 completes the final
statistics, plots, limitations, and report discussion.

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
- plots;
- representative verified logs;
- migration and reconnect video clips;
- Git revision and experiment metadata.

### Documentation deliverables

- project README;
- current-status document;
- Mininet workflow guide;
- recovery-analysis guide;
- result dataset README;
- final report;
- advisor presentation outline.

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

## Dependencies

Epic 5.4 depends on:

- the committed Epic 5.3 dataset;
- stable result schema version 1;
- working noninteractive launcher;
- working lifecycle verifier;
- final report structure;
- access to a functioning Mininet environment for demonstration recording.

No further recovery implementation is currently required.

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

The project is complete when:

- both recovery strategies are implemented and verified;
- the continuous media timeline is demonstrated;
- repeated experiment results are reproducible;
- aggregate statistics and plots are generated;
- measured findings and limitations are documented;
- final demo evidence is verified;
- the advisor-facing walkthrough is prepared;
- the report answers the engineering question;
- all code, test, build, and documentation gates pass;
- the repository and submission package are clean and understandable.
