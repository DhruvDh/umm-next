# Coercing PyTest into giving us python error messages

Below is a discussion of how to make pytest output structured XML for arbitrary Python code (even when you don’t already have tests) and what happens when you have multiple files. We’ll also look more closely at the snippet you posted for syntax checks, how to extend it to multiple files, and some practical strategies to handle larger Python projects in an autograding context.

**1. Forcing Pytest to Emit JUnit-Style XML**

By default, **pytest** is designed to discover and run actual tests. If you run:

pytest --junitxml=report.xml

Pytest will:

1.	Look for files named test_*.py or _test.py within the current directory (or per its config).

2.	Try to import them (and transitively the code they import).

3.	Run any test functions/classes found.

If Python code in those files has syntax or import errors, the test run fails or errors out—and you’ll see an <error> element in the JUnit XML. However, if you just have “random scripts” with no test functions, pytest won’t normally pick them up, nor produce any structured result about them.

**1.1 Creating a “Meta-Test” to Import Each Script**

A workaround is to create a single test file—say test_import_scripts.py—that *explicitly imports* the scripts you want to check:

import pytest

# Suppose you have multiple .py files in the same directory.

# You can list them out manually or discover them dynamically (see below).

def test_import_script1():

import script1  # if script1.py has a syntax error, Pytest will throw an error here

def test_import_script2():

import script2

Now, if script1.py or script2.py has a syntax error, Pytest fails or errors during the import phase. In your report.xml, you’ll see something like:

<testsuite ...>

<testcase classname="test_import_scripts" name="test_import_script1">

<error message="SyntaxError: invalid syntax" type="SyntaxError">

...

</error>

</testcase>

...

</testsuite>

This leverages Pytest’s standard mechanism for collecting tests and importing modules. For multiple files, you can do something more dynamic:

import pytest

import glob

import os

import importlib

python_files = [f for f in glob.glob("*.py") if not f.startswith("test_")]

@pytest.mark.parametrize("pyfile", python_files)

def test_import(pyfile):

module_name = os.path.splitext(pyfile)[0]

importlib.import_module(module_name)

•	Any syntax error in the files you import will appear in the JUnit XML under an <error> block.

•	Note that if *every* file has no syntax error, you’ll just get passing tests in the XML.

•	If you have no real tests of functionality, you only know that “it imports,” i.e. it’s at least syntactically valid.

**Drawback**: This still lumps together everything under “test_import,” so you only see which file triggered the error. The error message is in the <error> block, but it may not be super granular in how it’s reported.

**2. Handling Runtime Errors with Pytest**

If a script raises a runtime error upon import (e.g., code that runs at the top level, maybe dividing by zero or referencing a bad variable), you’ll also get a <error> in the JUnit XML. If the code only fails at function-call time, you can refine your “meta-tests” to call specific functions. For instance:

import pytest

import script1  # Suppose script1.py defines some function

def test_script1_main():

script1.main()  # If this raises an unhandled runtime error, we get an error block in JUnit XML

**3. What If You Don’t Have or Want Tests at All?**

Pytest is fundamentally a *test runner*, so if your code is not structured into tests, you’re effectively hacking in a “test” that just tries to import or run each file. This can still yield a JUnit XML with <error> tags for syntax and runtime issues, but you might find it a bit forced if you truly just have “random scripts” and no test logic.

When you only care about syntax checks or basic run checks:

1.	**Compile+Check Script**: The snippet you provided is a good approach to catch syntax errors in a structured JSON.

2.	**Pytest**: If you later want a JUnit XML, you can read that JSON, transform it into JUnit XML, or just run a “meta-test” that tries to import each script.

**4. Multiple Python Files and the Snippet You Posted**

You have a snippet that compiles a single file:

import json

import sys

filename = sys.argv[1]

try:

with open(filename, "r", encoding="utf-8") as f:

source = f.read()

code_obj = compile(source, filename, "exec")

print(json.dumps({"status": "ok"}))

except SyntaxError as e:

print(json.dumps({

"status": "error",

"type": "SyntaxError",

"message": str(e),

"lineno": e.lineno,

"offset": e.offset,

"text": e.text

}))

sys.exit(1)

**4.1 Extending to Multiple Files**

You could easily extend that snippet to handle a list of files, e.g.:

import json

import sys

files = sys.argv[1:]

results = []

for filename in files:

try:

with open(filename, "r", encoding="utf-8") as f:

source = f.read()

compile(source, filename, "exec")

results.append({

"filename": filename,

"status": "ok"

})

except SyntaxError as e:

results.append({

"filename": filename,

"status": "error",

"type": "SyntaxError",

"message": str(e),

"lineno": e.lineno,

"offset": e.offset,

"text": e.text

})

print(json.dumps(results, indent=2))

# If you want a non-zero exit code if ANY file fails:

if any(r["status"] == "error" for r in results):

sys.exit(1)

Then call it:

python syntax_check.py file1.py file2.py file3.py ...

You’ll get a JSON array with the results for each file. If you want to track runtime errors, you’d need a separate step to actually run the code (perhaps via exec or a function call).

**Pros**:

•	A straightforward JSON structure that you can parse in Rust (no guesswork from Pytest’s JUnit format).

•	Doesn’t rely on test discovery.

**Cons**:

•	This only checks for syntax errors. If you want to see if the file runs or has runtime errors, you need more logic.

**5. Typical Strategy in an Autograder with Multiple Files / Projects**

1.	**Syntax/Style Check**

•	Possibly run a separate “syntax check” script that compiles each file.

•	Or run linting tools (e.g., pylint --output-format=json) on the entire directory.

•	Collect structured results.

2.	**Import Check**

•	If you want to see if the code at least imports without runtime errors in top-level code, you can either:

•	(a) script that tries importlib.import_module(...) for each .py file

•	(b) rely on the Pytest collection phase (meta-test).

3.	**Run Actual Tests**

•	If you have real tests (like test_*.py), Pytest produces JUnit XML.

•	If you don’t, you could create a “wrapper test” that tries to run or call each script’s main function. This yields <error> entries if something crashes.

4.	**Aggregate Output**

•	If you prefer a single final “grade report,” you can parse JSON from Step 1 or 2, parse the Pytest JUnit XML from Step 3, and merge them.

•	You can store it in your own format or produce a single JSON/YAML with details about syntax errors, linter warnings, test results, etc.

**6. Conclusion**

•	**Can we force pytest to produce JUnit XML for arbitrary scripts?**

- Yes, but you must wrap each script in some “test” or “import test,” so Pytest sees them. Then it will produce <error> entries for syntax or runtime issues. This is a bit hacky if you don’t have real unit tests, but it’s feasible.

•	**What about the compile snippet for multiple files?**

- You can easily loop over each file and store results in one JSON array. This is often simpler if you want a consistent, *stable* format for autograding. Pytest JUnit XML is stable enough for tests, but *less* straightforward if you just want syntax-check results for random scripts.

•	**Python projects**

- In a full project with multiple modules, you can still do the same approach. If you have real tests, Pytest is designed to discover them. If not, you can either (1) add a meta-test to import the modules or (2) run a custom “compile-and-check” script. Both ways produce structured data you can parse in Rust.

Ultimately, many autograding platforms do a mix: they rely on Pytest to produce JUnit XML for test outcomes **and** run a separate step for syntax or lint checks. If you want a single tool for *all* random scripts, a short Python script that compiles each file to produce JSON is often the easiest to control and extend.