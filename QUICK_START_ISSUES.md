# Quick Start: Creating GitHub Issues for CortexOS

This guide helps you quickly create all the tracking issues for the CortexOS development plan.

## ⚡ Fastest Path (Recommended)

### Step 1: Install GitHub CLI

```bash
# macOS
brew install gh

# Linux (Debian/Ubuntu)
sudo apt install gh

# Or download from: https://cli.github.com/
```

### Step 2: Authenticate

```bash
gh auth login
```

Follow the prompts to authenticate with your GitHub account.

### Step 3: Create Issues

```bash
# Navigate to your CortexOS repository
cd ~/cortexOS  # or wherever you cloned the repository

# Run the script
./tools/create-tasklist-issues.sh
```

This will create **8 parent tracking issues**:
1. Phase 1: Foundation Stabilization (Weeks 1-4)
2. Phase 2: Core Features (Weeks 5-10)
3. Phase 3: Cognitive Layer (Weeks 11-16)
4. Phase 4: Physical Layer (Weeks 17-20)
5. Phase 5: Intelligence (Weeks 21-26)
6. Phase 6: Advanced Features (Weeks 27-32)
7. Phase 7: Beta Release (Weeks 33-40)
8. Cross-Cutting Concerns (Throughout)

### Step 4: Convert to Tracked Issues

1. View the created issues:
   ```bash
   gh issue list --label tracking
   ```

2. Open a parent issue in your browser:
   ```bash
   gh issue view <issue-number> --web
   ```

3. For each checkbox item in the issue:
   - Hover over the checkbox
   - Click "Convert to issue"
   - Fill in details (pre-filled from PR_BREAKDOWN.md)
   - Click "Create issue"

4. The parent issue will now show progress as you complete child issues!

### Step 5: Start Working

```bash
# See all Phase 1 issues
gh issue list --label phase-1

# Pick one and start working
gh issue develop <issue-number> --checkout

# Or manually create a branch
git checkout -b feature/pr-2-event-enhancements
```

## 📋 What Gets Created

Each parent issue includes:
- Overview and success criteria
- Complete checklist of PRs for that phase
- Timeline and dependencies
- Priority notes
- Links to detailed documentation
- Definition of done

Example structure:
```markdown
## Phase 1: Foundation Stabilization

## Sub-Issues (PRs)
- [ ] PR #2: Event System Enhancements
- [ ] PR #3: Backpressure Policy Testing
- [ ] PR #7: Grid Handshake Security ⚠️
...
```

## 🎯 Understanding the Workflow

### Parent Issues (Phases)
- Track overall progress of a development phase
- Show completion percentage
- Contain links to all related PRs
- Auto-update as child issues are closed

### Child Issues (Individual PRs)
- Created from parent issue checkboxes
- Link back to parent automatically
- Use the pr-task.md template
- Close when PR is merged

### Example Hierarchy
```
Phase 1: Foundation Stabilization (Weeks 1-4)
├── PR #2: Event System Enhancements
├── PR #3: Backpressure Policy Testing
├── PR #4: WASI Build Optimization
├── PR #7: Grid Handshake Security
├── PR #36: CI/CD Enhancement
├── PR #37: Error Handling
└── PR #39: Security Audit
```

## 🔍 Viewing Your Issues

```bash
# All tracking issues
gh issue list --label tracking

# All Phase 1 issues
gh issue list --label phase-1

# High priority issues
gh issue list --label priority-high

# My assigned issues
gh issue list --assignee @me

# Open in web browser
gh issue list --web
```

## 🛠️ Alternative Methods

### Option 2: Python Script (Bulk Creation)

If you prefer to create individual PR issues directly:

```bash
# Install PyGithub
pip install PyGithub

# Set token
export GITHUB_TOKEN=your_personal_access_token

# Run script
python3 tools/create-issues.py
```

### Option 3: Interactive Bash Script

```bash
# Run interactive script
./tools/create-pr-issues.sh

# Choose a phase when prompted
```

### Option 4: Manual Creation

Use the GitHub UI with the template:
1. Go to Issues → New Issue
2. Select "PR Task" template
3. Fill in details from PR_BREAKDOWN.md

## 📚 Next Steps

After creating issues:

1. **Review Phase 1** issues:
   ```bash
   gh issue list --label phase-1
   ```

2. **Start with high priority**:
   - PR #2: Event System Enhancements
   - PR #7: Grid Handshake Security (Critical)

3. **Follow the workflow** in [HOW_TO_CREATE_PRS.md](./HOW_TO_CREATE_PRS.md)

4. **Track progress** on parent issues

## 🆘 Troubleshooting

### "gh not found"
Install GitHub CLI: https://cli.github.com/

### "Not authenticated"
Run: `gh auth login`

### "Permission denied"
Make script executable: `chmod +x tools/create-tasklist-issues.sh`

### "Can't convert to tracked issue"
- Requires GitHub.com (not Enterprise Server < 3.6)
- Requires repository admin/maintainer access
- Check that Issues are enabled in repo settings

## 📖 More Information

- **Complete Guide**: [CREATING_ISSUES.md](./CREATING_ISSUES.md)
- **How to Create PRs**: [HOW_TO_CREATE_PRS.md](./HOW_TO_CREATE_PRS.md)
- **PR Details**: [PR_BREAKDOWN.md](./PR_BREAKDOWN.md)
- **Work Plan**: [WORK_PLAN.md](./WORK_PLAN.md)
- **Visual Roadmap**: [ROADMAP.md](./ROADMAP.md)

---

**Ready to start?** Run the script and begin building CortexOS! 🚀
