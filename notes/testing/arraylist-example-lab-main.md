# arraylist-example-lab (Branch: `main`)

This branch of the **arraylist-example-lab** repository provides a more comprehensive setup to test **all** major features of our autograder, including PIT mutation coverage. While the code structure itself may be simple, the goal is to ensure our autograder correctly:

1. **Builds and Tests** the project via Gradle.  
2. **Performs Mutation Coverage** using [PIT](https://pitest.org/).  
3. **Captures and Parses** any additional artifacts (reports, logs, JSON files).  
4. **Generates Consistent Output** for snapshot testing.

## Project Overview

- **Source Files**  
  - `src/Main.java`: A basic “Hello, World!” entry point for demonstration.  
  - *(Pending or Future): Additional classes/tests that illustrate arraylist functionality and exercises for PIT coverage.*
- **Gradle Configuration**  
  - Gradle 8.11.1 (via the wrapper).  
  - Java toolchain pinned to version 17.  
  - Build tasks, test tasks, and coverage tasks (PIT) can be configured or extended as needed.

## Key Gradle Tasks to Test

1. **`gradlew build`**  
   - Standard compilation, packaging, and (optionally) test execution.

2. **`gradlew test`**  
   - Runs JUnit tests (if present).  
   - Generates reports that the autograder can parse for partial credit or feedback.

3. **`gradlew pitest`**  
   - **Critical** for verifying mutation coverage functionality.  
   - Produces XML/HTML reports in `build/reports/pitest/` which the autograder will parse and incorporate into the overall grade.

4. **`gradlew run`**  
   - Demonstrates the runtime behavior of `Main.java` (helpful for checking standard output).

*(As we evolve the lab to have more advanced tests or style checks, we may add tasks like `checkstyle`, `javadoc`, etc.)*

## Snapshot Testing Strategy

We use [`insta`](https://docs.rs/insta) (or a similar tool) to capture the autograder’s outputs—console logs, JSON summaries, PIT coverage metrics—and compare them with expected “snapshots.”

1. **Checkout Branch**  
   ```bash
   git checkout main
   ```
2. **Run Autograder**  
   ```bash
   umm-next grade grading-script.rn
   ```
   - Invokes Gradle tasks (including `test`, `pitest`) as configured.  
   - Collects test results, coverage reports, style checks, etc.  
   - Consolidates them in output logs and optionally a Gradescope-compatible JSON.
3. **Snapshot Comparison**  
   - The outputs from step 2 are saved and compared to prior snapshots to detect regressions or unexpected changes.

## Why We Use This Branch

- **Comprehensive Testing**  
  The `main` branch is our go-to environment for ensuring that *all* advanced autograder functionalities—especially PIT mutation coverage—work end-to-end.
- **Future Growth**  
  As we add more tooling (static analysis, doc-lint, advanced coverage) or new tasks, this branch is the logical place to test them without overcomplicating other simpler branches.
- **Consistency Across Environments**  
  We want to confirm that everything from local development (VSCode tasks) to CI pipelines (GitHub Actions, container-based environments, etc.) produces consistent results.

## Repository & Branch Details

- **Repo Remote**: [https://github.com/DhruvDh/arraylist-lab-example-java.git](https://github.com/DhruvDh/arraylist-lab-example-java.git)  
- **Branch**: `main`  
- **Local Workflow**: The branch is typically cloned or checked out locally for comprehensive autograder feature testing.