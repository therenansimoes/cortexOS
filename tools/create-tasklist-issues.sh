#!/bin/bash
# Script to create GitHub issues with sub-issues (tasklists) for CortexOS development
# Uses GitHub CLI (gh) to create tracked tasklists

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== CortexOS Issue Creation with Sub-Issues ===${NC}\n"

# Check if gh is installed
if ! command -v gh &> /dev/null; then
    echo -e "${YELLOW}Error: GitHub CLI (gh) is not installed${NC}"
    echo "Please install it from: https://cli.github.com/"
    exit 1
fi

# Check if authenticated
if ! gh auth status &> /dev/null; then
    echo -e "${YELLOW}Not authenticated with GitHub. Please run:${NC}"
    echo "  gh auth login"
    exit 1
fi

echo -e "${GREEN}✓ GitHub CLI authenticated${NC}\n"

# Function to create an issue with tasklist
create_parent_issue() {
    local title="$1"
    local body="$2"
    local labels="$3"
    
    echo -e "${BLUE}Creating issue: ${title}${NC}"
    if gh issue create \
        --title "$title" \
        --body "$body" \
        --label "$labels" 2>&1; then
        echo -e "${GREEN}✓ Created successfully${NC}"
    else
        echo -e "${YELLOW}⚠ Warning: Issue may not have been created${NC}"
        echo -e "${YELLOW}  Check GitHub web UI to verify${NC}"
    fi
    echo ""
}

# Phase 1: Foundation Stabilization
PHASE1_BODY="## Overview

This is the parent tracking issue for **Phase 1: Foundation Stabilization** (Weeks 1-4).

**Goal**: Ensure core systems are production-ready and secure before building advanced features.

## Success Criteria
- [ ] All critical security issues addressed
- [ ] Event system enhanced with production features
- [ ] WASI build validated and optimized
- [ ] CI/CD pipeline enhanced
- [ ] Error handling standardized
- [ ] Security audit completed

## Sub-Issues (PRs)

The following PRs are part of this phase. Check them off as they are completed:

- [ ] PR #2: Event System Enhancements
- [ ] PR #3: Backpressure Policy Testing
- [ ] PR #4: WASI Build Optimization
- [ ] PR #7: Grid Handshake Security ⚠️ CRITICAL
- [ ] PR #36: CI/CD Enhancement
- [ ] PR #37: Error Handling Standardization
- [ ] PR #39: Security Audit ⚠️ CRITICAL

## Timeline
**Duration**: Weeks 1-4
**Start**: TBD
**Target Completion**: TBD

## Priority Notes
- **Critical Path**: PR #7 (Grid Handshake Security) should be started early
- **Parallelizable**: PRs #2, #3, #4 can run simultaneously
- **Security Focus**: PRs #7 and #39 are security-critical

