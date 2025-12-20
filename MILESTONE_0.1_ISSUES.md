# Milestone 0.1 Issues - Ready to Create

This document contains the 4 issues for Milestone 0.1 (Portable Runtime + Event Model), ready to be created in GitHub.

---

## Issue 1: PR #2 - Event System Enhancements

**Title:** `PR #2: Event System Enhancements`

**Labels:** `enhancement`, `milestone-0.1`, `priority-high`

**Body:**
```markdown
## Overview

**Milestone**: 0.1
**Priority**: High
**Estimated Size**: Small
**Duration**: 1 week

## Description

Enhance event system with production-ready features including validation, trace propagation, metrics, and improved error handling.

## Dependencies

None

## Tasks

- [ ] Add event validation and sanitization
- [ ] Implement trace context propagation
- [ ] Add metrics collection for event throughput
- [ ] Improve error handling in event bus
- [ ] Add benchmarks for event processing

## Acceptance Criteria

- [ ] Implementation complete
- [ ] Unit tests added and passing
- [ ] Integration tests passing
- [ ] Documentation updated
- [ ] CI checks passing
- [ ] Code review completed
- [ ] WASI build passes

## References

- See [PR_BREAKDOWN.md](https://github.com/therenansimoes/cortexOS/blob/main/PR_BREAKDOWN.md) for complete details
- See [WORK_PLAN.md](https://github.com/therenansimoes/cortexOS/blob/main/WORK_PLAN.md) for schedule
```

---

## Issue 2: PR #3 - Backpressure Policy Testing & Documentation

**Title:** `PR #3: Backpressure Policy Testing & Documentation`

**Labels:** `testing`, `documentation`, `milestone-0.1`, `priority-high`

**Body:**
```markdown
## Overview

**Milestone**: 0.1
**Priority**: High
**Estimated Size**: Small
**Duration**: 1-2 weeks

## Description

Comprehensive testing and documentation for all backpressure policies with performance benchmarks.

## Dependencies

None

## Tasks

- [ ] Add unit tests for each policy
- [ ] Add integration tests for policy behavior under load
- [ ] Document policy selection guidelines
- [ ] Add examples for each policy type
- [ ] Performance benchmarks

## Acceptance Criteria

- [ ] Implementation complete
- [ ] Unit tests added and passing
- [ ] Integration tests passing
- [ ] Documentation updated
- [ ] CI checks passing
- [ ] Code review completed
- [ ] Performance validated

## References

- See [PR_BREAKDOWN.md](https://github.com/therenansimoes/cortexOS/blob/main/PR_BREAKDOWN.md) for complete details
- See [WORK_PLAN.md](https://github.com/therenansimoes/cortexOS/blob/main/WORK_PLAN.md) for schedule
```

---

## Issue 3: PR #4 - WASI Build Optimization

**Title:** `PR #4: WASI Build Optimization`

**Labels:** `portability`, `wasm`, `milestone-0.1`, `priority-high`

**Body:**
```markdown
## Overview

**Milestone**: 0.1
**Priority**: High
**Estimated Size**: Medium
**Duration**: 2 weeks

## Description

Ensure WASI target builds efficiently with optimized binary size, CI checks, and comprehensive documentation.

## Dependencies

None

## Tasks

- [ ] Fix any WASI compilation issues
- [ ] Optimize binary size for WASM
- [ ] Add CI check for WASI builds
- [ ] Document WASI limitations
- [ ] Create WASM example

## Acceptance Criteria

- [ ] Implementation complete
- [ ] Unit tests added and passing
- [ ] Integration tests passing
- [ ] Documentation updated
- [ ] CI checks passing
- [ ] Code review completed
- [ ] WASI build passes
- [ ] Binary size target met (<1MB)

## References

- See [PR_BREAKDOWN.md](https://github.com/therenansimoes/cortexOS/blob/main/PR_BREAKDOWN.md) for complete details
- See [WORK_PLAN.md](https://github.com/therenansimoes/cortexOS/blob/main/WORK_PLAN.md) for schedule
```

---

## Issue 4: PR #5 - Runtime Improvements

**Title:** `PR #5: Runtime Improvements`

**Labels:** `enhancement`, `milestone-0.1`, `priority-high`

**Body:**
```markdown
## Overview

**Milestone**: 0.1
**Priority**: High
**Estimated Size**: Medium
**Duration**: 1 week

## Description

Production-ready runtime features including graceful shutdown, statistics, health checks, and configuration.

## Dependencies

**Blocked by**: PR #2

## Tasks

- [ ] Add graceful shutdown
- [ ] Implement runtime statistics
- [ ] Add agent registry with health checks
- [ ] Improve task scheduling
- [ ] Add runtime configuration

## Acceptance Criteria

- [ ] Implementation complete
- [ ] Unit tests added and passing
- [ ] Integration tests passing
- [ ] Documentation updated
- [ ] CI checks passing
- [ ] Code review completed
- [ ] WASI build passes

## References

- See [PR_BREAKDOWN.md](https://github.com/therenansimoes/cortexOS/blob/main/PR_BREAKDOWN.md) for complete details
- See [WORK_PLAN.md](https://github.com/therenansimoes/cortexOS/blob/main/WORK_PLAN.md) for schedule
```

---

## How to Create These Issues

### Option 1: GitHub Web UI

1. Go to https://github.com/therenansimoes/cortexOS/issues/new
2. Copy the Title and Body from each issue above
3. Add the specified Labels
4. Click "Submit new issue"
5. Repeat for all 4 issues

### Option 2: GitHub CLI (if authenticated)

```bash
# Issue 1
gh issue create --title "PR #2: Event System Enhancements" \
  --label "enhancement,milestone-0.1,priority-high" \
  --body-file <(cat << 'EOF'
[paste Issue 1 body here]
EOF
)

# Issue 2
gh issue create --title "PR #3: Backpressure Policy Testing & Documentation" \
  --label "testing,documentation,milestone-0.1,priority-high" \
  --body-file <(cat << 'EOF'
[paste Issue 2 body here]
EOF
)

# Issue 3
gh issue create --title "PR #4: WASI Build Optimization" \
  --label "portability,wasm,milestone-0.1,priority-high" \
  --body-file <(cat << 'EOF'
[paste Issue 3 body here]
EOF
)

# Issue 4
gh issue create --title "PR #5: Runtime Improvements" \
  --label "enhancement,milestone-0.1,priority-high" \
  --body-file <(cat << 'EOF'
[paste Issue 4 body here]
EOF
)
```

---

## Milestone 0.1 Success Metrics

Once all 4 PRs are complete, Milestone 0.1 should achieve:

- ✅ Event throughput > 100K events/sec
- ✅ WASI binary size < 1MB
- ✅ Test coverage > 80%
- ✅ Portable runtime working on native + WASI
- ✅ Production-ready event handling with backpressure
