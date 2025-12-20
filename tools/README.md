# CortexOS Issue Creation Tools

This directory contains tools to help create GitHub issues for the 40 planned PRs in the CortexOS development roadmap.

## Available Tools

### 1. Python Script (Recommended)

**File**: `create-issues.py`

The most comprehensive tool that creates all 40 issues programmatically using the GitHub API.

#### Requirements
```bash
pip install PyGithub
```

#### Setup
```bash
# Create a GitHub Personal Access Token
# Go to: https://github.com/settings/tokens
# Generate token with 'repo' scope

# Set the token
export GITHUB_TOKEN=your_token_here
```

#### Usage
```bash
# Create all issues
python3 create-issues.py

# You'll be prompted to confirm before creating
```

**Pros**:
- Creates all issues in one command
- Properly formatted with all metadata
- Progress tracking
- Error handling

**Cons**:
- Requires Python and PyGithub library
- Need to set up GitHub token

### 2. JSON Data File

**File**: `pr-definitions.json`

A comprehensive JSON file containing all 40 PR definitions that can be used with custom scripts or GitHub CLI.

#### Structure
```json
{
  "repository": "therenansimoes/cortexOS",
  "prs": [
    {
      "number": 2,
      "title": "Event System Enhancements",
      "milestone": "0.1",
      "priority": "High",
      "size": "Small",
      "duration": "1 week",
      "description": "...",
      "dependencies": [],
      "tasks": [...],
      "labels": [...]
    },
    ...
  ]
}
```

#### Usage with `jq` and GitHub CLI
```bash
# Example: Create issue for PR #2
cat pr-definitions.json | jq '.prs[0]' | \
  gh issue create \
    --title "$(jq -r '.title')" \
    --body "$(jq -r '.description')" \
    --label "$(jq -r '.labels | join(",")')"
```

### 3. Bash Script

**File**: `create-pr-issues.sh`

Interactive bash script using GitHub CLI to create issues by phase.

#### Requirements
```bash
# Install GitHub CLI
brew install gh              # macOS
sudo apt install gh          # Ubuntu/Debian

# Authenticate
gh auth login
```

#### Usage
```bash
# Make executable
chmod +x create-pr-issues.sh

# Run
./create-pr-issues.sh

# Select which phase to create (1-8)
```

**Pros**:
- Interactive selection by phase
- No Python required
- Uses official GitHub CLI

**Cons**:
- Currently only implements Phase 1
- Need to extend for all phases

## Quick Start Guide

### Option A: Create All 40 Issues (Python)

```bash
# 1. Install dependencies
pip install PyGithub

# 2. Set up token
export GITHUB_TOKEN=your_github_token

# 3. Run script
cd tools
python3 create-issues.py

# 4. Confirm when prompted
```

### Option B: Create Issues by Phase (Bash)

```bash
# 1. Install GitHub CLI
brew install gh  # or apt install gh

# 2. Authenticate
gh auth login

# 3. Run script
cd tools
./create-pr-issues.sh

# 4. Select phase (e.g., 1 for Phase 1)
```

### Option C: Manual Creation Using JSON

```bash
# View PR definitions
cat tools/pr-definitions.json | jq '.prs[] | select(.number == 2)'

# Use GitHub web interface or CLI to create manually
```

## Development Phases

The 40 PRs are organized into 7 phases:

1. **Phase 1** (Weeks 1-4): Foundation Stabilization
   - PRs: #2, #3, #4, #7, #36, #37, #39
   - Focus: Security, Stability, Portability

2. **Phase 2** (Weeks 5-10): Core Features
   - PRs: #5, #6, #8, #9, #10, #11, #12, #13
   - Focus: Grid Bootstrap, Relay Mesh

3. **Phase 3** (Weeks 11-16): Cognitive Layer
   - PRs: #14, #15, #16, #17, #18, #19
   - Focus: Thought Graph, MindLang

4. **Phase 4** (Weeks 17-20): Physical Layer
   - PRs: #20, #21, #22, #23
   - Focus: Signal Framing, Hardware

