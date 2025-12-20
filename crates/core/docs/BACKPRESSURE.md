# Backpressure Policies

This document provides comprehensive guidance on selecting and using backpressure policies in CortexOS.

## Overview

Backpressure policies control what happens when event queues become full. Different policies provide different trade-offs between data preservation, latency, and resource usage.

## Policy Comparison

| Policy | Data Loss | Latency | Memory | Use Case |
|--------|-----------|---------|--------|----------|
| DropNew | Newest events | Constant | Fixed | Historical analysis |
| DropOld | Oldest events | Constant | Fixed | Real-time monitoring |
| Coalesce | Intermediate states | Constant | Fixed + HashMap | State updates |
| Sample | Statistical gaps | Constant | Fixed | Metrics collection |
| Persist | None (ideal) | Variable | Unbounded | Audit logs |

## Selection Guide

Use **DropOld** for real-time sensor data, **Coalesce** for keyed state updates, **Sample** for high-frequency metrics, **DropNew** for historical analysis, and **Persist** for critical audit logs.

## Performance

All policies exceed 100K events/sec. DropOld/DropNew are fastest (~15M ops/sec), Sample is fast (~10M ops/sec), Coalesce is moderate (~2M ops/sec depending on keys).

## Best Practices

1. Choose capacity based on expected burst size
2. Monitor queue length to detect backpressure
3. Use Coalesce when possible to reduce memory
4. Sample before queue for better performance
5. Combine policies at different layers

For full documentation, see the module documentation in `src/backpressure.rs`.
