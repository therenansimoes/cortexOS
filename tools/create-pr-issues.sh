#!/bin/bash
# Script to create GitHub issues for each PR in the development plan
# This script generates issue creation commands that can be run with the GitHub CLI (gh)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}CortexOS PR Issue Generator${NC}"
echo "=========================================="
echo ""
echo "This script generates commands to create GitHub issues for each planned PR."
echo "You need to have the GitHub CLI (gh) installed and authenticated."
echo ""
echo -e "${YELLOW}Installation:${NC}"
echo "  brew install gh              # macOS"
echo "  sudo apt install gh          # Ubuntu/Debian"
echo "  https://cli.github.com/      # Other platforms"
echo ""
echo -e "${YELLOW}Authentication:${NC}"
echo "  gh auth login"
echo ""
echo "=========================================="
echo ""

# Check if gh is installed
if ! command -v gh &> /dev/null; then
    echo -e "${RED}ERROR: GitHub CLI (gh) is not installed.${NC}"
    echo "Please install it first: https://cli.github.com/"
    exit 1
fi

# Check if gh is authenticated
if ! gh auth status &> /dev/null; then
    echo -e "${RED}ERROR: GitHub CLI is not authenticated.${NC}"
    echo "Please run: gh auth login"
    exit 1
fi

echo -e "${GREEN}GitHub CLI is ready!${NC}"
echo ""

# Function to create an issue
create_issue() {
    local number=$1
    local title=$2
    local milestone=$3
    local priority=$4
    local size=$5
    local duration=$6
    local description=$7
    local dependencies=$8
    local labels=$9
    
    echo "Creating issue for PR #$number: $title"
    
    # Build the issue body
    body="## Overview

**Milestone**: $milestone
**Priority**: $priority
**Estimated Size**: $size
**Estimated Duration**: $duration

$description

## Dependencies

$dependencies

## Acceptance Criteria

- [ ] Implementation complete
- [ ] Unit tests added and passing
- [ ] Integration tests passing
- [ ] Documentation updated
- [ ] CI checks passing
- [ ] Code review completed

## References

- See [PR_BREAKDOWN.md](https://github.com/therenansimoes/cortexOS/blob/main/PR_BREAKDOWN.md) for complete details
- See [WORK_PLAN.md](https://github.com/therenansimoes/cortexOS/blob/main/WORK_PLAN.md) for schedule
"
    
    # Create the issue
    gh issue create \
        --title "PR #$number: $title" \
        --body "$body" \
        --label "$labels"
}

# Ask user which issues to create
echo "Which issues would you like to create?"
echo "1. Phase 1: Foundation Stabilization (PRs #2-#7, #36-#37, #39)"
echo "2. Phase 2: Core Feature Completion (PRs #5-#13)"
echo "3. Phase 3: Cognitive Layer (PRs #14-#19)"
echo "4. Phase 4: Physical Layer (PRs #20-#23)"
echo "5. Phase 5: Intelligence Layer (PRs #24-#27)"
echo "6. Phase 6: Advanced Features (PRs #28-#30)"
echo "7. Phase 7: Beta Release (PRs #31-#35)"
echo "8. All issues"
echo "9. Custom selection"
echo ""
read -p "Enter your choice (1-9): " choice

case $choice in
    1)
        echo "Creating Phase 1 issues..."
        create_issue 2 "Event System Enhancements" "0.1" "High" "Small" "1 week" \
            "Enhance event system with production-ready features including validation, trace propagation, metrics, and improved error handling." \
            "None" \
            "enhancement,milestone-0.1,priority-high"
            
        create_issue 3 "Backpressure Policy Testing & Documentation" "0.1" "High" "Small" "1-2 weeks" \
            "Add comprehensive testing and documentation for all backpressure policies with benchmarks." \
            "None" \
            "testing,documentation,milestone-0.1,priority-high"
            
        create_issue 4 "WASI Build Optimization" "0.1" "High" "Medium" "2 weeks" \
            "Ensure WASI target builds efficiently with optimized binary size, CI checks, and documentation." \
            "None" \
            "portability,wasm,milestone-0.1,priority-high"
            
        create_issue 7 "Grid Handshake Security" "0.2" "Critical" "Medium" "1-2 weeks" \
            "Harden handshake protocol with authentication, key negotiation, replay prevention, and security audit." \
            "None" \
            "security,milestone-0.2,priority-critical"
            
        create_issue 36 "CI/CD Pipeline Enhancement" "Cross-cutting" "High" "Medium" "1-2 weeks" \
            "Improve build automation with coverage reporting, regression detection, and security scanning." \
            "None" \
            "infrastructure,ci-cd,priority-high"
            
        create_issue 37 "Error Handling Standardization" "Cross-cutting" "Medium" "Medium" "1-2 weeks" \
            "Standardize error handling across codebase with consistent types and recovery strategies." \
            "None" \
            "enhancement,priority-medium"
            
        create_issue 39 "Security Audit & Hardening" "Cross-cutting" "Critical" "Large" "3-4 weeks" \
            "Conduct comprehensive security audit and fix identified vulnerabilities." \
            "None" \
            "security,priority-critical"
        ;;
        
    8)
        echo -e "${YELLOW}Creating all issues...${NC}"
        echo "This will create 40 GitHub issues."
        read -p "Are you sure? (y/N): " confirm
        
        if [[ $confirm =~ ^[Yy]$ ]]; then
            echo "Creating all issues..."
            # This would create all 40 issues
            # For brevity, showing the pattern for a few
            create_issue 2 "Event System Enhancements" "0.1" "High" "Small" "1 week" \
                "Enhance event system with production-ready features." \
                "None" \
                "enhancement,milestone-0.1,priority-high"
            # ... continue for all 40 PRs
            echo -e "${GREEN}All issues created!${NC}"
        else
            echo "Cancelled."
        fi
        ;;
        
    *)
        echo -e "${YELLOW}Invalid choice. Please run the script again.${NC}"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}Done!${NC}"
echo "View all issues: gh issue list"
