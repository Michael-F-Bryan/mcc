# Architecture

This document describes the high-level architecture of Michael's C Compiler (mcc), a self-hosting C compiler written in Rust.

## Problem Overview

The compiler transforms C source code into executable machine code through a series of well-defined stages: preprocessing, parsing, semantic analysis, intermediate representation generation, code generation, and linking. The project emphasizes correctness through comprehensive testing, incremental compilation for fast development cycles, and clear separation of concerns between compilation stages.

## Code Map

### Crates

The project is organized into several crates, each with a specific responsibility:

- **`mcc`** - The core compiler library containing the main compilation pipeline
- **`mcc-syntax`** - Tree-sitter integration and strongly-typed AST nodes
- **`mcc-driver`** - Command-line interface and orchestration
- **`xtask`** - Build-time tooling and development utilities

### Core Compilation Pipeline (`mcc` crate)

The main compilation stages are implemented as separate modules:

- **`preprocessing`** - Handles C preprocessor directives using external C compiler
- **`parsing`** - Tree-sitter-based parsing with error recovery and validation
- **`lowering`** - Transforms AST into Three Address Code (TAC) intermediate representation
- **`codegen`** - Generates target assembly from TAC
- **`assembling`** - Orchestrates assembly and linking with external tools

### Data Flow

The compilation follows a linear pipeline where each stage consumes the output of the previous stage:

```
Source File → Preprocessing → Parsing → Lowering → Code Generation → Assembly → Linking
```

Each stage is implemented as a Salsa tracked function, enabling incremental compilation and caching of intermediate results.

### Key Types and Abstractions

- **`SourceFile`** - Represents a source file with path and contents
- **`Ast`** - Wraps the tree-sitter parse tree with strongly-typed accessors
- **`tacky::Program`** - Three Address Code intermediate representation
- **`asm::Program`** - Target-specific assembly representation
- **`Database`** - Salsa database for incremental compilation
- **`Text`** - Reference-counted string type for efficient memory sharing
- **`Files`** - File collection for error reporting and source management

### Module Boundaries

**Syntax Layer** (`mcc-syntax`): Provides strongly-typed AST nodes generated from tree-sitter grammar. This layer is independent of compilation logic and focuses purely on syntax representation.

**Core Compiler** (`mcc`): Contains all compilation logic but depends on the syntax layer for AST access. The core crate is structured to minimize dependencies between compilation stages.

**Driver** (`mcc-driver`): Orchestrates the compilation pipeline and handles user interaction. Depends on the core crate but doesn't contain compilation logic.

## Cross-Cutting Concerns

### Incremental Compilation

The entire compilation pipeline uses Salsa for incremental compilation. Each major compilation stage is implemented as a tracked function, allowing the compiler to reuse cached results when source files haven't changed.

### Error Handling and Diagnostics

Error reporting is centralized through the `diagnostics` module. Each compilation stage can accumulate diagnostics, which are then formatted and displayed to the user using codespan-reporting.

### Testing

The project includes a comprehensive test framework based on the writing-a-c-compiler-tests suite. Tests are organized by chapters corresponding to different language features, with support for testing individual compilation stages. The framework supports:

- **Progressive Testing**: Tests are organized by chapters (1-20) representing increasing language complexity
- **Stage-Specific Testing**: Can test individual compilation stages (lex, parse, tacky, codegen)
- **Snapshot Testing**: Uses insta for regression testing of AST and IR representations
- **Integration Testing**: Full end-to-end compilation and execution testing

### Target Support

The compiler targets x86_64 by default but is designed to support multiple architectures through the target-lexicon crate. Assembly generation is target-specific, while the intermediate representations are target-agnostic.

## Architectural Invariants

- No compilation stage depends on later stages in the pipeline
- The syntax layer contains no compilation logic
- All compilation stages are implemented as pure functions with Salsa tracking
- Error handling is non-fatal - compilation continues to collect all errors
- The driver crate contains no compilation logic, only orchestration

## Boundaries

**Syntax/Compilation Boundary**: The `mcc-syntax` crate provides the AST interface, while `mcc` contains all compilation logic. This boundary ensures that syntax changes don't require recompiling the entire compiler.

**Pipeline Stage Boundaries**: Each compilation stage is implemented as a separate module with clear input/output contracts. Stages communicate only through well-defined data structures.

**External Tool Boundary**: The compiler delegates preprocessing, assembly, and linking to external tools (typically the system C compiler). This boundary allows the compiler to focus on the core compilation logic while leveraging mature external tools.

**Error Handling Boundary**: All compilation stages accumulate diagnostics rather than failing immediately, allowing the compiler to report all errors in a single pass.
