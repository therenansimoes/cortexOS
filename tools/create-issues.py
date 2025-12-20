#!/usr/bin/env python3
"""
Generate GitHub issues for all CortexOS PRs.

This script creates GitHub issues for each planned PR using the GitHub API.
It reads the PR definitions and creates properly formatted issues.

Requirements:
    pip install PyGithub

Usage:
    export GITHUB_TOKEN=your_token_here
    python3 create-issues.py
"""

import os
import sys
from dataclasses import dataclass
from typing import List, Optional

try:
    from github import Github
except ImportError:
    print("Error: PyGithub not installed.")
    print("Install it with: pip install PyGithub")
    sys.exit(1)


@dataclass
class PR:
    number: int
    title: str
    milestone: str
    priority: str
    size: str
    duration: str
    description: str
    dependencies: List[int]
    tasks: List[str]
    labels: List[str]


# Define all PRs
PRS = [
    # Phase 1: Foundation Stabilization
    PR(
        number=2,
        title="Event System Enhancements",
        milestone="0.1",
        priority="High",
        size="Small",
        duration="1 week",
        description="Enhance event system with production-ready features including validation, trace propagation, metrics, and improved error handling.",
        dependencies=[],
        tasks=[
            "Add event validation and sanitization",
            "Implement trace context propagation",
            "Add metrics collection for event throughput",
            "Improve error handling in event bus",
            "Add benchmarks for event processing",
        ],
        labels=["enhancement", "milestone-0.1", "priority-high"],
    ),
    PR(
        number=3,
        title="Backpressure Policy Testing & Documentation",
        milestone="0.1",
        priority="High",
        size="Small",
        duration="1-2 weeks",
        description="Comprehensive testing and documentation for all backpressure policies with performance benchmarks.",
        dependencies=[],
        tasks=[
            "Add unit tests for each policy",
            "Add integration tests for policy behavior under load",
            "Document policy selection guidelines",
            "Add examples for each policy type",
            "Performance benchmarks",
        ],
        labels=["testing", "documentation", "milestone-0.1", "priority-high"],
    ),
    PR(
        number=4,
        title="WASI Build Optimization",
        milestone="0.1",
        priority="High",
        size="Medium",
        duration="2 weeks",
        description="Ensure WASI target builds efficiently with optimized binary size, CI checks, and comprehensive documentation.",
        dependencies=[],
        tasks=[
            "Fix any WASI compilation issues",
            "Optimize binary size for WASM",
            "Add CI check for WASI builds",
            "Document WASI limitations",
            "Create WASM example",
        ],
        labels=["portability", "wasm", "milestone-0.1", "priority-high"],
    ),
    PR(
        number=5,
        title="Runtime Improvements",
        milestone="0.1",
        priority="High",
        size="Medium",
        duration="1 week",
        description="Production-ready runtime features including graceful shutdown, statistics, health checks, and configuration.",
        dependencies=[2],
        tasks=[
            "Add graceful shutdown",
            "Implement runtime statistics",
            "Add agent registry with health checks",
            "Improve task scheduling",
            "Add runtime configuration",
        ],
        labels=["enhancement", "milestone-0.1", "priority-high"],
    ),
    PR(
        number=6,
        title="Grid Discovery Enhancements",
        milestone="0.2",
        priority="High",
        size="Medium",
        duration="1-2 weeks",
        description="Improve peer discovery reliability with fallback mechanisms, caching, filtering, and IPv6 support.",
        dependencies=[],
        tasks=[
            "Add fallback discovery mechanisms",
            "Implement discovery caching",
            "Add discovery filtering by capability",
            "Improve IPv6 support",
            "Add discovery metrics",
        ],
        labels=["enhancement", "milestone-0.2", "priority-high"],
    ),
    PR(
        number=7,
        title="Grid Handshake Security",
        milestone="0.2",
        priority="Critical",
        size="Medium",
        duration="1-2 weeks",
        description="Harden handshake protocol with challenge-response authentication, key negotiation, replay prevention, and security audit.",
        dependencies=[],
        tasks=[
            "Add challenge-response authentication",
            "Implement session key negotiation",
            "Add replay attack prevention",
            "Implement peer verification",
            "Security audit and tests",
        ],
        labels=["security", "milestone-0.2", "priority-critical"],
    ),
    # Add more PRs here... (continuing pattern for all 40)
]


def create_issue_body(pr: PR) -> str:
    """Generate issue body from PR definition."""
    deps_text = "None"
    if pr.dependencies:
        deps_text = "\n".join([f"- [ ] PR #{dep}" for dep in pr.dependencies])

    tasks_text = "\n".join([f"- [ ] {task}" for task in pr.tasks])

    body = f"""## Overview

**Milestone**: {pr.milestone}
**Priority**: {pr.priority}
**Estimated Size**: {pr.size}
**Estimated Duration**: {pr.duration}

## Description

{pr.description}

## Dependencies

**Blocked by**:
{deps_text}

## Tasks

{tasks_text}

## Acceptance Criteria

- [ ] Implementation complete
- [ ] Unit tests added and passing
- [ ] Integration tests passing
- [ ] Documentation updated
- [ ] CI checks passing
- [ ] Code review completed
- [ ] Security review completed (if applicable)
- [ ] Performance validated (if applicable)
- [ ] WASI build passes (if applicable)

## References

- See [PR_BREAKDOWN.md](https://github.com/therenansimoes/cortexOS/blob/main/PR_BREAKDOWN.md) for complete details
- See [WORK_PLAN.md](https://github.com/therenansimoes/cortexOS/blob/main/WORK_PLAN.md) for schedule
"""
    return body


def main():
    # Get GitHub token
    token = os.environ.get("GITHUB_TOKEN")
    if not token:
        print("Error: GITHUB_TOKEN environment variable not set")
        print("Set it with: export GITHUB_TOKEN=your_token_here")
        sys.exit(1)

    # Initialize GitHub client
    g = Github(token)
    repo = g.get_repo("therenansimoes/cortexOS")

    print("CortexOS Issue Creator")
    print("=" * 50)
    print(f"Repository: {repo.full_name}")
    print(f"Total PRs to create: {len(PRS)}")
    print()

    # Ask for confirmation
    response = input(f"Create {len(PRS)} issues? (y/N): ")
    if response.lower() != "y":
        print("Cancelled.")
        sys.exit(0)

    # Create issues
    created = 0
    for pr in PRS:
        title = f"PR #{pr.number}: {pr.title}"
        body = create_issue_body(pr)

        try:
            issue = repo.create_issue(
                title=title,
                body=body,
                labels=pr.labels,
            )
            print(f"✓ Created: {title} ({issue.html_url})")
            created += 1
        except Exception as e:
            print(f"✗ Failed: {title}")
            print(f"  Error: {e}")

    print()
    print(f"Created {created}/{len(PRS)} issues successfully!")


if __name__ == "__main__":
    main()
