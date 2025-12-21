# Signal Evolution Example

This example demonstrates the **Signal Evolution Framework** - an adaptive communication protocol learning system that uses reinforcement learning to optimize signal patterns across different communication channels.

## Overview

The Signal Evolution Framework combines three key components:

1. **Evolution Engine**: Generates and evolves signal patterns through genetic algorithms
2. **Recognition Engine**: Learns from observed patterns and maintains templates
3. **Learning System**: Coordinates evolution and recognition to optimize communication

## What This Example Shows

### Phase 1: Initial Pattern Evolution
- Creates initial random populations of signal patterns
- Simulates communication attempts with varying success rates
- Automatically evolves patterns based on fitness metrics (SNR, latency, success rate)
- Demonstrates multiple generations of pattern improvement

### Phase 2: Pattern Recognition
- Shows how to teach the system a known-good pattern
- Demonstrates template-based learning and recognition
- Tracks success rates for learned patterns

### Phase 3: Multi-Symbol Evolution
- Evolves different patterns for different communication symbols
- Demonstrates fitness evaluation with multiple criteria
- Shows how patterns adapt to different success conditions

## Running the Example

```bash
cargo run -p signal-evolution-example
```

## Key Features Demonstrated

### Adaptive Learning
The system learns from communication outcomes:
- **Success**: Patterns that work well are reinforced
- **Failure**: Poor patterns are evolved away
- **Automatic Evolution**: Triggers after every N attempts (configurable)

### Fitness Metrics
Patterns are evaluated on multiple dimensions:
- **Success Rate**: Did the communication succeed?
- **SNR (Signal-to-Noise Ratio)**: How clear was the signal?
- **Latency**: How fast was the transmission?
- **Energy Cost**: How efficient was the pattern?
- **Distinctiveness**: How unique is this pattern?

### Learning Strategies
Three strategies are available:
- **Evolution**: Pure genetic algorithm approach
- **Recognition**: Learn from observed patterns
- **Hybrid**: Combines both methods (used in this example)

## Output Explanation

The example shows visual signal patterns using ASCII characters:
- `█` = Signal ON (pulse)
- `░` = Signal OFF (gap)

Each pattern's duration is shown in microseconds (µs).

Statistics are displayed showing:
- Generation number (how many evolution cycles)
- Success rate percentage
- Number of successful vs failed communications
- Number of patterns learned

## Evolution Configuration

The example uses these default settings:
- Population size: 50 patterns per symbol
- Mutation rate: 20%
- Auto-evolution: Every 5 communication attempts
- Minimum confidence: 70% for pattern recognition
- Fitness threshold: 70% for accepting patterns

## Real-World Applications

This framework could be used for:
- **Adaptive IoT Communication**: Optimize signal patterns for varying environmental conditions
- **Mesh Networks**: Evolve efficient routing patterns
- **Low-Power Devices**: Learn energy-efficient communication protocols
- **Hostile Environments**: Adapt to interference and noise
- **Multi-Modal Communication**: Switch between light, audio, BLE based on performance

## Architecture

```
LearningSystem
├── EvolutionEngine
│   ├── Population management
│   ├── Mutation operators
│   ├── Crossover operators
│   └── Fitness tracking
├── RecognitionEngine
│   ├── Template matching
│   ├── Pattern distance calculation
│   └── Usage statistics
└── Learning coordination
    ├── Strategy selection
    ├── Outcome recording
    └── Auto-evolution triggers
```

## Related Code

- `crates/signal/src/evolution.rs` - Evolution algorithm implementation
- `crates/signal/src/recognition.rs` - Pattern recognition engine
- `crates/signal/src/learning.rs` - Coordinated learning system
- `crates/signal/src/codebook.rs` - Symbol-to-pattern mapping

## Next Steps

To extend this example:
1. Add real hardware emitters (LED, speaker, BLE)
2. Implement multi-device communication testing
3. Add environmental noise simulation
4. Create fitness functions for specific use cases
5. Persist evolved patterns to disk for reuse
