# umm-next

## 1. Project Background and Motivation

### 1.1 Original "umm" Autograder

- The original `umm` tool was a scriptable build tool/grader for Java novices:
  - Students typically ran `umm [command]` in a local or CI environment.
  - It used Java's compiler or external tasks, parsed test results, and provided final numeric or textual feedback.
  - **Rhai scripts** were used to define custom grading logic (e.g., penalizing missing Javadoc, partial credit for tests, etc.).

### 1.2 New Goals

**"umm-next"** is the next iteration with several key objectives:

1. **Full Gradle Integration**:
   - Instead of manually invoking `javac` or external scripts, rely on **Gradle** to build/test Java.
   - Parse Gradle's XML or HTML outputs (e.g., JUnit, PIT, Checkstyle) to gather results.

2. **Rune Scripting** (Replacing Rhai):
   - Move all custom grading flows to **Rune** scripts.
   - Maintain naming (e.g., `new_by_unit_test_grader()`, `GradeResult`) for backward familiarity, but use Rune as the underlying engine.

3. **Multiple Language Support**:
   - Expand beyond Java-centric approach to handle **Python** (Pytest) and potentially other languages (e.g., Rust) in the future.
   - Use **a single config file** (with `figment`) to indicate the chosen language, build/test commands, and other environment details.

4. **Preserving Familiar Patterns**:
   - Maintain the "flow": a single CLI command `umm-next grade <script>` triggers:
     1. Build & test the code.
     2. Parse results into data structures.
     3. Hand them to the "grader script".
     4. Show a table summarizing the outcome, optionally produce a Gradescope JSON, etc.
   - Continue to produce `GradeResult`, show a table, do partial scoring, etc.

5. **Support for Local (VSCode) + CI**:
   - Enable students to run primarily in VSCode tasks locally.
   - Allow instructors to run in a container or CI environment (e.g., GitHub Actions or Gradescope container) for final scoring.
   - Provide consistent "table on CLI + `results.json` for Gradescope" outputs.

6. **Optional LLM Integration**:
   - Plan for future embedding of advanced feedback generation with an LLM (similar to "SGLang").
   - Account for hooking into advanced prompt logic from Rune, if desired.

## 2. High-Level Architecture

Here's an **end-to-end** outline of how "umm-next" will operate in a typical Java scenario, with extension to other languages:

1. **Single Self-Updating Binary**
   - Distribute "umm-next" as a single Rust binary, similar to the old `umm`.
   - Include logic for self-updates (using a crate like `self_update`).
   - Avoid requiring users to install Rust or Python directly; they download or use a script/VSCode task to obtain the correct binary.

2. **Configuration with figment**
   - Use a file like `umm-next.toml` in the project folder to specify:
     ```toml
     [project]
     language = "java"

     [java]
     gradle_tasks = ["test", "pitest"]
     # Additional Gradle or doc-lint tasks can go here
     # Possibly Java version, etc.

     [python]
     # If using python, specify:
     # command = "pytest"
     # etc.
     ```
   - "umm-next" merges these config values to decide which pipeline to run.

3. **Running Gradle (Java Flow)**
   - For `language="java"`, call the Gradle wrapper (`./gradlew`) with tasks from config (e.g., `test` or `pitest`).
   - Parse results:
     - JUnit test results (`<project>/build/test-results/` or `<project>/build/reports/tests/test`)
     - PIT mutation results (`<project>/build/reports/pitest/mutations.xml`)
     - Possibly Checkstyle for doc-lint
   - Assemble a `ProjectReport` or similar struct in Rust.

4. **Rune Scripting**
   - Load the "grading script" (a `.rn` file).
   - Example script content:
     ```rune
     fn main() {
         let test_grade = new_by_unit_test_grader("./build/test-results");
         let mutation_grade = new_by_mutation_grader("./build/reports/pitest/mutations.xml");
         // ...
         show_results([test_grade, mutation_grade]);
     }
     ```
   - Alternatively, pass an entire `ProjectReport` object to `main(report)`.

5. **Output**
   - Print a table or call `show_results()` at the end of the script.
   - Write a "results.json" for Gradescope or other downstream usage.

6. **Future Python or Other Languages**
   - For `language="python"`, run Pytest, parse `results.xml`, produce a similar `ProjectReport`.
   - Maintain consistent Rune scripts: introduce `new_by_pytest_grader()` if needed.
   - Add new pipelines over time without changing the script's overall structure.

## 3. Detailed Steps and Components

### 3.1 CLI + Config

