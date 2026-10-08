# F11 — symbol lookup indexing decision (2026-10-08)

The baseline at `25e178026b2b84794b34d335cca2cdfeda94c4d9` used up to
three table-order scans of the 476-row symbol catalog. The adopted candidate
uses one lazily initialized pair of hash maps while preserving exact-match,
bare-command, and first-alias precedence. The exhaustive catalog oracle test
compares the indexed result against the original selection rule.

## Paired measurement

Reported by the project developer on Windows x86-64 with Cargo's optimized
benchmark profile; `benches/layout.rs`, 31 samples per workload, median
nanoseconds per operation. CPU model and memory measurements were not captured.

| Workload | Linear | Indexed | Observation |
| --- | ---: | ---: | --- |
| Lookup early (`\alpha`) | 12.8 | 30.9 | 18.1 ns slower |
| Lookup late (`\backepsilon`) | 177.4 | 36.8 | 4.8× faster |
| Bare alias (`aleph`) | 956.6 | 25.5 | 37.5× faster |
| Composite (`\aleph_0`) | 51.4 | 32.4 | 1.6× faster |
| Missing command | 4058.9 | 34.4 | 118× faster |
| Symbol-heavy parse | 5353.5 | 3133.9 | 41.5% less time |
| Layout simple (pre-parsed) | 3204.3 | 3247.7 | 1.4% slower in this run |
| Layout complex (pre-parsed) | 44902.3 | 45394.5 | 1.1% slower in this run |
| Layout stress (pre-parsed) | 495403.1 | 492506.2 | 0.6% faster in this run |

## Decision and limits

Retain the indexed lookup because the parser-intensive workload and late/alias/
miss lookups improve substantially, without changes to lookup semantics. Accept
the measured absolute early-hit penalty of 18.1 ns as a current tradeoff, not
as a claim that every lookup becomes faster. The existing core benchmark suite
retains the distinguishing workloads as a permanent performance contract.

The measurements do not establish the CPU-memory overhead, cold-start cost,
production mix of early/late queries, or a statistically paired regression for
pre-parsed layout workloads. Reopen the choice if those quantities are measured
as material, or if typical complete parse workloads regress on supported targets.
No production dependency, `unsafe`, or additional public API was introduced.
