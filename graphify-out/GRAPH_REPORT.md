# Graph Report - .  (2026-04-11)

## Corpus Check
- Corpus is ~2,386 words - fits in a single context window. You may not need a graph.

## Summary
- 29 nodes · 23 edges · 6 communities detected
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## God Nodes (most connected - your core abstractions)
1. `EegSensor` - 4 edges
2. `SpeedSensor` - 4 edges
3. `LogWriter<'a>` - 2 edges
4. `EegData` - 1 edges
5. `LogWriter` - 1 edges
6. `Shared` - 1 edges
7. `Local` - 1 edges

## Surprising Connections (you probably didn't know these)
- None detected - all connections are within the same source files.

## Communities

### Community 0 - "Main Orchestration & Logging"
Cohesion: 0.2
Nodes (3): LogWriter, Local, Shared

### Community 1 - "EEG Sensor Module"
Cohesion: 0.33
Nodes (2): EegData, EegSensor

### Community 2 - "Speed Sensor Module"
Cohesion: 0.4
Nodes (1): SpeedSensor

### Community 3 - "Python Data Reader"
Cohesion: 0.67
Nodes (0): 

### Community 4 - "Log Writer Implementation"
Cohesion: 0.67
Nodes (1): LogWriter<'a>

### Community 5 - "Build Script"
Cohesion: 1.0
Nodes (0): 

## Knowledge Gaps
- **4 isolated node(s):** `EegData`, `LogWriter`, `Shared`, `Local`
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Build Script`** (2 nodes): `build.rs`, `main()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **What connects `EegData`, `LogWriter`, `Shared` to the rest of the system?**
  _4 weakly-connected nodes found - possible documentation gaps or missing edges._