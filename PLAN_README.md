# CortexOS Development Plan Overview

This directory contains the comprehensive development plan for CortexOS, broken down into manageable PRs.

## Quick Links

- **[PR_BREAKDOWN.md](./PR_BREAKDOWN.md)** - Detailed breakdown of all 40 planned PRs with descriptions, dependencies, and priorities
- **[WORK_PLAN.md](./WORK_PLAN.md)** - Organized work plan with phases, timelines, and resource requirements
- **[tools/create-pr-issues.sh](./tools/create-pr-issues.sh)** - Script to create GitHub issues for PRs

## Overview

Based on the CortexOS blueprint (README.md), we've identified 40 PRs organized into 7 phases:

### Phase Breakdown

| Phase | Milestone | PRs | Duration | Focus |
|-------|-----------|-----|----------|-------|
| 1 | Foundation | #2-7, 36-37, 39 | 4 weeks | Core stabilization & security |
| 2 | Core Features | #5-13 | 6 weeks | Milestones 0.1 & 0.2 |
| 3 | Cognitive Layer | #14-19 | 6 weeks | Milestone 0.3 (Thought Graph) |
| 4 | Physical Layer | #20-23 | 4 weeks | Milestone 0.4 (Signals) |
| 5 | Intelligence | #24-27 | 6 weeks | Milestone 0.5 (AI Agents) |
| 6 | Advanced | #28-30 | 6 weeks | Milestone 0.6 (Evolution) |
| 7 | Beta Release | #31-35 | 8 weeks | Milestone 1.0 |

**Total Timeline**: ~40 weeks (assuming 1 FTE or equivalent)

## Getting Started

### 1. Review the Plan

Start by reading:
1. [PR_BREAKDOWN.md](./PR_BREAKDOWN.md) - Understand all PRs
2. [WORK_PLAN.md](./WORK_PLAN.md) - See the timeline and dependencies

### 2. Create GitHub Issues

Use the provided script to create issues:

```bash
# Install GitHub CLI if needed
brew install gh  # macOS
# or: sudo apt install gh  # Ubuntu/Debian

# Authenticate
gh auth login

# Create issues for Phase 1 (recommended starting point)
./tools/create-pr-issues.sh
# Select option 1 when prompted
```

Or create issues manually using the template in `.github/ISSUE_TEMPLATE/pr-task.md`

### 3. Start Development

**Recommended first PRs** (can be done in parallel):
- PR #2: Event System Enhancements (Week 1)
- PR #3: Backpressure Testing (Week 1-2)
- PR #7: Grid Handshake Security (Week 1-2)

See [WORK_PLAN.md](./WORK_PLAN.md#quick-start-guide-first-3-prs) for detailed first steps.

## Current Status

✅ **Completed**: Milestone 0.1-0.5 basic implementations (PR #1)
🔄 **In Progress**: Planning and organization
📋 **Next**: Foundation stabilization (Phase 1)

## Key Principles

When working on these PRs, remember:

1. **Small, focused changes** - Each PR should do one thing well
2. **Test everything** - No PR without tests
3. **Document as you go** - Code without docs is incomplete
4. **Security first** - Security PRs are marked as Critical
5. **Portability matters** - Keep WASI builds green
6. **No mocks** - Use real implementations (see AGENTS.md)

## PR Dependencies

Some PRs must be done in order. Key dependency chains:

```
PR #2 → PR #5 (Event system before runtime improvements)
PR #7 → PR #8 (Security before protocol extensions)
PR #8 → PR #9, #10 (Protocol before delegation/sync)
PR #14 → PR #15 → PR #16 (Storage before queries before privacy)
PR #17 → PR #18 → PR #19 (Parser before VM before integration)
```

See [PR_BREAKDOWN.md](./PR_BREAKDOWN.md) for complete dependency graph.

## Contributing

### Branch Naming
- Feature: `feature/pr-X-short-description`
- Bugfix: `fix/pr-X-short-description`
- Security: `security/pr-X-short-description`

### PR Template
Each PR should include:
- Link to the tracking issue
- Implementation summary
- Test coverage report
- Documentation updates
- Breaking changes (if any)

### Review Process
1. Self-review checklist
2. Automated CI checks
3. Peer code review
4. Security review (for Critical PRs)
5. Merge to main

## Tracking Progress

### GitHub Projects
Consider creating a GitHub Project board with columns:
- Backlog
- Ready
- In Progress
- In Review
- Done

### Milestones
Create GitHub milestones for each phase:
- Milestone 0.1: Foundation
- Milestone 0.2: Core Features
- Milestone 0.3: Cognitive Layer
- etc.

### Labels
Suggested labels:
- `milestone-X.Y` - Which milestone
- `priority-critical/high/medium/low` - Priority level
- `size-small/medium/large` - Effort estimate
- `security` - Security-related
- `portability` - WASM/WASI related
- `enhancement` - New feature
- `bug` - Bug fix
- `documentation` - Docs only

## Resource Requirements

### Development
- Rust toolchain (stable + nightly)
- WASI target: `rustup target add wasm32-wasip1`
- Test devices (for Phase 4+): LED, audio, BLE capable

### Infrastructure
- CI/CD capacity
- Storage for benchmarks
- Test Grid (3+ nodes)

See [WORK_PLAN.md](./WORK_PLAN.md#resource-requirements) for details.

## Success Metrics

We'll measure success by:
- **Velocity**: PRs completed per week
- **Quality**: Test coverage, bug rate
- **Performance**: Benchmarks vs targets
- **Portability**: All platforms building
- **Security**: Zero critical vulnerabilities

Target metrics per milestone in [WORK_PLAN.md](./WORK_PLAN.md#success-metrics).

## Questions?

- 📖 See [README.md](./README.md) for the full blueprint
- 🛠️ See [AGENTS.md](./AGENTS.md) for development guidelines
- 💬 Open a GitHub Discussion for questions
- 🐛 Open an Issue for bugs

## License

This plan is part of the CortexOS project.
See the main [LICENSE](./LICENSE) file for details.

---

**Last Updated**: 2025-12-20
**Plan Version**: 1.0
**Total PRs**: 40
**Estimated Completion**: ~40 weeks
