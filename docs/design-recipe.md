# **Design Recipe for Java**

This structure provides a **systematic approach** to writing robust, maintainable Java methods. Each step builds on the previous, ensuring you solve the **right** problem with **clear data definitions**, thorough **tests**, and a well-organized **implementation**.

By following **all six steps**—including the often-overlooked **Step 0**—you avoid many common pitfalls like:

- Guessing data formats you never clarified
- Skipping key edge cases
- Having to rewrite code because of hidden assumptions or contradictions

> **Caution:** Skipping or glossing over these steps—especially **Step 0** (Restate the Problem) and **Step 1** (Data Definitions)—frequently leads to **major rewrites** when you discover the real requirements or constraints later.

## **Step 0. Understand & Restate the Problem**

1. **Purpose**  
   Ensure you genuinely know what needs to be solved or computed. **Misunderstanding** the core requirements is the #1 cause of extensive rewrites later.

2. **Activities**  
   - **Re-read** the problem statement or prompt carefully—likely more than once.  
   - **Paraphrase** it in your own words: identify **inputs**, **outputs**, constraints, and assumptions.  
   - List potential **edge cases** (e.g., “What if the input is `null`?” “What if the data structure is empty or partially filled?”).  
   - Whenever possible, ask clarifying questions (to a TA, instructor, or client). If you cannot, **document your assumptions explicitly**.

3. **Deliverables**  
   - A short **restatement** of the problem’s requirements and key goals.  
   - A summary of ambiguous details or **special conditions** you plan to handle.

<details>
<summary>Example Deliverable</summary>

**Short Restatement**  

- We want a method that, given a list of integer temperatures, computes the **average temperature**.  
- We assume the list is never null or empty; otherwise, we throw an `IllegalArgumentException`.  
- The method returns a `double` representing the computed average.

</details>

**Why It Matters**  

- Skipping Step 0 often leads to implementing the **wrong** solution or ignoring hidden constraints. You’ll either produce incorrect code or face a massive rewrite when you discover the real requirements—sometimes only after everything else is “done.”

## **Step 1. Data Definitions & Representations**

1. **Purpose**  
   Specify which Java types or classes the method will **use** or **produce**. Declare **preconditions** (what must hold before we use the data) and **postconditions** (what’s guaranteed after we finish) if relevant.

2. **Activities**  
   - Decide on the appropriate Java types (e.g., `int`, `String`, `List<String>`, or custom classes/records).  
   - If the data is non-trivial, define new classes (e.g., `public class Song { ... }`) with **private fields** and constraints (like rating ∈ [1..5]).  
   - Document **invariants** or constraints, turning them into **preconditions** for the code. (For instance: “`playCount ≥ 0`” or “`temperatures` must not be null.”)

3. **Deliverables**  
   - A concise description of each data structure’s fields and valid ranges.  
   - **Explicit** preconditions (like “list cannot be empty, must not exceed size 1000,” etc.).  
   - Any postconditions if relevant (like “after adding an item, the list’s size is exactly 1 greater”).

<details>
<summary>Example Deliverable</summary>

**Data Definition**  

- **Inputs**: `List<Integer> temperatures`  
- **Output**: `double` (the average temperature)  
- **Preconditions**:  
  - `temperatures` != null  
  - `temperatures.isEmpty() == false`  
- **Additional Constraints**:  
  - Negative numbers are allowed, but the list must have at least one element.  
  - The final average might be negative if all values are negative.

</details>

**Why It Matters**  

- Having a **well-defined** data structure with documented preconditions ensures no hidden “magic” assumptions (like “this list is always non-empty, right?”). This clarifies the code for both you and future maintainers.

## **Step 2. Method Signature & Purpose Statement**

1. **Purpose**  
   Provide a **contract** describing the method’s **signature** (name, parameters, return type) and the **intent** of the method, including how it handles errors (e.g., throwing exceptions when preconditions fail).

2. **Activities**  
   - Pick a method name and define the parameter/return types in Java.  
   - Write a **succinct** purpose statement (in Javadoc) that clarifies:  
     - **What** the method does  
     - **Why** or when it should be used  
     - **Preconditions** that the **caller** must meet  
     - **Postconditions** or guaranteed outcomes  
   - Indicate **expected exceptions** (`@throws SomeException`) if inputs violate these constraints.