5. **Phase 5** (Weeks 21-26): Intelligence
   - PRs: #24, #25, #26, #27
   - Focus: AI Agents, LLM Integration

6. **Phase 6** (Weeks 27-32): Advanced Features
   - PRs: #28, #29, #30
   - Focus: Signal Evolution, Multi-hop

7. **Phase 7** (Weeks 33-40): Beta Release
   - PRs: #31, #32, #33, #34, #35
   - Focus: Documentation, Examples, Polish

**Cross-Cutting**: PRs #36, #37, #38, #39, #40 (ongoing)

## Issue Structure

Each created issue will have:

### Metadata
- **Title**: "PR #X: [Title]"
- **Milestone**: e.g., "0.1", "0.2", "1.0", "Cross-Cutting"
- **Priority**: Critical, High, Medium, Low
- **Size**: Small (< 1 week), Medium (1-2 weeks), Large (2-4 weeks)
- **Duration**: Estimated time

### Content
- **Description**: What the PR aims to achieve
- **Dependencies**: Which PRs must be completed first
- **Tasks**: Detailed checklist of implementation steps
- **Acceptance Criteria**: Definition of done
- **References**: Links to planning documents

### Labels
Examples:
- `enhancement`, `testing`, `documentation`, `security`
- `milestone-0.1`, `milestone-0.2`, etc.
- `priority-high`, `priority-critical`, etc.
- `hardware`, `ai`, `demo`, etc.

## Verification

After creating issues, verify:

```bash
# List all issues
gh issue list

# Count issues (should be 40)
gh issue list --json number --jq length

# View specific issue
gh issue view <number>

# Filter by label
gh issue list --label "milestone-0.1"
gh issue list --label "priority-critical"
```

## Troubleshooting

### Python Script Issues

**Error: PyGithub not installed**
```bash
pip install PyGithub
```

**Error: GITHUB_TOKEN not set**
```bash
export GITHUB_TOKEN=your_token_here
```

**Error: Bad credentials**
- Verify token has `repo` scope
- Generate new token at https://github.com/settings/tokens

### Bash Script Issues

**Error: gh not found**
```bash
# Install GitHub CLI
brew install gh  # macOS
sudo apt install gh  # Linux
```

**Error: Not authenticated**
```bash
gh auth login
```

## Customization

### Modify PR Definitions

1. Edit `pr-definitions.json` for JSON-based tools
2. Edit `create-issues.py` for Python script
3. Edit `create-pr-issues.sh` for bash script

### Add Custom Labels

In `create-issues.py`, modify the `labels` field for each PR:
```python
labels=["enhancement", "milestone-0.1", "priority-high", "custom-label"]
```

### Change Repository

In `pr-definitions.json`:
```json
{
  "repository": "your-username/your-repo",
  ...
}
```

In `create-issues.py`:
```python
repo = g.get_repo("your-username/your-repo")
```

## Integration with Project Management

### GitHub Projects

After creating issues:

```bash
# Create a project board
gh project create --title "CortexOS Development" --body "40-week development plan"

# Add issues to project (requires project ID)
gh project item-add <project-id> --url https://github.com/therenansimoes/cortexOS/issues/<number>
```

### Milestones

Create milestones for better tracking:

```bash
gh api repos/therenansimoes/cortexOS/milestones \
  -f title="Milestone 0.1" \
  -f description="Portable Runtime + Event Model"

# Assign issues to milestone
gh issue edit <number> --milestone "Milestone 0.1"
```

## References

- [PR_BREAKDOWN.md](../PR_BREAKDOWN.md) - Complete PR details
- [WORK_PLAN.md](../WORK_PLAN.md) - Timeline and resources
- [ROADMAP.md](../ROADMAP.md) - Visual roadmap
- [HOW_TO_CREATE_PRS.md](../HOW_TO_CREATE_PRS.md) - Step-by-step guide

## Contributing

To improve these tools:

1. Test with your GitHub account first
2. Submit improvements via PR
3. Update documentation
4. Add error handling for edge cases

## Support

For issues with these tools:
1. Check this README
2. Review the planning documents
3. Open a GitHub issue with the `tools` label
