# Compiler Test Framework

This directory contains a comprehensive test framework for the C compiler, built on top of the [writing-a-c-compiler-tests](https://github.com/nlsandler/writing-a-c-compiler-tests) test suite.

## Overview

The test framework provides:

1. **Dynamic test discovery** - Automatically finds all test cases from the writing-a-c-compiler-tests suite
2. **Flexible configuration** - Control which chapters, stages, and features to test
3. **Integration with libtest-mimic** - Provides a familiar test runner interface
4. **Extra credit support** - Enable/disable specific language features
5. **Stage-specific testing** - Test individual compilation stages

## Quick Start

### Running Tests with libtest-mimic

The main integration test uses [libtest-mimic](https://docs.rs/libtest-mimic/latest/libtest_mimic/) to provide a familiar test runner interface:

```bash
# Test chapters 1-4 with default settings
cargo test --test compliance_tests -- --chapter 4

# Test only valid tests up to parse stage
cargo test --test compliance_tests -- --chapter 4 --stage parse --skip-invalid

# Test with extra credit features
cargo test --test compliance_tests -- --chapter 9 --extra-credit bitwise,compound

# Test with custom compiler and options
cargo test --test compliance_tests -- --compiler ./my-compiler --compiler-option --verbose
```

### Programmatic Usage

You can also use the test framework programmatically:

```rust
use tests::{CompilationStage, ExtraCreditFlags, TestConfig, TestRunner};

let mut config = TestConfig::default();
config.max_chapter = 4;
config.stage = CompilationStage::Run;
config.extra_credit = ExtraCreditFlags::BITWISE;

let runner = TestRunner::new(config)?;
let test_cases = runner.discover_tests()?;

for test_case in test_cases {
    let result = runner.run_test(&test_case);
    println!("{}: {}", test_case.name, if result.passed { "PASS" } else { "FAIL" });
}
```

## Configuration Options

### TestConfig

- `compiler_path`: Path to the compiler under test (default: `target/debug/mcc`)
- `max_chapter`: Maximum chapter to test (default: 20)
- `stage`: Compilation stage to test up to (default: `Run`)
- `skip_invalid`: Whether to skip invalid test cases (default: false)
- `extra_credit`: Extra credit features to enable (default: none)
- `expected_error_codes`: Expected error codes for invalid tests (default: any non-zero)
- `timeout`: Timeout for individual tests (default: 30 seconds)
- `compiler_options`: Additional compiler options

### Compilation Stages

- `Lex`: Test lexical analysis only
- `Parse`: Test parsing only
- `Validate`: Test semantic validation
- `Tacky`: Test intermediate representation
- `Codegen`: Test code generation
- `Run`: Test full compilation and execution

### Extra Credit Features

- `Bitwise`: Bitwise operators (`&`, `|`, `^`, `<<`, `>>`)
- `Compound`: Compound assignment operators (`+=`, `-=`, etc.)
- `Increment`: Increment/decrement operators (`++`, `--`)
- `Goto`: Goto statements and labels
- `Switch`: Switch statements
- `Nan`: Floating-point NaN handling
- `Union`: Union types

## Test Organization

Tests are organized by chapters that correspond to different stages of compiler development:

- **Chapters 1-10**: Basic language features (lexing, parsing, basic expressions)
- **Chapters 11-18**: Advanced language features (pointers, arrays, structs, etc.)
- **Chapter 19**: Optimizations (constant folding, dead code elimination, etc.)
- **Chapter 20**: Register allocation

Each chapter contains:
- `valid/`: Programs that should compile and run successfully
- `invalid_lex/`: Programs with lexical errors
- `invalid_parse/`: Programs with parsing errors
- `invalid_semantics/`: Programs with semantic errors
- `invalid_types/`: Programs with type errors
- And more...

## Expected Results

The framework automatically loads expected results from `expected_results.json` in the test suite. For valid tests, it compares:
- Return codes
- Standard output

For invalid tests, it verifies that compilation fails appropriately.

## Skipping Tests

You can skip individual tests by modifying the test discovery logic in `TestRunner::should_run_test()`. This is useful for:
- Tests that require features not yet implemented
- Tests that are known to be problematic
- Performance optimization

## Integration with Your Compiler

Your compiler should:

1. Accept source files as command-line arguments
2. Support a `--stage` option to stop at specific compilation stages
3. Exit with appropriate error codes for invalid programs
4. Produce executables or assembly output as needed
5. Support optimization flags for Chapter 19+ tests

Example compiler interface:
```bash
./my-compiler input.c                    # Full compilation
./my-compiler --stage parse input.c      # Stop after parsing
./my-compiler --fold-constants input.c   # Enable optimizations
```

## Examples

See `tests/src/tests/example_test.rs` for examples of how to use the framework programmatically.

## Troubleshooting

### No tests discovered
- Ensure the writing-a-c-compiler-tests directory exists
- Check that the test suite is properly initialized
- Verify the chapter directories exist

### Tests failing unexpectedly
- Check that your compiler supports the required features
- Verify expected results in `expected_results.json`
- Use `--skip-invalid` to focus on valid tests during development

### Performance issues
- Use `--chapter` to limit the number of chapters tested
- Use `--skip-invalid` to reduce test count
- Adjust timeout settings if needed