1. **CLI Command**
   - `umm-next grade <rune_script.rn> [--config umm-next.toml]`
   - Allow users to supply an alternate config path or rely on defaults.

2. **Load Configuration with figment**
   - **Load** the `.toml` file (e.g., `Figment::new().merge(Toml::file("umm-next.toml")).extract()`).
   - **Deserialize** into a `Config` struct including `[project]` and `[java]` (or `[python]`) sections.

3. **Language Dispatch**
   - Call `gradle_pipeline::execute(&config)` for `language="java"`.
   - Call `python_pipeline::execute(&config)` (or future approach) for `language="python"`.

### 3.2 Orchestrating Gradle (Java)

Within `gradle_pipeline::execute(&JavaConfig)`:

1. **Locate `gradlew`**
   - Usually in the project root. Ensure it's executable if needed.

2. **Run Gradle**
   - For each task in config (e.g., `["test", "pitest"]`), run `Command::new("./gradlew").arg(task).output()`.

3. **Parse Reports**
   - JUnit test results: `build/test-results/test/*.xml`
   - PIT mutation results: `build/reports/pitest/mutations.xml`
   - Possibly Checkstyle results: `build/reports/checkstyle/main.xml`
   - Summarize data into a `ProjectReport` or separated `TestReport`, `MutationReport`, etc.

4. **Return** the structured data.

### 3.3 Python Flow (Future)

For `language="python"`:
- Use `pytest --junitxml=results.xml`, parse results, produce a `ProjectReport`.

### 3.4 Rune Integration

**Key Innovations**:

1. **Registering External Types**
   - Define a `GradeResult` struct in Rust:
     ```rust
     #[derive(Debug, rune::Any)]
     pub struct GradeResult {
         pub name: String,
         pub score: f64,
         pub out_of: f64,
         pub reason: String,
     }
     ```
   - Expose it to Rune with methods or field getters:
     ```rust
     use rune::Module;

     pub fn create_module() -> Result<Module, rune::ContextError> {
         let mut module = Module::new();
         module.ty::<GradeResult>()?;
         // Possibly method to create or manipulate it
         Ok(module)
     }
     ```

2. **Implementing Grader Functions**
   - Define Rust functions for each "grader":
     ```rust
     fn new_by_unit_test_grader(test_report: &TestReport) -> GradeResult {
         // e.g. for each test that fails => subtract points
         // ...
         GradeResult { name: "Unit Tests".into(), score, out_of, reason }
     }
     ```
   - Register as associated functions in the Rune module:
     ```rust
     module.associated_function("new_by_unit_test_grader", new_by_unit_test_grader)?;
     ```
   - Use in Rune script:
     ```rune
     fn main() {
       let test_results = new_by_unit_test_grader("test_report.xml");
       let coverage = new_by_coverage_grader("coverage.xml");
       show_results([test_results, coverage]);
     }
     ```

3. **Advanced LLM Prompting**
   - Implement SGLang-like flows in Rust or as a specialized Rune function:
     ```rust
     fn generate_llm_feedback(grade_result: &GradeResult) -> String {
         // call out to your LLM or sglang server
         // return the feedback text
     }
     module.associated_function("generate_llm_feedback", generate_llm_feedback)?;
     ```
   - Use in instructor's script:
     ```rune
     let feedback = generate_llm_feedback(test_results);
     println(feedback);
     ```

### 3.5 Final Output

- Replicate the original `umm` style:
  ```rune
  fn main() {
    let t = new_by_unit_test_grader("./build/test-results/test");
    let m = new_by_mutation_grader("./build/reports/pitest/mutations.xml");
    show_results([t, m]); // prints a table, writes Gradescope results, etc.
  }
  ```
- Implement `show_results()` to:
  - Summarize all `GradeResult`s and print them in a pretty ASCII table.
  - Write to `results.json` for Gradescope if `--gradescope` or config demands it.

## 4. Implementation Roadmap

High-level milestones (adapt to your timeline):

1. **Initialize Rust Project**
   - Reuse or adapt the existing "umm" repository structure.
   - Introduce a new subcommand for `grade` or replace the old approach in `main.rs`.

2. **Configuration System**
   - Add `figment` to `Cargo.toml`.
   - Create `config.rs` with `#[derive(Deserialize)]` types for `[project]`, `[java]`, `[python]`, etc.
   - Write a `load_config()` function to read `umm-next.toml`.

3. **Gradle Integration (Java)**
   - Write `gradle_pipeline.rs` or `java_gradle.rs` with `fn run_gradle_tasks(config: &JavaConfig) -> Result<JavaProjectData, ...>`
   - Parse JUnit, PIT (and optionally Checkstyle) results into `JavaProjectData`.

