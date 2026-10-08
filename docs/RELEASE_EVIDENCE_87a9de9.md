# Mathematical-core release-candidate evidence — 87a9de9

This is a historical verification record for the *existing* candidate commit. It
is not a release tag, publication authorization, or evidence for any later SHA.

Candidate commit: `87a9de99562acbe9557f4a43a12eb81c02ba19f8`
Parent commit: `790365dd3e4e9eaae9c0d0c99ea535242240b957`

| Required GitHub workflow | Run | Result | Artifact |
| --- | --- | --- | --- |
| Core CI | [37734754576](https://github.com/baselogic/texpose/actions/runs/37734754576) | `success` | `dependency-trees` |
| Canonical oracle | [37734814143](https://github.com/baselogic/texpose/actions/runs/37734814143) | `success` | `canonical-oracle-evidence` |
| Stress oracle | [37734826850](https://github.com/baselogic/texpose/actions/runs/37734826850) | `success` | `stress-oracle-evidence` |

The three workflow runs identify the candidate SHA above. The GitHub API
reported all three artifacts present and not expired at the time of review
(2026-10-08). The workflow conclusions establish successful remote gates;
this record does not claim independent inspection of the contents of the
artifact ZIPs or the final local working-tree state.

The local worktree previously reported a pre-existing unstaged `.gitignore`
modification after candidate commit. That modification was not part of the
candidate, and must be reviewed or otherwise dealt with by its owner before
claiming a clean final worktree under `docs/RELEASE.md`.

This document records M1 remote-workflow evidence and M2 documentary closure
for the cited commit only. Applying this patch creates a *new* repository tree;
the historical workflow results must not be reattributed to that new tree.
The publication boundary defined in `docs/RELEASE.md` remains unchanged.