3. **Deliverables**  
   - A Java method header, e.g. `public static double averageTemp(List<Integer> temps)`.  
   - A Javadoc block explaining valid parameters, the return value, and exceptions for invalid inputs.

<details>
<summary>Example Deliverable</summary>

```java
/**
 * Computes the average of a non-empty list of integer temperatures.
 *
 * @param temperatures A non-null, non-empty list of integers
 * @return The average temperature as a double
 * @throws IllegalArgumentException If temperatures is null or empty
 *
 * Precondition: temperatures != null && !temperatures.isEmpty()
 * Postcondition: Returned value is the arithmetic mean of all elements in 'temperatures'.
 */
public static double averageTemp(List<Integer> temperatures) {
    // ...
}
```

</details>

**Why It Matters**  

- A **clear signature and purpose** statement is how you anchor your method in the rest of the codebase. It’s your promise to users (or yourself) that, **if** the preconditions are met, the method will produce a certain result—or throw a known exception if not.

## **Step 3. Examples & Tests**

1. **Purpose**  
   Pin down the method’s behavior with real examples **before** or **during** coding. Then convert these examples into automated tests to ensure the behavior remains correct as you refine or refactor.

2. **Activities**  
   - Identify **typical inputs**, edge cases, and error cases.  
   - For each example, state the **expected output** clearly.  
   - Write **JUnit** (or another framework) tests verifying that the method returns or throws exactly what you documented.

3. **Deliverables**  
   - A **table** of test cases, showing input vs. expected output (plus rationale/notes).  
   - Automated test methods that cover **every row** in that table.

### **Using a Table-based Approach**  

A recommended format is:

| **Case Type**       | **Input**    | **Expected Output**           | **Notes / Rationale**                                      |
|---------------------|------------- |-------------------------------|------------------------------------------------------------|
| Normal usage        | `"Hello"`    | 5                             | `'H','e','l','l','o'` → 5 letters                         |
| Mixed content       | `"He11o!"`   | 3                             | Only `'H','e','o'` are letters; digits/punctuation don’t count |
| Empty string        | `""`         | 0                             | No letters at all                                         |
| Whitespace only     | `"    "`     | 0                             | Spaces aren’t letters                                     |
| Error scenario      | `null`       | `throws` `IllegalArgumentException` | Precondition: text can’t be null                          |

**Turning Table Rows into JUnit Tests**  

```java
@Test
public void testCountLettersNormal() {
    assertEquals(5, countLetters("Hello"));
}

@Test
public void testCountLettersMixed() {
    assertEquals(3, countLetters("He11o!"));
}

@Test
public void testCountLettersEmpty() {
    assertEquals(0, countLetters(""));
}

@Test
public void testCountLettersWhitespace() {
    assertEquals(0, countLetters("    "));
}

@Test
public void testCountLettersNull() {
    assertThrows(IllegalArgumentException.class, () -> countLetters(null));
}
```

**Why It Matters**  

- **Table-based** examples ensure you don’t forget crucial edge or error cases. Each row maps to a test, so you can systematically confirm whether the code meets your declared behavior.

## **Step 4. Skeleton / Method Template**

1. **Purpose**  
   Outline the **logical flow** of the solution (e.g., input validation → loop → results) **before** writing the complete code. This prevents mixing everything in one big, messy implementation.

2. **Activities**  
   - Write a **high-level** pseudocode or comment structure, referencing how you’ll handle each test scenario.  
   - Explicitly mark steps like “1) Validate inputs for null or empty,” “2) Initialize sum,” “3) Loop over data,” etc.  
   - Confirm these steps cover all your **test** scenarios (from Step 3).

3. **Deliverables**  
   - A commented skeleton in Java.  
   - No full logic yet—just placeholders or commented sections.

<details>
<summary>Example Deliverable</summary>

```java
public static double averageTemp(List<Integer> temperatures) {
    // Step 1: Validate inputs (throw if null or empty)
    // Step 2: Initialize sum = 0.0
    // Step 3: Loop through temperatures, accumulate sum
    // Step 4: Compute average = sum / size
    // Step 5: Return average
}
```

</details>

**Why It Matters**  

