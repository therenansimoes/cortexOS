# Quick Start: Creating GitHub Issues for CortexOS

This guide will help you quickly create all 40 GitHub issues for the CortexOS development plan.

## TL;DR

```bash
# Preview what will be created (no GitHub account needed)
python3 tools/create-issues.py --dry-run

# Create Phase 1 issues (recommended start)
export GITHUB_TOKEN=your_token_here
python3 tools/create-issues.py --phase 1

# Create all 40 issues
python3 tools/create-issues.py
```

## Step-by-Step Guide

### Step 1: Preview the Issues (No Setup Required)

```bash
# See what all 40 issues look like
python3 tools/create-issues.py --dry-run

# Preview just Phase 1 (Foundation Stabilization)
python3 tools/create-issues.py --dry-run --phase 1
```

This shows you:
- Issue titles and numbers
- Milestones and priorities
- Dependencies between issues
- Number of tasks per issue
- Labels that will be applied

**No installation or GitHub token needed for preview!**

### Step 2: Set Up GitHub Token

#### Option A: Create a Token

1. Go to https://github.com/settings/tokens
2. Click "Generate new token (classic)"
3. Give it a name like "CortexOS Issues"
4. Select scope: `repo` (full control of private repositories)
5. Click "Generate token"
6. Copy the token (you won't see it again!)

#### Option B: Use GitHub CLI

```bash
# If you have gh installed and authenticated
export GITHUB_TOKEN=$(gh auth token)
```

### Step 3: Set the Token

```bash
export GITHUB_TOKEN=ghp_your_token_here
```

**Security Note**: Don't commit this token to git! It's only stored in your terminal session.

### Step 4: Create Issues

#### Recommended: Start with Phase 1

```bash
# Create 7 issues for Phase 1 (Foundation Stabilization)
python3 tools/create-issues.py --phase 1
```

This creates:
- PR #2: Event System Enhancements
- PR #3: Backpressure Policy Testing
- PR #4: WASI Build Optimization
- PR #7: Grid Handshake Security
- PR #36: CI/CD Pipeline Enhancement
- PR #37: Error Handling Standardization
- PR #39: Security Audit & Hardening

#### Create All Issues

```bash
# Create all 40 issues at once
python3 tools/create-issues.py
```

You'll be asked to confirm before creating.

### Step 5: Verify

```bash
# Using GitHub CLI
gh issue list

# Count issues (should see your new issues)
gh issue list --json number --jq length

# Filter by label
gh issue list --label "milestone-0.1"
gh issue list --label "priority-critical"
```

Or visit: https://github.com/therenansimoes/cortexOS/issues

## Phase-by-Phase Creation

You can create issues incrementally:

```bash
# Phase 1: Foundation (7 issues)
python3 tools/create-issues.py --phase 1

# Phase 2: Core Features (8 issues)
python3 tools/create-issues.py --phase 2

# Phase 3: Cognitive Layer (6 issues)
python3 tools/create-issues.py --phase 3

# And so on... (phases 4-7)

# Cross-cutting issues (5 issues)
python3 tools/create-issues.py --phase 8
```

## What Gets Created

Each issue includes:

✅ **Title**: "PR #X: [Descriptive Name]"
✅ **Milestone**: Which development milestone (0.1, 0.2, etc.)
✅ **Priority**: Critical, High, Medium, or Low
✅ **Size**: Small (< 1 week), Medium (1-2 weeks), Large (2-4 weeks)
✅ **Description**: Clear explanation of the PR's purpose
✅ **Dependencies**: Which other PRs must be completed first
✅ **Tasks**: Detailed checklist of implementation steps
✅ **Acceptance Criteria**: Definition of done
✅ **Labels**: For filtering and organization
✅ **References**: Links to planning documents

## Troubleshooting

### "PyGithub not installed"

Only needed if actually creating issues (not for --dry-run):

```bash
pip install PyGithub
```

### "GITHUB_TOKEN not set"

```bash
# Set it first
export GITHUB_TOKEN=your_token_here

# Then try again
python3 tools/create-issues.py
```

### "Bad credentials"

Your token might be invalid or expired:
1. Generate a new token at https://github.com/settings/tokens
2. Make sure it has `repo` scope
3. Export it again

### Issues already exist

The script will fail if issues with the same title already exist. This is intentional to prevent duplicates.

## Alternative Methods

### Method 1: Using JSON + GitHub CLI

```bash
# Install jq for JSON parsing
brew install jq  # macOS
sudo apt install jq  # Linux

# Create an issue from JSON
cat tools/pr-definitions.json | jq '.prs[0]' > /tmp/pr.json

# Parse and create with gh
gh issue create \
  --title "$(cat /tmp/pr.json | jq -r '.title')" \
  --body "$(cat /tmp/pr.json | jq -r '.description')"
```

### Method 2: Manual Creation

1. Go to https://github.com/therenansimoes/cortexOS/issues/new
2. Use the template from `.github/ISSUE_TEMPLATE/pr-task.md`
3. Fill in details from `PR_BREAKDOWN.md`

## Next Steps After Creation

1. **Organize**: Create GitHub milestones and assign issues
   ```bash
   gh api repos/therenansimoes/cortexOS/milestones \
     -f title="Milestone 0.1" \
     -f description="Portable Runtime + Event Model"
   ```

2. **Assign**: Add team members to issues
   ```bash
   gh issue edit <number> --add-assignee username
   ```

3. **Track**: Create a project board
   ```bash
   gh project create --title "CortexOS Development"
   ```

4. **Start**: Begin with Phase 1 issues (#2, #3, #4, #7)

## Reference Documents

- **[ISSUES_SUMMARY.md](./ISSUES_SUMMARY.md)**: What will be generated
- **[tools/README.md](./tools/README.md)**: Detailed tool documentation
- **[PR_BREAKDOWN.md](./PR_BREAKDOWN.md)**: All 40 PRs described
- **[WORK_PLAN.md](./WORK_PLAN.md)**: Timeline and resources
- **[ROADMAP.md](./ROADMAP.md)**: Visual roadmap

## Summary

```bash
# 1. Preview (no setup needed)
python3 tools/create-issues.py --dry-run

# 2. Set token
export GITHUB_TOKEN=your_token

# 3. Create Phase 1 (recommended)
python3 tools/create-issues.py --phase 1

# 4. Verify
gh issue list

# 5. Start coding!
```

**Questions?** See [tools/README.md](./tools/README.md) for detailed documentation.
