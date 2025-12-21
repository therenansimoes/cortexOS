# Creating GitHub Issues for CortexOS Development

This guide explains how to create and manage the development issues for CortexOS using GitHub's tasklist and sub-issue features.

## Overview

The CortexOS development plan consists of:
- **7 Development Phases** (40 weeks total)
- **40 Pull Requests** organized into phases
- **7 Major Milestones** (0.1 through 1.0)

We use **GitHub's tasklist feature** to create parent issues for each phase, with sub-issues for individual PRs. This provides excellent visibility and tracking.

## Quick Start

### Prerequisites

1. **GitHub CLI** installed:
   ```bash
   # macOS
   brew install gh
   
   # Linux
   sudo apt install gh  # Debian/Ubuntu
   # or download from https://cli.github.com/
   ```

2. **Authenticate with GitHub**:
   ```bash
   gh auth login
   ```

### Create All Parent Issues

Run the provided script to create parent tracking issues:

```bash
./tools/create-tasklist-issues.sh
```

This creates 8 parent issues:
1. Phase 1: Foundation Stabilization (Weeks 1-4)
2. Phase 2: Core Features (Weeks 5-10)
3. Phase 3: Cognitive Layer (Weeks 11-16)
4. Phase 4: Physical Layer (Weeks 17-20)
5. Phase 5: Intelligence (Weeks 21-26)
6. Phase 6: Advanced Features (Weeks 27-32)
7. Phase 7: Beta Release (Weeks 33-40)
8. Cross-Cutting Concerns (Throughout)

### View Created Issues

```bash
# List all tracking issues
gh issue list --label tracking

# View a specific issue
gh issue view <issue-number>
```

## Understanding GitHub Tasklists

### What are Tasklists?

GitHub's tasklist feature (formerly called "sub-issues") allows you to:
- Create checkbox items in an issue description
- Convert those checkboxes to tracked child issues
- See progress automatically calculated
- Maintain parent-child relationships

### How to Use Tasklists

1. **Parent issues are created** with checkboxes like:
   ```markdown
   - [ ] PR #2: Event System Enhancements
   - [ ] PR #3: Backpressure Testing
   ```

2. **Convert to tracked issues**:
   - Open the parent issue in GitHub web UI
   - Hover over a checkbox item
   - Click "Convert to issue" button
   - Fill in details and create

3. **Track progress**:
   - Parent issue shows completion percentage
   - Closing child issues automatically checks boxes
   - Visual progress indicator appears

## Detailed Workflow

### Phase 1: Getting Started

1. **Create parent issues** (already done by script):
   ```bash
   ./tools/create-tasklist-issues.sh
   ```

2. **Navigate to Phase 1 issue**:
   ```bash
   # Find the issue number
   gh issue list --label phase-1
   
   # View in browser
   gh issue view <issue-number> --web
   ```

3. **Convert sub-tasks to issues**:
   - In the web UI, find the checklist
   - Click "Convert to issue" next to each PR
   - Or manually create issues using the template

### Creating Individual PR Issues

You can also create individual PR issues manually:

```bash
gh issue create \
  --title "PR #2: Event System Enhancements" \
  --label "enhancement,milestone-0.1,priority-high,phase-1" \
  --template pr-task.md
```

Or use the existing Python script:

```bash
python3 tools/create-issues.py
```

### Linking Issues to Parent

When creating child issues manually, reference the parent:

```markdown
Part of #<parent-issue-number>

Closes when all acceptance criteria are met.
```

## Issue Organization

### Labels

Issues should use these labels:

**Phase Labels:**
- `phase-1`, `phase-2`, ..., `phase-7`
- `cross-cutting`

**Milestone Labels:**
- `milestone-0.1`, `milestone-0.2`, `milestone-0.2.1`, `milestone-0.3`
- `milestone-0.4`, `milestone-0.5`, `milestone-0.6`, `milestone-1.0`

**Priority Labels:**
- `priority-critical` (security, blockers)
- `priority-high` (core functionality)
- `priority-medium` (enhancements)
- `priority-low` (nice-to-have)

**Type Labels:**
- `enhancement` (new features)
- `bug` (fixes)
- `documentation` (docs)
- `security` (security-related)
- `tracking` (parent tracking issues)

### Example Issue Labels

```bash
# PR #2: Event System Enhancements
gh issue edit <issue-number> --add-label "enhancement,milestone-0.1,priority-high,phase-1"

# PR #7: Grid Handshake Security
gh issue edit <issue-number> --add-label "enhancement,milestone-0.2,priority-critical,phase-1,security"
```

## Working with Issues

### Starting Work on an Issue

1. **Find an available issue**:
   ```bash
   # High priority issues in Phase 1
   gh issue list --label "priority-high,phase-1" --state open
   ```

2. **Self-assign**:
   ```bash
   gh issue edit <issue-number> --add-assignee @me
   ```

3. **Create a branch**:
   ```bash
   # GitHub CLI can do this automatically
   gh issue develop <issue-number> --checkout
   
   # Or manually
   git checkout -b feature/pr-2-event-enhancements
   ```

