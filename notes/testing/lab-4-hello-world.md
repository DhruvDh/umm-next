# Lab 4: Hello World (Branch `origin/lab-4`)

This branch contains a minimal Java "Hello, World!" project to validate our autograder's ability to:

1. **Detect project structure** (Gradle-based Java)
2. **Invoke Gradle** tasks (`test`, `run`, `build`) and parse output artifacts
3. **Produce consistent output** (logs, JSON results) for snapshot testing

## Project Overview

- **Source Files**
  - `src/Main.java`: Simple entry point printing "Hello, World!"
- **Gradle Configuration**
  - Gradle 8.11.1 (via wrapper)
  - `build.gradle.kts`: Sets up `application` plugin and Java 17 toolchain
  - No test classes: `gradlew test` results in **0 tests run**

## Key Gradle Tasks to Test

1. **`gradlew build`**: Compiles, runs (zero) tests, packages output
2. **`gradlew test`**: Verifies empty test report parsing
3. **`gradlew run`**: Launches `Main` class, prints "Hello, World!"

## Snapshot Testing Strategy

Using [`insta`](https://docs.rs/insta) to capture and compare autograder outputs:

1. **Checkout branch**
   ```bash
   git checkout origin/lab-4
   ```
2. **Run autograder**
   ```bash
   umm-next grade grading-script.rn
   ```
   - Calls Gradle tasks
   - Parses test report
   - Generates output (console table or JSON)
3. **Create snapshot**
   - Record CLI output or JSON summary
   - Compare future runs against snapshot

## Remote Info

- **Remote**: https://github.com/DhruvDh/arraylist-lab-example-java.git
- **Branch**: `lab-4`
- **Local**: Cloned for snapshot tests

## Purpose

- **Sanity Check**: Verify autograder handling of trivial Gradle project
- **Baseline**: Confirm basic functionality without complex tests
- **Foundation**: Prepare for more advanced lab configurations