4. **Rune Scripting**
   - Add `rune` and `rune_modules` to `Cargo.toml`.
   - Create a function to build a `Context`, install a module with `GradeResult`, `new_by_unit_test_grader(...)`, etc.
   - Write a minimal `.rn` script to test function calls.

5. **Final "grade" Subcommand**
   - Steps:
     1. Load config.
     2. Run appropriate pipeline based on `config.project.language`.
     3. Instantiate Rune with your "grading module".
     4. Compile and run the user's `.rn` script.
     5. Print or produce final outputs.

6. **Other Languages**
   - For `python_runner.rs`: `fn run_pytest(config: &PythonConfig) -> PyProjectData`.
   - Add a new grader function for Python tests if partial test scoring is needed.

7. **Advanced LLM or "SGLang"** (Optional)
   - Provide `generate_llm_feedback()` in Rust to call an external service (e.g., sglang server or OpenAI endpoint).
   - Register in the Rune module for dynamic feedback in `.rn` scripts.

8. **Self-Updating Mechanism**
   - Reuse the approach from `umm`: "check for updates" subcommand or check on each run.
   - Use the `self_update` crate to fetch releases from GitHub.

9. **Refinement & Testing**
   - Provide "sample labs" (like `arraylist-example-lab`) for local testing.
   - Confirm functionality in container or CI environment and ability to produce `results.json` recognized by Gradescope.

## 5. Practical Example for an Instructor or Developer

Instructor/developer steps:

1. Create `umm-next.toml`:
   ```toml
   [project]
   language = "java"

   [java]
   gradle_tasks = ["test", "pitest"]
   ```

2. Write `grade_script.rn`:
   ```rune
   fn main() {
       let tests = new_by_unit_test_grader("./build/test-results/test");
       let mutation = new_by_mutation_grader("./build/reports/pitest/mutations.xml");
       show_results([tests, mutation]);
   }
   ```

3. Students run:
   ```
   umm-next grade grade_script.rn
   ```
   - "umm-next" loads config, identifies `language=java`.
   - Calls `./gradlew test pitest`, parses results.
   - Instantiates Rune, runs `grade_script.rn`.
   - Script calls grader functions, prints table or writes Gradescope JSON.

4. Example output:
   ```
   ┌───────────────────────────────────┐
   │     Grading Overview              │
   ├───────────────────────────────────┤
   │ Requirement │    Grade    │ ...   │
   ...
   ```

---

## 6. **Why This Plan Works**

1. **Minimizes Disruption**  
   - We keep the same conceptual approach: a single CLI, one or more grader function calls, final table, JSON output.  
   - We only replace Rhai with Rune, and “javac” integration with Gradle.  
   - The plan is flexible enough to add Python or other languages without major rewrites.

2. **Extendable Architecture**  
   - New languages or new graders (Checkstyle doc-lint, coverage, advanced LLM-based feedback) can be added in smaller modules.  
   - figment config ensures we only run the pipeline needed.

3. **Consistent UX**  
   - Students see the same “local usage” experience. They can tie it to VSCode tasks exactly as before (like `"command": "umm-next", "args": ["grade", "grade_script.rn"]`).

4. **Time-Efficient**  
   - You can implement a minimal version in a few days:  
     - The Java pipeline for test + PIT, plus a basic Rune script.  
     - Then refine or add new graders.

---

## 7. **Conclusion**

With **this plan**, you:

- Maintain **familiar** naming and flows from the old `umm`.
- Provide **multi-language** readiness via a figment-based config (`umm-next.toml`).
- Switch from Rhai to Rune for **powerful** scripting, including potential advanced LLM logic.
- Integrate deeply with **Gradle** for Java, plus future expansions for Python, etc.
- Keep a **single self-updating** binary that **students** can run in local dev setups (VSCode tasks) or in CI/Gradescope for final grading.

### **Next Steps**:
1. Prototype the figment-based config reading.  
2. Implement a “java gradle pipeline,” parse JUnit/PIT outputs.  
3. Hook up a minimal Rune script with `new_by_unit_test_grader`.  
4. Confirm output in a real “example-lab” scenario.  
5. Optionally add doc-lint (Checkstyle) parsing or advanced LLM feedback.  

By following this strategy, you’ll have a robust, future-proof “umm-next” that **seamlessly** merges your previous autograder logic with the new Gradle + Rune approach and is well-prepared to handle expansions and improvements down the road.