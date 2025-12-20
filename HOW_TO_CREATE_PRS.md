# How to Create Multiple PRs for CortexOS Development

This guide explains how to use the development plan to create many smaller, focused PRs.

## What We've Created

This planning effort has produced:

1. **[PR_BREAKDOWN.md](./PR_BREAKDOWN.md)** - Complete breakdown of 40 PRs with:
   - Detailed descriptions
   - Dependencies between PRs
   - Size estimates (Small/Medium/Large)
   - Priority levels (Critical/High/Medium/Low)
   - Organized by milestone (0.1 through 1.0)

2. **[WORK_PLAN.md](./WORK_PLAN.md)** - Actionable work plan with:
   - 7 development phases
   - Week-by-week timeline
   - Resource requirements
   - Success metrics
   - Risk management

3. **[PLAN_README.md](./PLAN_README.md)** - Quick start guide

4. **Automation Tools**:
   - `.github/ISSUE_TEMPLATE/pr-task.md` - Template for creating issues
   - `tools/create-pr-issues.sh` - Bash script to create issues
   - `tools/create-issues.py` - Python script for bulk issue creation

## Quick Start: 3 Ways to Create PRs

### Option 1: Manual (Good for learning)

1. Read [PR_BREAKDOWN.md](./PR_BREAKDOWN.md) to understand the first PR
2. Create a GitHub issue using the template in `.github/ISSUE_TEMPLATE/pr-task.md`
3. Create a branch: `git checkout -b feature/pr-2-event-enhancements`
4. Implement the changes
5. Create a PR referencing the issue

### Option 2: Using GitHub CLI (Recommended)

```bash
# Install GitHub CLI
brew install gh  # macOS
# or: sudo apt install gh  # Linux

# Authenticate
gh auth login

# Create issues for Phase 1 (PRs 2-7, 36-37, 39)
./tools/create-pr-issues.sh
# Select option 1 when prompted

# View created issues
gh issue list

# Start working on an issue
gh issue develop <issue-number> --checkout
```

### Option 3: Bulk Creation with Python

```bash
# Install PyGithub
pip install PyGithub

# Set your GitHub token
export GITHUB_TOKEN=your_personal_access_token

# Create all issues at once (you'll be prompted to confirm)
python3 tools/create-issues.py
```

## Recommended Workflow

### Week 1: Get Started

1. **Review the plan**
   - Read [PLAN_README.md](./PLAN_README.md)
   - Review [PR_BREAKDOWN.md](./PR_BREAKDOWN.md) for Phase 1

2. **Create Phase 1 issues**
   ```bash
   ./tools/create-pr-issues.sh
   # Choose option 1 (Phase 1)
   ```

3. **Start with PR #2 or PR #7**
   - PR #2: Event System Enhancements (no dependencies)
   - PR #7: Grid Handshake Security (critical security work)

### Ongoing: Development Cycle

For each PR:

1. **Pick an issue**
   ```bash
   gh issue list --label "priority-high"
   ```

2. **Create a branch**
   ```bash
   git checkout -b feature/pr-X-short-description
   ```

3. **Implement**
   - Follow the tasks in the issue
   - Write tests as you go
   - Update documentation

4. **Validate**
   ```bash
   cargo test
   cargo clippy
   cargo build --target wasm32-wasip1  # If applicable
   ```

5. **Create PR**
   ```bash
   gh pr create --fill
   # Reference the issue: "Closes #123"
   ```

6. **Review & Merge**
   - Get code review
   - Address feedback
   - Merge when approved

7. **Move to next PR**

## Understanding the Plan

### PR Numbering