- A skeleton clarifies the structure. If you see a step is missing (e.g., you forgot to handle an “all negative” scenario or a “null” scenario?), it’s **easier to fix** at the outline stage than to refactor 50 lines of final code.

> **Common Mistake**: Novices skip the skeleton and jump into coding loops, conditionals, etc. **Result**: they realize too late they needed an extra check or branch. Reorganizing a big chunk of code is far more painful than adjusting an outline.

## **Step 5. Implementation, Testing & Reflection**

In reality, you often cycle through these sub-steps: write some code, run tests, discover new constraints or edge cases, refine data definitions, etc. **It’s normal** to revisit earlier steps if new information surfaces (e.g., adjusting Step 0’s restatement or Step 1’s data constraints).

### **5.1 Implementation**

- **Goal**: Write the actual Java code that satisfies the skeleton and respects the data definitions.  
- **Activities**:  
  1. Fill in each placeholder with working Java code.  
  2. Keep it readable—avoid excessive complexity.  
  3. Use helper methods if it clarifies the logic (e.g., `validateTemperatures(temps)`).

### **5.2 Testing & Refinement**

- **Goal**: Ensure the code meets **all** examples from Step 3. If a test fails, debug the mismatch.  
- **Activities**:  
  1. Run your entire test suite.  
  2. If any test fails, ask:  
     - Did we code incorrectly, or is the test (or assumption) wrong?  
     - Did we skip a precondition that the user might violate?  
  3. Fix the code **or** revise your steps if you discovered a new edge case that wasn’t in your original data definitions.

- **Deliverables**:  
  - A passing test suite covering normal, edge, and error cases.  
  - Possibly updated docstrings or data definitions if the new edge case changes assumptions.

### **5.3 Reflection**

- **Goal**: Check if you truly satisfied the original problem from Step 0. Consider performance, readability, and maintainability.  
- **Activities**:  
  1. Confirm you’re returning correct results under the preconditions.  
  2. Think about performance (O(n) vs. O(log n), etc.). For large data sets, do you need a more optimal approach?  
  3. Adjust or note improvements. If you adopt them, test again.

<details>
<summary>Example Deliverable</summary>

```java
public static double averageTemp(List<Integer> temperatures) {
    // Precondition checks
    if (temperatures == null || temperatures.isEmpty()) {
        throw new IllegalArgumentException("List cannot be null or empty.");
    }

    // Implementation
    double sum = 0.0;
    for (int temp : temperatures) {
        sum += temp;
    }
    double avg = sum / temperatures.size();

    // Reflection:
    // - Time complexity: O(n). For extremely large lists, consider parallel streams or chunking.

    return avg; // Postcondition: returns arithmetic mean of all valid elements
}

// Then run your JUnit tests from Step 3:
@Test
public void testAverageTempTypical() {
    // ...
}

@Test
public void testAverageTempEmptyList() {
    // ...
}

// etc.
```

All tests pass. We discovered no new constraints, so our data definition remains correct.
</details>

**Why It Matters**  

- “Implementation, Testing & Reflection” closes the loop. You finalize code, confirm correctness with automated tests, and **reflect** on performance or additional edge cases. If new requirements pop up (e.g., “maybe some temperatures come from a sensor array with invalid readings…”), it’s normal to revisit Steps 0–2.

## **Summary of All Steps**

1. **Step 0**: **Understand & Restate the Problem**  
2. **Step 1**: **Data Definitions & Representations**  
3. **Step 2**: **Method Signature & Purpose Statement**  
4. **Step 3**: **Examples & Tests**  
5. **Step 4**: **Skeleton / Method Template**  
6. **Step 5**: **Implementation, Testing & Reflection**

By following these steps **in order**—and being prepared to loop back if new info arises—you’ll build **clear, testable, and maintainable** methods. **Don’t skip** Step 0 or Step 1, and do not regard the table-based approach to testing as optional. Each step protects you from guesswork and silent breakage.

### **Final Note on Iteration**

- If you discover contradictions or overlooked scenarios, go back to **Step 0** (update your restatement) or Step 1–2 (update data definitions, constraints, or method purpose).  
- Re-run your tests after **every** change.  
- Keep docstrings and assumptions in sync with the actual code: this ensures your “design recipe” remains the single source of truth for the entire project.

**Best of luck—and remember: the biggest mistakes come from ignoring Step 0 and Step 1!**
