# 🦾 URDFix

A fast, Rust-powered CLI tool for cleaning, formatting, and repairing URDF (Unified Robot Description Format) files.

## What is URDFix?

URDFix is designed for roboticists, researchers, and engineers who work with robot description files. Whether you're building a simple manipulator or a complex humanoid robot, URDF files can quickly become messy with inconsistent formatting, duplicate elements, and structural issues that make debugging and maintenance difficult.

URDFix automates the tedious task of cleaning up these files, ensuring they're properly formatted, validated, and ready for use in simulation or deployment environments like ROS, Gazebo, or MoveIt.

## Features

- **Format & Clean**: Automatically formats URDF files for consistency and readability
- **Validate**: Checks XML structure and highlights URDF-specific issues  
- **Fix**: Removes duplicates, extraneous whitespace, and structural problems
- **Analyze**: Provides insights into robot structure and potential issues
- **Convert**: Transform URDF files to other formats
- **Diff**: Compare two URDF files to see differences

## Installation

```bash
# From source
git clone https://github.com/pratyaypandey/urdfix
cd urdfix
cargo build --release

# Or install directly (when published)
cargo install urdfix
```

## Quick Start

```bash
# Format a URDF file
urdfix format robot.urdf

# Fix common issues
urdfix fix robot.urdf

# Validate structure
urdfix validate robot.urdf

# Analyze robot properties
urdfix analyze robot.urdf

# Compare two files
urdfix diff robot1.urdf robot2.urdf
```

## Commands

### `urdfix lint <file>`
Check for common issues and best practices violations.

### `urdfix fix <file>`  
Automatically fix structural issues, remove duplicates, and clean up formatting.

### `urdfix format <file>`
Reformat URDF with consistent indentation and spacing.

### `urdfix analyze <file>`
Show statistics and insights about the robot structure.

### `urdfix convert <file>`
Convert URDF to other formats (planned: SDF, XACRO).

### `urdfix diff <file1> <file2>`
Compare two URDF files and highlight differences.

## Global Options

- `-v, --verbose`: Enable detailed output
- `-h, --help`: Show command help

## Example

**Before:**
```xml
<link name="base_link" />
  <link name="base_link" />
  <joint name="joint1" type="revolute" />
<!-- messy whitespace and duplicate links -->
```

**After `urdfix fix robot.urdf`:**
```xml
<link name="base_link"/>
<joint name="joint1" type="revolute"/>
```

## Why URDFix?

URDF files often accumulate structural issues, inconsistencies, and clutter—especially after manual edits or auto-generation. URDFix ensures your robot descriptions are clean, valid, and production-ready.

Think of it as `prettier` or `black`, but for robotics.

## Development

```bash
# Build
cargo build

# Run tests  
cargo test

# Run with verbose output
cargo run -- fix robot.urdf -v
```

## Roadmap

- [x] Basic CLI structure
- [ ] XML parsing and validation
- [ ] Formatting and cleanup
- [ ] URDF-specific linting rules
- [ ] XACRO compatibility
- [ ] Interactive refactoring tools

## License

MIT License © 2025 Pratyay Pandey

## Contributing

Issues, feature requests, and PRs welcome at [GitHub](https://github.com/pratyaypandey/urdfix). 