## Resources
- See [WORK_PLAN.md](../../../WORK_PLAN.md#phase-1-foundation-stabilization-weeks-1-4) for detailed plan
- See [PR_BREAKDOWN.md](../../../PR_BREAKDOWN.md) for PR specifications
- See [ROADMAP.md](../../../ROADMAP.md) for visual overview

## Definition of Done
- [ ] All sub-issues completed and merged
- [ ] Milestone 0.1 features stable
- [ ] Security baseline established
- [ ] All tests passing
- [ ] Documentation updated"

# Phase 2: Core Features
PHASE2_BODY="## Overview

This is the parent tracking issue for **Phase 2: Core Features** (Weeks 5-10).

**Goal**: Complete Milestone 0.1 and Milestone 0.2 features for Grid Bootstrap and Relay Mesh.

## Success Criteria
- [ ] Milestone 0.1 (Portable Runtime) completed
- [ ] Milestone 0.2 (Grid Bootstrap) completed
- [ ] Milestone 0.2.1 (Relay Mesh) completed
- [ ] Event throughput > 100K events/sec
- [ ] Grid handshake latency < 100ms
- [ ] Relay delivery success rate > 95%

## Sub-Issues (PRs)

### Milestone 0.1 Completion
- [ ] PR #5: Runtime Improvements

### Milestone 0.2 Completion
- [ ] PR #6: Grid Discovery Enhancements
- [ ] PR #8: Grid Wire Protocol Extensions
- [ ] PR #9: Task Delegation System
- [ ] PR #10: Event Chunk Sync

### Milestone 0.2.1 Completion
- [ ] PR #11: Relay Mesh Security Hardening ⚠️
- [ ] PR #12: DHT Integration Testing
- [ ] PR #13: Relay Mesh Optimization

## Timeline
**Duration**: Weeks 5-10
**Dependencies**: Phase 1 must be completed
**Target Completion**: TBD

## Priority Notes
- **Dependencies**: PR #5 depends on PR #2; PR #8 depends on PR #7
- **Parallelizable**: PRs #6 and #11 can run in parallel during Week 6
- **Security**: PR #11 is security-critical

## Resources
- See [WORK_PLAN.md](../../WORK_PLAN.md#phase-2-core-feature-completion-weeks-5-10)
- See [PR_BREAKDOWN.md](../../PR_BREAKDOWN.md#milestone-02---grid-bootstrap)
- See [ROADMAP.md](../../ROADMAP.md)

## Definition of Done
- [ ] All sub-issues completed and merged
- [ ] All milestone metrics met
- [ ] Integration tests passing
- [ ] Documentation complete"

# Phase 3: Cognitive Layer
PHASE3_BODY="## Overview

This is the parent tracking issue for **Phase 3: Cognitive Layer** (Weeks 11-16).

**Goal**: Complete Milestone 0.3 (Thought Graph) with persistent memory and MindLang.

## Success Criteria
- [ ] Milestone 0.3 (Thought Graph) completed
- [ ] Graph query latency < 10ms (p95)
- [ ] Privacy audit passed
- [ ] Graph supports 1M+ nodes
- [ ] MindLang parser and VM functional

## Sub-Issues (PRs)

### Graph Store & Queries
- [ ] PR #14: Graph Store Backend Selection
- [ ] PR #15: Graph Query Engine
- [ ] PR #16: Privacy Controls Implementation ⚠️

### MindLang Implementation
- [ ] PR #17: MindLang Parser Implementation
- [ ] PR #18: MindLang VM Implementation
- [ ] PR #19: MindLang-Graph Integration

## Timeline
**Duration**: Weeks 11-16
**Dependencies**: Phase 2 must be completed
**Target Completion**: TBD

## Priority Notes
- **Critical**: PR #16 (Privacy Controls) is critical for user trust
- **Parallelizable**: PRs #14-15 and #17-18 can run in parallel
- **Dependencies**: PR #19 depends on both PR #15 and PR #18

## Resources
- See [WORK_PLAN.md](../../WORK_PLAN.md#phase-3-cognitive-layer-weeks-11-16)
- See [PR_BREAKDOWN.md](../../PR_BREAKDOWN.md#milestone-03---thought-graph)
- See [ROADMAP.md](../../ROADMAP.md)

## Definition of Done
- [ ] All sub-issues completed and merged
- [ ] Privacy controls validated
- [ ] Performance benchmarks met
- [ ] MindLang examples working"

# Phase 4: Physical Layer
PHASE4_BODY="## Overview

This is the parent tracking issue for **Phase 4: Physical Layer** (Weeks 17-20).

**Goal**: Complete Milestone 0.4 (Subnet Framing) with LED and audio signal implementations.

## Success Criteria
- [ ] Milestone 0.4 (Subnet Framing) completed
- [ ] Signal reliability > 90% at 1m range
- [ ] Frame error rate < 1%
- [ ] Multi-device communication validated (3+ devices)

## Sub-Issues (PRs)

- [ ] PR #20: Signal Framing Protocol
- [ ] PR #21: LED Emitter/Receiver Implementation
- [ ] PR #22: Audio Emitter/Receiver Implementation
- [ ] PR #23: Multi-Device Signal Testing

## Timeline
**Duration**: Weeks 17-20
**Dependencies**: Phase 3 must be completed
**Target Completion**: TBD

## Priority Notes
- **Hardware Required**: PRs #21-23 require physical devices for testing
- **Parallelizable**: PRs #21 and #22 can run in parallel
- **Dependencies**: PRs #21-23 all depend on PR #20

## Resources
- See [WORK_PLAN.md](../../WORK_PLAN.md#phase-4-physical-layer-weeks-17-20)
- See [PR_BREAKDOWN.md](../../PR_BREAKDOWN.md#milestone-04---subnet-framing)
- See [ROADMAP.md](../../ROADMAP.md)

## Definition of Done
- [ ] All sub-issues completed and merged
- [ ] Hardware tests documented
- [ ] Signal reliability validated
- [ ] Multi-device demos working"

# Phase 5: Intelligence
PHASE5_BODY="## Overview

This is the parent tracking issue for **Phase 5: Intelligence** (Weeks 21-26).

**Goal**: Complete Milestone 0.5 (Compiler & Planner Agents) with LLM integration.

## Success Criteria
- [ ] Milestone 0.5 (Compiler & Planner) completed
- [ ] Code generation quality score > 80%
- [ ] Compilation success rate > 90%
- [ ] Planning accuracy > 85%
- [ ] Distributed compilation demo working

## Sub-Issues (PRs)

- [ ] PR #24: LLM Model Integration
- [ ] PR #25: Compiler Agent Implementation
- [ ] PR #26: Planner Agent Implementation
- [ ] PR #27: Distributed Compilation Demo

## Timeline
**Duration**: Weeks 21-26
**Dependencies**: Phases 1-4 must be completed
**Target Completion**: TBD

## Priority Notes
- **Model Selection**: PR #24 requires careful evaluation of LLM options
- **Parallelizable**: PRs #25 and #26 can run in parallel
- **Showcase**: PR #27 demonstrates distributed capabilities

## Resources
- See [WORK_PLAN.md](../../WORK_PLAN.md#phase-5-intelligence-layer-weeks-21-26)
- See [PR_BREAKDOWN.md](../../PR_BREAKDOWN.md#milestone-05---compiler--planner-agents)
- See [ROADMAP.md](../../ROADMAP.md)

## Definition of Done
- [ ] All sub-issues completed and merged
- [ ] Agent quality metrics met
- [ ] Distributed demo documented
- [ ] Model licensing verified"

# Phase 6: Advanced Features
PHASE6_BODY="## Overview

This is the parent tracking issue for **Phase 6: Advanced Features** (Weeks 27-32).

**Goal**: Complete Milestone 0.6 (Evolution) with BLE signals and adaptive protocols.

## Success Criteria
- [ ] Milestone 0.6 (Evolution) completed
- [ ] BLE signal implementation working
- [ ] Signal evolution framework functional
- [ ] Multi-hop communication validated

## Sub-Issues (PRs)

- [ ] PR #28: BLE Signal Implementation
- [ ] PR #29: Signal Evolution Framework
- [ ] PR #30: Multi-Hop Communication

## Timeline
**Duration**: Weeks 27-32
**Dependencies**: Phases 1-5 must be completed
**Target Completion**: TBD

## Priority Notes
- **Optional Features**: These are advanced features, can be deprioritized if needed
- **Hardware**: BLE testing requires compatible devices
- **Dependencies**: PR #30 depends on PRs #28 and #29

## Resources
- See [WORK_PLAN.md](../../WORK_PLAN.md#phase-6-advanced-features-weeks-27-32)
- See [PR_BREAKDOWN.md](../../PR_BREAKDOWN.md#milestone-06---subnet-adaptation--evolution)
- See [ROADMAP.md](../../ROADMAP.md)

## Definition of Done
- [ ] All sub-issues completed and merged
- [ ] BLE communication validated
- [ ] Evolution metrics documented
- [ ] Multi-hop tests passing"

# Phase 7: Beta Release
PHASE7_BODY="## Overview

This is the parent tracking issue for **Phase 7: Beta Release** (Weeks 33-40).

**Goal**: Complete Milestone 1.0 with stable APIs, comprehensive docs, and production readiness.

## Success Criteria
- [ ] Milestone 1.0 (Production Ready) completed
- [ ] API stability: 100% documented
- [ ] Platform coverage: 3+ platforms
- [ ] Example count: 5+ complete examples
- [ ] Installation time: < 5 minutes
- [ ] Test coverage: > 80%

## Sub-Issues (PRs)

- [ ] PR #31: API Stabilization
- [ ] PR #32: Documentation Overhaul
- [ ] PR #33: Example Gallery
- [ ] PR #34: Installation & Deployment
- [ ] PR #35: Performance Benchmarking Suite

## Timeline
**Duration**: Weeks 33-40
**Dependencies**: All previous phases must be completed
**Target Completion**: TBD (Beta Release Date)

## Priority Notes
- **Critical Path**: PR #31 must be completed before others
- **Parallelizable**: PRs #32 and #33 can run in parallel
- **High Priority**: All PRs in this phase are high priority

## Resources
- See [WORK_PLAN.md](../../WORK_PLAN.md#phase-7-beta-release-preparation-weeks-33-40)
- See [PR_BREAKDOWN.md](../../PR_BREAKDOWN.md#milestone-10---beta-release)
- See [ROADMAP.md](../../ROADMAP.md)

## Definition of Done
- [ ] All sub-issues completed and merged
- [ ] Beta release tagged and published
- [ ] All documentation complete
- [ ] Installation tested on all platforms
- [ ] Performance benchmarks published
- [ ] Community ready for contributions"

# Cross-Cutting Concerns
CROSSCUTTING_BODY="## Overview

This is the parent tracking issue for **Cross-Cutting Concerns** (Throughout all phases).

**Goal**: Maintain quality, security, and platform support throughout development.

## Success Criteria
- [ ] CI/CD pipeline robust and automated
- [ ] Error handling consistent across codebase
- [ ] Logging and observability comprehensive
- [ ] Security vulnerabilities addressed
- [ ] Platform-specific optimizations completed

## Sub-Issues (PRs)

These PRs can be worked on incrementally alongside milestone work:

- [ ] PR #36: CI/CD Enhancement (Week 1-2)
- [ ] PR #37: Error Handling Standardization (Week 2-3)
- [ ] PR #38: Logging & Observability (Ongoing)
- [ ] PR #39: Security Audit ⚠️ CRITICAL (Week 3-4)
- [ ] PR #40: Platform Optimizations (Ongoing)

## Timeline
**Duration**: Throughout all 40 weeks
**Can Start**: Immediately (no dependencies)
**Ongoing**: These are continuous improvements

## Priority Notes
- **Critical**: PR #39 (Security Audit) must be completed in Phase 1
- **Foundation**: PR #36 (CI/CD) should be prioritized early
- **Continuous**: PRs #38 and #40 are ongoing improvements

## Resources
- See [WORK_PLAN.md](../../WORK_PLAN.md#continuous-work-throughout-all-phases)
- See [PR_BREAKDOWN.md](../../PR_BREAKDOWN.md#cross-cutting-prs-can-be-done-anytime)
- See [ROADMAP.md](../../ROADMAP.md)

## Definition of Done
- [ ] All sub-issues completed and merged
- [ ] Quality standards maintained
- [ ] Security baseline established
- [ ] Platform support validated"

# Create all parent issues
echo -e "${BLUE}Creating parent issues with sub-tasks...\n${NC}"

create_parent_issue \
    "Phase 1: Foundation Stabilization (Weeks 1-4)" \
    "$PHASE1_BODY" \
    "phase-1,milestone-0.1,enhancement,tracking"

create_parent_issue \
    "Phase 2: Core Features (Weeks 5-10)" \
    "$PHASE2_BODY" \
    "phase-2,milestone-0.2,enhancement,tracking"

create_parent_issue \
    "Phase 3: Cognitive Layer (Weeks 11-16)" \
    "$PHASE3_BODY" \
    "phase-3,milestone-0.3,enhancement,tracking"

create_parent_issue \
    "Phase 4: Physical Layer (Weeks 17-20)" \
    "$PHASE4_BODY" \
    "phase-4,milestone-0.4,enhancement,tracking"

create_parent_issue \
    "Phase 5: Intelligence (Weeks 21-26)" \
    "$PHASE5_BODY" \
    "phase-5,milestone-0.5,enhancement,tracking"

create_parent_issue \
    "Phase 6: Advanced Features (Weeks 27-32)" \
    "$PHASE6_BODY" \
    "phase-6,milestone-0.6,enhancement,tracking"

create_parent_issue \
    "Phase 7: Beta Release (Weeks 33-40)" \
    "$PHASE7_BODY" \
    "phase-7,milestone-1.0,enhancement,tracking"

create_parent_issue \
    "Cross-Cutting Concerns (Throughout)" \
    "$CROSSCUTTING_BODY" \
    "cross-cutting,enhancement,tracking"

echo -e "${GREEN}✓ All parent issues created!${NC}\n"
echo -e "${BLUE}Next Steps:${NC}"
echo "1. View created issues: ${YELLOW}gh issue list --label tracking${NC}"
echo "2. Convert task items to tracked issues using the GitHub web UI"
echo "3. Start working on Phase 1 PRs!"
echo ""
echo -e "${BLUE}Note:${NC} GitHub's tasklist feature requires converting checkboxes to"
echo "tracked issues through the web UI. Visit each parent issue and click"
echo "'Convert to issue' next to each checkbox item."
