#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use crate::java::grade::{JavacDiagnostic, LineRef, MutationDiagnostic};

peg::parser! {
    /// includes some useful grammars for parsing JUNit/javac/pitest outputs.
    pub grammar parser() for str {
        /// matches any sequence of 1 or more numbers
        rule number() -> u32
            = n:$(['0'..='9']+) {? n.parse().or(Err("u32")) }

        /// matches any number of whitespace characters
        rule whitespace() = quiet!{[' ' | '\n' | '\t' | '\r']+}

        /// matches the keyword "tests successful"
        rule successful_tests()
            = " tests successful"

        /// matches the keyword "tests found"
        rule found_tests()
            = " tests found"

        /// parses and returns the number of tests passed
        pub rule num_tests_passed() -> u32
            = "[" whitespace()? l:number() successful_tests() whitespace()? "]" { l }

        /// parses and returns the number of tests found
        pub rule num_tests_found() -> u32
            = "[" whitespace()? l:number() found_tests() whitespace()? "]" { l }

        /// matches any path separator, hopefully cross-platform
        rule path_separator() =
            whitespace()?
            "."?
            "/" / "\\" / "\\\\"
            whitespace()?

        /// matches any sequence of upper and lowercase alphabets
        // TODO: support drive letters (e.g., `C:`) by allowing ':' once we have
        // windows-specific javac fixtures and tests.
        rule word() -> String
            = whitespace()?
                w:[
                    'a'..='z' |
                    'A'..='Z' |
                    '0'..='9' |
                    '-' | '.' | ' ' |
                    '[' | ']' | '_'
                ]+
                whitespace()?
            { w.iter().collect::<String>() }

        /// matches any sequence of upper and lowercase alphabets
        rule mutations_csv_word() -> String
            = whitespace()?
                w:[
                    'a'..='z' |
                    'A'..='Z' |
                    '0'..='9' |
                    '-' | '.' | ' ' |
                    '[' | ']' | ':' |
                    '<' | '>' | '_' |
                    '(' | ')'
                ]+
                whitespace()?
            { w.iter().collect::<String>() }

        /// matches any valid path, hopefully
        rule path() -> String
            = whitespace()?
              path_separator()?
              p:(word() ++ path_separator())
              whitespace()?
            { p.iter().fold(String::new(), |acc, w| format!("{acc}/{w}")) }

        /// matches line numbers (colon followed by numbers, eg. :23)
        rule line_number() -> u32
            = ":" n:number() ":" whitespace()? { n }

        /// matches "error" or "warning", returns true if error
        rule diag_type() -> bool
            = whitespace()?
              a:"error"? b:"warning"?
              ":"
              whitespace()?
            { a.is_some() }

        /// matches anything, placed where diagnostic should be
        rule diagnostic() -> String
            = a:([_]+)
            { a.iter().collect::<String>() }

        /// parses the first line of a javac diagnostic message and returns a `JavacDiagnostic`
        pub rule parse_diag() -> JavacDiagnostic
            = p:path() l:line_number() d:diag_type() m:diagnostic()
            {
                let p = std::path::PathBuf::from(p);
            let name = p
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| p.display().to_string());
            let display_path = format!(".{}", p.display());

            JavacDiagnostic::builder()
                .path(display_path)
                .file_name(name)
                .severity(d.into())
                .line_number(l)
                .message(if d { format!("Error: {m}") } else { m })
                .build()
            }

        rule mutation_test_examined_path() -> Vec<String>
            = segs:(mutations_csv_word() ** "/")
            { segs }

        rule mutation_test_examined_none() -> &'input str
            = $("none")

        /// parses one row of mutation report
        pub rule mutation_report_row() -> MutationDiagnostic
            = file_name:word()
              ","
              source_file_name:word()
              ","
              mutation:word()
              ","
              source_method:mutations_csv_word()
              ","
              line_no:number()
              ","
              result:word()
              ","
              test_method:mutation_test_examined_path()?
              whitespace()?
                {
                // Be lenient with the optional last CSV column.
                let test = test_method.unwrap_or_else(std::vec::Vec::new);
                let mut test_file_name = String::from("NA");
                let mut test_method_name = String::from("None");

        // Prefer explicit class/method markers if present; otherwise fall back to last
        // two segments. Normalize method names to strip trailing parens.
        for seg in &test {
            if let Some((_, rhs)) = seg.split_once("[class:") {
                test_file_name = rhs.trim_end_matches(']').to_string();
            } else if let Some((_, rhs)) = seg.split_once("[runner:") {
                test_file_name = rhs.trim_end_matches(']').to_string();
            }
        }

        for seg in &test {
            if let Some((_, rhs)) = seg.split_once("[method:") {
                test_method_name = rhs
                    .trim_end_matches(']')
                    .trim_end_matches(')')
                    .trim_end_matches('(')
                    .to_string();
                break;
            } else if let Some((_, rhs)) = seg.split_once("[test:") {
                test_method_name = rhs
                    .trim_end_matches(']')
                    .trim_end_matches(')')
                    .trim_end_matches('(')
                    .to_string();
                break;
            }
        }

        if test_file_name == "NA" && test.len() >= 2 {
            test_file_name = test.get(test.len() - 2).cloned().unwrap_or_default();
        }
        if test_method_name == "None" && !test.is_empty() {
            let fallback = test.last().cloned().unwrap_or_default();
            if fallback.eq_ignore_ascii_case("none") {
                test_method_name = "None".to_string();
            } else {
                test_method_name = fallback
                    .trim_end_matches(']')
                    .trim_end_matches(')')
                    .trim_end_matches('(')
                    .to_string();
            }
        }

                MutationDiagnostic::builder()
                    .line_number(line_no)
                    .mutator(mutation)
                    .source_file_name(source_file_name)
                    .source_method(source_method)
                    .test_file_name(test_file_name)
                    .test_method(test_method_name)
                    .result(result)
                    .build()
            }

            /// Parses a word in a JUnit stacktrace
            rule junit_stacktrace_word() -> String
                = whitespace()?
                w:[
                    'a'..='z' |
                    'A'..='Z' |
                    '0'..='9' |
                    '-' | '.' | ' ' |
                    '[' | ']' | '/' |
                    '>' | '=' | '$'
                ]+
                whitespace()?
            { w.iter().collect::<String>() }

            /// Parses a filename from a JUnit stacktrace
            rule junit_stacktrace_filename() -> String
                = whitespace()?
                w:[
                    'a'..='z' |
                    'A'..='Z' |
                    '0'..='9' |
                    '-' | '_' | '$'
                ]+
                ".java:"
                whitespace()?
            { w.iter().collect::<String>() }


            /// Parses a LineRef from a JUnit stacktrace
            pub rule junit_stacktrace_line_ref() -> LineRef
                = whitespace()?
                junit_stacktrace_word()*
                whitespace()?
                "("
                c:junit_stacktrace_filename()
                d:number()
                whitespace()?
                ")"
                whitespace()?
                {
                    LineRef { line_number: d as usize, file_name: c }
                }
    }
}