PRs are numbered 2-40 (PR #1 was the initial implementation):

- **#2-5**: Milestone 0.1 (Portable Runtime + Event Model)
- **#6-13**: Milestone 0.2 & 0.2.1 (Grid & Relay Mesh)
- **#14-19**: Milestone 0.3 (Thought Graph)
- **#20-23**: Milestone 0.4 (Subnet Framing)
- **#24-27**: Milestone 0.5 (Compiler & Planner Agents)
- **#28-30**: Milestone 0.6 (Subnet Adaptation)
- **#31-35**: Milestone 1.0 (Beta Release)
- **#36-40**: Cross-cutting concerns (CI/CD, security, etc.)

### Dependencies

Some PRs must be completed before others:

```
Critical Path:
PR #2 → PR #5 → ...
PR #7 → PR #8 → PR #9, #10

Parallel Work:
PR #2, #3, #4 can all run simultaneously
PR #6, #7 can run in parallel
```

See the dependency graph in [PR_BREAKDOWN.md](./PR_BREAKDOWN.md).

### Priority Levels

- **Critical**: Security, blockers (do first)
- **High**: Core functionality (do early)
- **Medium**: Enhancements (do when ready)
- **Low**: Nice-to-have (do last)

## Example: Working on PR #2

Here's a complete example of tackling the first PR:

```bash
# 1. Create the issue (if not already created)
gh issue create --title "PR #2: Event System Enhancements" \
  --label "enhancement,milestone-0.1,priority-high" \
  --body "See PR_BREAKDOWN.md for details"

# 2. Create branch
git checkout -b feature/pr-2-event-enhancements

# 3. Implement the tasks
# - Add event validation and sanitization
# - Implement trace context propagation
# - Add metrics collection
# - Improve error handling
# - Add benchmarks

# Edit files in crates/core/src/event.rs, etc.

# 4. Test
cargo test -p cortex-core
cargo clippy --all-targets
cargo bench  # Run benchmarks

# 5. Document
# Update crates/core/README.md or docs/

# 6. Create PR
git add .
git commit -m "feat(core): enhance event system with validation and metrics"
git push origin feature/pr-2-event-enhancements

gh pr create \
  --title "PR #2: Event System Enhancements" \
  --body "Closes #<issue-number>

## Summary
Enhanced the event system with production-ready features:
- Event validation and sanitization
- Trace context propagation
- Metrics collection
- Improved error handling
- Performance benchmarks

## Test Results
- All tests passing
- Benchmarks show 10% improvement
- WASI build successful

## Documentation
- Updated API docs
- Added usage examples
"

# 7. Review and merge
# Wait for CI, code review, then merge
```

## Tracking Progress

### Using GitHub Projects

Create a project board:

1. Go to your repository
2. Projects → New Project
3. Choose "Board" template
4. Add columns: To Do, In Progress, In Review, Done
5. Add all issues to the board

### Milestones

Create milestones for each phase:

```bash
gh api repos/therenansimoes/cortexOS/milestones -f title="Milestone 0.1" -f description="Portable Runtime + Event Model"
gh api repos/therenansimoes/cortexOS/milestones -f title="Milestone 0.2" -f description="Grid Bootstrap"
# etc.
```

Then assign issues to milestones:
```bash
gh issue edit <issue-number> --milestone "Milestone 0.1"
```

## Tips for Success

1. **Start small**: Don't try to do everything at once
2. **Follow dependencies**: Check PR_BREAKDOWN.md before starting
3. **Test thoroughly**: Every PR needs tests
4. **Document well**: Code without docs is incomplete
5. **Review the plan**: Re-read WORK_PLAN.md periodically
6. **Ask questions**: Open a discussion if unclear
7. **Stay focused**: Each PR should do one thing well

## Common Questions

### Q: Can I work on multiple PRs at once?
A: Yes, if they don't have dependencies. Check PR_BREAKDOWN.md for the dependency graph.

### Q: What if I find issues while working?
A: Create new issues for bugs. The plan is flexible.

### Q: How strict is the order?
A: Dependencies are strict. Otherwise, use priority as a guide.

### Q: Can I combine PRs?
A: Only if it makes sense. Generally, keep them separate for easier review.

### Q: What if a PR is too large?
A: Split it into multiple smaller PRs. Update the plan accordingly.

## Next Steps

1. ✅ **Right now**: Read [PLAN_README.md](./PLAN_README.md)
2. 📖 **Today**: Review [PR_BREAKDOWN.md](./PR_BREAKDOWN.md) Phase 1
3. 🎯 **This week**: Create issues for Phase 1
4. 💻 **This month**: Complete PR #2, #3, and #7
5. 🚀 **This quarter**: Complete Milestone 0.1 and 0.2

## Resources

- **Planning Docs**:
  - [PR_BREAKDOWN.md](./PR_BREAKDOWN.md) - All PR details
  - [WORK_PLAN.md](./WORK_PLAN.md) - Timeline and resources
  - [PLAN_README.md](./PLAN_README.md) - Quick overview

- **Project Docs**:
  - [README.md](./README.md) - CortexOS blueprint
  - [AGENTS.md](./AGENTS.md) - Development guidelines

- **Tools**:
  - `.github/ISSUE_TEMPLATE/pr-task.md` - Issue template
  - `tools/create-pr-issues.sh` - Bash script
  - `tools/create-issues.py` - Python script

- **External**:
  - [GitHub CLI Docs](https://cli.github.com/manual/)
  - [PyGithub Docs](https://pygithub.readthedocs.io/)

---

**Ready to start?** Begin with Phase 1, PR #2: Event System Enhancements! 🚀
