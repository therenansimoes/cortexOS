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

# Only import Github if not in dry-run mode
def import_github():
    try:
        from github import Github
        return Github
    except ImportError:
        return None


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
    # Phase 2: Grid Bootstrap (continued)
    PR(
        number=8,
        title="Grid Wire Protocol Extensions",
        milestone="0.2",
        priority="High",
        size="Large",
        duration="2-3 weeks",
        description="Complete wire protocol implementation with all message types, versioning, compression, and validation.",
        dependencies=[7],
        tasks=[
            "Implement all message types from spec",
            "Add protocol versioning support",
            "Implement message compression",
            "Add message validation",
            "Protocol documentation",
        ],
        labels=["enhancement", "milestone-0.2", "priority-high"],
    ),
    PR(
        number=9,
        title="Task Delegation System",
        milestone="0.2",
        priority="Medium",
        size="Large",
        duration="2-3 weeks",
        description="Implement cross-node task execution with queuing, timeout, retry logic, and result aggregation.",
        dependencies=[8],
        tasks=[
            "Design task request/response protocol",
            "Implement task queuing",
            "Add task timeout and retry logic",
            "Implement task result aggregation",
            "Add task execution metrics",
        ],
        labels=["enhancement", "milestone-0.2", "priority-medium"],
    ),
    PR(
        number=10,
        title="Event Chunk Sync",
        milestone="0.2",
        priority="Medium",
        size="Large",
        duration="2-3 weeks",
        description="Implement event-log synchronization with chunk transfer, verification, throttling, and delta sync.",
        dependencies=[8],
        tasks=[
            "Design chunk transfer protocol",
            "Implement chunk verification",
            "Add bandwidth throttling",
            "Implement delta sync",
            "Add sync progress tracking",
        ],
        labels=["enhancement", "milestone-0.2", "priority-medium"],
    ),
    # Milestone 0.2.1 - Relay Mesh
    PR(
        number=11,
        title="Relay Mesh Security Hardening",
        milestone="0.2.1",
        priority="Critical",
        size="Medium",
        duration="1-2 weeks",
        description="Security improvements for relay protocol including encryption audit, key rotation, and spam prevention.",
        dependencies=[],
        tasks=[
            "Audit encryption implementation",
            "Add key rotation mechanism",
            "Implement beacon rate limiting",
            "Add spam prevention",
            "Security documentation",
        ],
        labels=["security", "milestone-0.2.1", "priority-critical"],
    ),
    PR(
        number=12,
        title="DHT Integration Testing",
        milestone="0.2.1",
        priority="High",
        size="Medium",
        duration="1-2 weeks",
        description="Validate DHT bulletin board functionality with libp2p Kademlia integration tests and monitoring.",
        dependencies=[],
        tasks=[
            "Integration tests with libp2p Kademlia",
            "Test beacon delivery and retrieval",
            "Add DHT performance metrics",
            "Document DHT configuration",
            "Add DHT monitoring",
        ],
        labels=["testing", "milestone-0.2.1", "priority-high"],
    ),
    PR(
        number=13,
        title="Relay Mesh Optimization",
        milestone="0.2.1",
        priority="Medium",
        size="Medium",
        duration="1-2 weeks",
        description="Optimize relay performance with reduced beacon size, improved routing, deduplication, and adaptive TTL.",
        dependencies=[11],
        tasks=[
            "Reduce beacon size",
            "Optimize routing algorithm",
            "Add beacon deduplication",
            "Implement adaptive TTL",
            "Performance benchmarks",
        ],
        labels=["performance", "milestone-0.2.1", "priority-medium"],
    ),
    # Milestone 0.3 - Thought Graph
    PR(
        number=14,
        title="Graph Store Backend Selection",
        milestone="0.3",
        priority="High",
        size="Medium",
        duration="1-2 weeks",
        description="Evaluate and optimize storage backend with benchmarks, graph query optimization, and indexes.",
        dependencies=[],
        tasks=[
            "Benchmark RocksDB vs alternatives",
            "Optimize for graph queries",
            "Implement graph indexes",
            "Add storage metrics",
            "Document storage configuration",
        ],
        labels=["enhancement", "milestone-0.3", "priority-high"],
    ),
    PR(
        number=15,
        title="Graph Query Engine",
        milestone="0.3",
        priority="High",
        size="Large",
        duration="2-3 weeks",
        description="Implement graph traversal and queries with API design, algorithms, optimization, and caching.",
        dependencies=[14],
        tasks=[
            "Design query API",
            "Implement common graph algorithms",
            "Add query optimization",
            "Implement query caching",
            "Add query metrics",
        ],
        labels=["enhancement", "milestone-0.3", "priority-high"],
    ),
    PR(
        number=16,
        title="Privacy Controls Implementation",
        milestone="0.3",
        priority="Critical",
        size="Large",
        duration="2-3 weeks",
        description="Complete privacy system for graph data with access control, encryption, selective sharing, and audit logging.",
        dependencies=[15],
        tasks=[
            "Implement access control policies",
            "Add data encryption at rest",
            "Implement selective sharing",
            "Add audit logging",
            "Privacy documentation",
        ],
        labels=["security", "milestone-0.3", "priority-critical"],
    ),
    PR(
        number=17,
        title="MindLang Parser Implementation",
        milestone="0.3",
        priority="Medium",
        size="Large",
        duration="2-3 weeks",
        description="Complete MindLang language parser with grammar specification, syntax validation, and AST generation.",
        dependencies=[],
        tasks=[
            "Define grammar specification",
            "Implement parser",
            "Add syntax validation",
            "Implement AST generation",
            "Add parser tests",
        ],
        labels=["enhancement", "milestone-0.3", "priority-medium"],
    ),
    PR(
        number=18,
        title="MindLang VM Implementation",
        milestone="0.3",
        priority="Medium",
        size="Large",
        duration="2-3 weeks",
        description="Implement MindLang virtual machine with instruction set, execution engine, debugging, and security sandbox.",
        dependencies=[17],
        tasks=[
            "Design VM instruction set",
            "Implement VM execution engine",
            "Add debugging support",
            "Implement VM security sandbox",
            "VM performance benchmarks",
        ],
        labels=["enhancement", "milestone-0.3", "priority-medium"],
    ),
    PR(
        number=19,
        title="MindLang-Graph Integration",
        milestone="0.3",
        priority="Medium",
        size="Medium",
        duration="1-2 weeks",
        description="Connect MindLang with Thought Graph including query syntax, update operations, and memory logging.",
        dependencies=[15, 18],
        tasks=[
            "Implement graph query syntax",
            "Add graph update operations",
            "Implement memory logging",
            "Add integration tests",
            "Documentation and examples",
        ],
        labels=["enhancement", "milestone-0.3", "priority-medium"],
    ),
    # Milestone 0.4 - Subnet Framing
    PR(
        number=20,
        title="Signal Framing Protocol",
        milestone="0.4",
        priority="Medium",
        size="Medium",
        duration="1-2 weeks",
        description="Implement low-level signal framing with frame structure, encoding/decoding, error correction, and ACK/NACK.",
        dependencies=[],
        tasks=[
            "Design frame structure (preamble, CRC, sequence)",
            "Implement frame encoding/decoding",
            "Add error correction",
            "Implement ACK/NACK protocol",
            "Frame validation tests",
        ],
        labels=["enhancement", "milestone-0.4", "priority-medium"],
    ),
    PR(
        number=21,
        title="LED Emitter/Receiver Implementation",
        milestone="0.4",
        priority="Medium",
        size="Medium",
        duration="1-2 weeks",
        description="First physical signal implementation with LED emission driver, light sensor receiver, and calibration.",
        dependencies=[20],
        tasks=[
            "Implement LED emission driver",
            "Implement light sensor receiver",
            "Add signal calibration",
            "Implement error handling",
            "Hardware testing guide",
        ],
        labels=["enhancement", "milestone-0.4", "priority-medium", "hardware"],
    ),
    PR(
        number=22,
        title="Audio Emitter/Receiver Implementation",
        milestone="0.4",
        priority="Medium",
        size="Medium",
        duration="1-2 weeks",
        description="Audio-based signal transmission with ultrasonic chirps, noise filtering, and signal encoding.",
        dependencies=[20],
        tasks=[
            "Implement audio emission (ultrasonic chirps)",
            "Implement audio receiver",
            "Add noise filtering",
            "Implement signal encoding",
            "Audio testing guide",
        ],
        labels=["enhancement", "milestone-0.4", "priority-medium", "hardware"],
    ),
    PR(
        number=23,
        title="Multi-Device Signal Testing",
        milestone="0.4",
        priority="Medium",
        size="Small",
        duration="< 1 week",
        description="Cross-device communication validation with test protocol, reliability measurements, and documentation.",
        dependencies=[21, 22],
        tasks=[
            "Create test protocol",
            "Test LED communication",
            "Test audio communication",
            "Measure reliability and range",
            "Document test results",
        ],
        labels=["testing", "milestone-0.4", "priority-medium", "hardware"],
    ),
    # Milestone 0.5 - Compiler & Planner Agents
    PR(
        number=24,
        title="LLM Model Integration",
        milestone="0.5",
        priority="Medium",
        size="Large",
        duration="2-3 weeks",
        description="Integrate local LLM for code generation with model evaluation, loading, optimization, and caching.",
        dependencies=[],
        tasks=[
            "Evaluate model options (Code LLaMA, etc.)",
            "Implement model loading",
            "Add inference optimization",
            "Implement model caching",
            "Performance benchmarks",
        ],
        labels=["enhancement", "milestone-0.5", "priority-medium", "ai"],
    ),
    PR(
        number=25,
        title="Compiler Agent Implementation",
        milestone="0.5",
        priority="Medium",
        size="Large",
        duration="2-3 weeks",
        description="Code generation agent with API design, code generation, validation, and compilation.",
        dependencies=[24],
        tasks=[
            "Design compiler agent API",
            "Implement code generation",
            "Add code validation",
            "Implement compilation",
            "Add tests and examples",
        ],
        labels=["enhancement", "milestone-0.5", "priority-medium", "ai"],
    ),
    PR(
        number=26,
        title="Planner Agent Implementation",
        milestone="0.5",
        priority="Medium",
        size="Large",
        duration="2-3 weeks",
        description="Task planning agent with goal decomposition, task scheduling, and agent coordination.",
        dependencies=[],
        tasks=[
            "Design planner API",
            "Implement goal decomposition",
            "Add task scheduling",
            "Implement agent coordination",
            "Add planning tests",
        ],
        labels=["enhancement", "milestone-0.5", "priority-medium", "ai"],
    ),
    PR(
        number=27,
        title="Distributed Compilation Demo",
        milestone="0.5",
        priority="Low",
        size="Medium",
        duration="1-2 weeks",
        description="Multi-node compilation demonstration with task protocol, demo scenario, and tutorial.",
        dependencies=[9, 25, 26],
        tasks=[
            "Implement compilation task protocol",
            "Create demo scenario",
            "Add performance metrics",
            "Document distributed workflow",
            "Create tutorial",
        ],
        labels=["documentation", "milestone-0.5", "priority-low", "demo"],
    ),
    # Milestone 0.6 - Subnet Adaptation & Evolution
    PR(
        number=28,
        title="BLE Signal Implementation",
        milestone="0.6",
        priority="Low",
        size="Medium",
        duration="1-2 weeks",
        description="Bluetooth Low Energy signaling with emission, reception, encoding, and range testing.",
        dependencies=[20],
        tasks=[
            "Implement BLE emission",
            "Implement BLE reception",
            "Add BLE signal encoding",
            "Test BLE range and reliability",
            "BLE documentation",
        ],
        labels=["enhancement", "milestone-0.6", "priority-low", "hardware"],
    ),
    PR(
        number=29,
        title="Signal Evolution Framework",
        milestone="0.6",
        priority="Low",
        size="Large",
        duration="2-3 weeks",
        description="Adaptive signal encoding with evolution algorithm, signal learning, and recognition.",
        dependencies=[23],
        tasks=[
            "Design evolution algorithm",
            "Implement signal learning",
            "Add signal recognition",
            "Implement fitness evaluation",
            "Evolution tests",
        ],
        labels=["enhancement", "milestone-0.6", "priority-low", "ai"],
    ),
    PR(
        number=30,
        title="Multi-Hop Communication",
        milestone="0.6",
        priority="Low",
        size="Large",
        duration="2-3 weeks",
        description="Multi-device signal relay with routing protocol, message forwarding, and performance analysis.",
        dependencies=[28, 29],
        tasks=[
            "Design multi-hop protocol",
            "Implement routing",
            "Add message forwarding",
            "Test 3+ device chains",
            "Performance analysis",
        ],
        labels=["enhancement", "milestone-0.6", "priority-low"],
    ),
    # Milestone 1.0 - Beta Release
    PR(
        number=31,
        title="API Stabilization",
        milestone="1.0",
        priority="High",
        size="Medium",
        duration="1-2 weeks",
        description="Finalize public APIs with stability levels, deprecation warnings, and comprehensive documentation.",
        dependencies=[2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30],
        tasks=[
            "Review all public APIs",
            "Mark stability levels",
            "Add deprecation warnings",
            "Update documentation",
            "API changelog",
        ],
        labels=["enhancement", "milestone-1.0", "priority-high"],
    ),
    PR(
        number=32,
        title="Documentation Overhaul",
        milestone="1.0",
        priority="High",
        size="Large",
        duration="2-3 weeks",
        description="Complete documentation with API docs, architecture guide, deployment guide, tutorials, and troubleshooting.",
        dependencies=[31],
        tasks=[
            "API documentation",
            "Architecture guide",
            "Deployment guide",
            "Tutorial series",
            "Troubleshooting guide",
        ],
        labels=["documentation", "milestone-1.0", "priority-high"],
    ),
    PR(
        number=33,
        title="Example Gallery",
        milestone="1.0",
        priority="High",
        size="Large",
        duration="2-3 weeks",
        description="Comprehensive examples including Hello World, sensor demo, distributed build, Grid communication, and full-stack demo.",
        dependencies=[31],
        tasks=[
            "Hello World example",
            "Sensor demo",
            "Distributed build demo",
            "Grid communication demo",
            "Full-stack demo app",
        ],
        labels=["documentation", "milestone-1.0", "priority-high", "demo"],
    ),
    PR(
        number=34,
        title="Installation & Deployment",
        milestone="1.0",
        priority="High",
        size="Medium",
        duration="1-2 weeks",
        description="Easy installation experience with scripts, platform packages, WebAssembly demo, and Docker images.",
        dependencies=[31],
        tasks=[
            "Create installation scripts",
            "Add platform packages (brew, apt, etc.)",
            "WebAssembly demo deployment",
            "Docker images",
            "Installation documentation",
        ],
        labels=["enhancement", "milestone-1.0", "priority-high"],
    ),
    PR(
        number=35,
        title="Performance Benchmarking Suite",
        milestone="1.0",
        priority="Medium",
        size="Medium",
        duration="1-2 weeks",
        description="Comprehensive benchmarks for event processing, network performance, storage, and end-to-end latency.",
        dependencies=[31],
        tasks=[
            "Event processing benchmarks",
            "Network performance tests",
            "Storage benchmarks",
            "End-to-end latency tests",
            "Benchmark documentation",
        ],
        labels=["testing", "milestone-1.0", "priority-medium"],
    ),
    # Cross-Cutting PRs
    PR(
        number=36,
        title="CI/CD Pipeline Enhancement",
        milestone="Cross-Cutting",
        priority="High",
        size="Medium",
        duration="1-2 weeks",
        description="Improve build and test automation with coverage reporting, regression detection, security scanning, and release automation.",
        dependencies=[],
        tasks=[
            "Add comprehensive test coverage reporting",
            "Add performance regression detection",
            "Implement automated security scanning",
            "Add release automation",
            "CI documentation",
        ],
        labels=["infrastructure", "priority-high"],
    ),
    PR(
        number=37,
        title="Error Handling Standardization",
        milestone="Cross-Cutting",
        priority="Medium",
        size="Medium",
        duration="1-2 weeks",
        description="Consistent error handling across codebase with error taxonomy, types, context, and recovery.",
        dependencies=[],
        tasks=[
            "Define error taxonomy",
            "Standardize error types",
            "Add error context",
            "Implement error recovery",
            "Error handling guide",
        ],
        labels=["enhancement", "priority-medium"],
    ),
    PR(
        number=38,
        title="Logging & Observability",
        milestone="Cross-Cutting",
        priority="Medium",
        size="Medium",
        duration="1-2 weeks",
        description="Comprehensive logging and monitoring with structured logging, distributed tracing, and metrics exporters.",
        dependencies=[],
        tasks=[
            "Standardize logging levels",
            "Add structured logging",
            "Implement distributed tracing",
            "Add metrics exporters",
            "Observability documentation",
        ],
        labels=["enhancement", "priority-medium"],
    ),
    PR(
        number=39,
        title="Security Audit & Hardening",
        milestone="Cross-Cutting",
        priority="Critical",
        size="Large",
        duration="2-4 weeks",
        description="Security review and improvements with comprehensive audit, vulnerability fixes, and security testing.",
        dependencies=[],
        tasks=[
            "Conduct security audit",
            "Fix identified vulnerabilities",
            "Add security tests",
            "Implement fuzzing",
            "Security documentation",
        ],
        labels=["security", "priority-critical"],
    ),
    PR(
        number=40,
        title="Platform-Specific Optimizations",
        milestone="Cross-Cutting",
        priority="Medium",
        size="Large",
        duration="2-4 weeks",
        description="Optimize for target platforms including mobile, embedded, browser/WASM, and desktop.",
        dependencies=[],
        tasks=[
            "Mobile optimizations (battery, memory)",
            "Embedded device optimizations",
            "Browser/WASM optimizations",
            "Desktop optimizations",
            "Platform guides",
        ],
        labels=["performance", "priority-medium"],
    ),
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
    import argparse
    
    parser = argparse.ArgumentParser(description='Create GitHub issues for CortexOS PRs')
    parser.add_argument('--dry-run', action='store_true', 
                       help='Preview issues without creating them')
    parser.add_argument('--phase', type=int, choices=range(1, 9),
                       help='Create issues for a specific phase (1-7) or all cross-cutting (8)')
    args = parser.parse_args()

    # Filter PRs by phase if specified
    prs_to_create = PRS
    if args.phase:
        phase_prs = {
            1: [2, 3, 4, 7, 36, 37, 39],  # Phase 1: Foundation
            2: [5, 6, 8, 9, 10, 11, 12, 13],  # Phase 2: Core Features
            3: [14, 15, 16, 17, 18, 19],  # Phase 3: Cognitive
            4: [20, 21, 22, 23],  # Phase 4: Physical
            5: [24, 25, 26, 27],  # Phase 5: Intelligence
            6: [28, 29, 30],  # Phase 6: Advanced
            7: [31, 32, 33, 34, 35],  # Phase 7: Beta Release
            8: [36, 37, 38, 39, 40],  # Cross-cutting
        }
        phase_numbers = phase_prs.get(args.phase, [])
        prs_to_create = [pr for pr in PRS if pr.number in phase_numbers]
        print(f"Phase {args.phase} selected: {len(prs_to_create)} issues")
    
    print("CortexOS Issue Creator")
    print("=" * 50)
    print(f"Total PRs to create: {len(prs_to_create)}")
    
    if args.dry_run:
        print("\n🔍 DRY RUN MODE - No issues will be created\n")
        print("=" * 50)
        for pr in prs_to_create:
            title = f"PR #{pr.number}: {pr.title}"
            print(f"\n📝 Would create: {title}")
            print(f"   Milestone: {pr.milestone}")
            print(f"   Priority: {pr.priority}")
            print(f"   Size: {pr.size}")
            print(f"   Labels: {', '.join(pr.labels)}")
            if pr.dependencies:
                print(f"   Dependencies: PR #{', #'.join(map(str, pr.dependencies))}")
            print(f"   Tasks: {len(pr.tasks)} tasks")
        print("\n" + "=" * 50)
        print(f"Summary: {len(prs_to_create)} issues would be created")
        print("To actually create issues, run without --dry-run")
        return

    # Get GitHub token
    token = os.environ.get("GITHUB_TOKEN")
    if not token:
        print("\nError: GITHUB_TOKEN environment variable not set")
        print("Set it with: export GITHUB_TOKEN=your_token_here")
        print("\nTo preview without creating, use: --dry-run")
        sys.exit(1)

    # Initialize GitHub client
    try:
        Github = import_github()
        if Github is None:
            print("\nError: PyGithub not installed.")
            print("Install it with: pip install PyGithub")
            sys.exit(1)
            
        g = Github(token)
        repo = g.get_repo("therenansimoes/cortexOS")
        print(f"Repository: {repo.full_name}")
    except Exception as e:
        print(f"\nError connecting to GitHub: {e}")
        print("Please check your GITHUB_TOKEN and internet connection")
        sys.exit(1)

    print()

    # Ask for confirmation
    response = input(f"Create {len(prs_to_create)} issues? (y/N): ")
    if response.lower() != "y":
        print("Cancelled.")
        sys.exit(0)

    # Create issues
    created = 0
    failed = 0
    for pr in prs_to_create:
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
            failed += 1

    print()
    print(f"Created {created}/{len(prs_to_create)} issues successfully!")
    if failed > 0:
        print(f"Failed to create {failed} issues")


if __name__ == "__main__":
    main()