4. **Update issue status**:
   Add a comment to track progress:
   ```bash
   gh issue comment <issue-number> --body "Started work on this issue"
   ```

### Completing an Issue

1. **Create PR**:
   ```bash
   gh pr create \
     --title "PR #2: Event System Enhancements" \
     --body "Closes #<issue-number>"
   ```

2. **Link PR to issue**:
   - Use `Closes #<issue-number>` in PR description
   - Issue automatically closes when PR merges

3. **Update parent issue**:
   - The checkbox in parent issue auto-updates when child closes
   - Or manually check it if not using tracked issues

## Best Practices

### Do's ✅

- **Use descriptive titles** that match the PR breakdown
- **Add all relevant labels** for easy filtering
- **Reference parent issues** in child issues
- **Keep issue descriptions updated** as work progresses
- **Close issues promptly** when PRs merge
- **Add comments** for significant updates or blockers

### Don'ts ❌

- **Don't create duplicate issues** - check existing first
- **Don't skip testing** - add test results to issues
- **Don't work on blocked issues** - check dependencies
- **Don't forget documentation** - update docs before closing
- **Don't lose context** - keep issue comments informative

## Tracking Progress

### View Overall Progress

```bash
# All open issues by phase
gh issue list --label phase-1 --state open
gh issue list --label phase-2 --state open

# Critical issues
gh issue list --label priority-critical --state open

# My assigned issues
gh issue list --assignee @me --state open
```

### Project Boards

Create a GitHub Project board for better visualization:

1. Go to repository → Projects → New Project
2. Choose "Board" template
3. Add columns: Todo, In Progress, In Review, Done
4. Add all issues to the board
5. Move issues as work progresses

### Milestones

Create GitHub Milestones for each major version:

```bash
# Example: Create Milestone 0.1
gh api repos/:owner/:repo/milestones \
  -f title="Milestone 0.1 - Portable Runtime" \
  -f description="Event system and WASI support" \
  -f due_on="2026-03-01T00:00:00Z"  # Adjust date as needed

# Assign issue to milestone
gh issue edit <issue-number> --milestone "Milestone 0.1"
```

## Troubleshooting

### GitHub CLI Not Working

```bash
# Check version
gh --version

# Update
brew upgrade gh  # macOS
sudo apt update && sudo apt upgrade gh  # Linux

# Re-authenticate
gh auth login
```

### Can't Convert to Tracked Issue

This feature requires:
- Repository admin or maintainer access
- GitHub.com (not GitHub Enterprise Server < 3.6)
- Issues enabled in repository settings

### Issues Not Appearing

Check filters:
```bash
# All issues (including closed)
gh issue list --state all

# With specific label
gh issue list --label tracking --state all
```

## Advanced Usage

### Bulk Operations

Close multiple issues:
```bash
# Close all completed Phase 1 issues
for issue in $(gh issue list --label phase-1 --state open --json number -q '.[].number'); do
  gh issue close $issue --comment "Completed as part of Phase 1"
done
```

### Custom Queries

```bash
# High priority issues not assigned
gh issue list --label priority-high --assignee ""

# Issues blocked by dependencies
gh issue list --search "blocked in:body"

# Security issues
gh issue list --label security --state open
```

### Export to CSV

```bash
# Export all issues
gh issue list --state all --json number,title,state,labels,assignees \
  > issues.json

# Convert to CSV (requires jq)
cat issues.json | jq -r '.[] | [.number, .title, .state] | @csv' \
  > issues.csv
```

## Resources

### Documentation Links

- **Planning Documents**:
  - [ROADMAP.md](./ROADMAP.md) - Visual 40-week overview
  - [PR_BREAKDOWN.md](./PR_BREAKDOWN.md) - Detailed PR specifications
  - [WORK_PLAN.md](./WORK_PLAN.md) - Phased work plan
  - [HOW_TO_CREATE_PRS.md](./HOW_TO_CREATE_PRS.md) - PR creation guide

- **Templates**:
  - [.github/ISSUE_TEMPLATE/pr-task.md](./.github/ISSUE_TEMPLATE/pr-task.md) - Issue template

- **Tools**:
  - [tools/create-tasklist-issues.sh](./tools/create-tasklist-issues.sh) - Create parent issues
  - [tools/create-issues.py](./tools/create-issues.py) - Bulk issue creation
  - [tools/create-pr-issues.sh](./tools/create-pr-issues.sh) - Interactive creation

### External Resources

- [GitHub CLI Manual](https://cli.github.com/manual/)
- [GitHub Tasklists Documentation](https://docs.github.com/en/issues/tracking-your-work-with-issues/about-tasklists)
- [GitHub Projects Guide](https://docs.github.com/en/issues/planning-and-tracking-with-projects)

## Next Steps

1. ✅ **Run the script**: `./tools/create-tasklist-issues.sh`
2. 📋 **View issues**: `gh issue list --label tracking`
3. 🌐 **Open in browser**: `gh issue view <issue-number> --web`
4. 🔄 **Convert tasklists**: Use web UI to convert checkboxes to tracked issues
5. 🚀 **Start Phase 1**: Begin with PR #2 or PR #7!

---

**Questions or issues?** Open a discussion or contact the maintainers.
