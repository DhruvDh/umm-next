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
                .severity(d)
                .line_number(l)
                .message(if d { format!("Error: {m}") } else { m })
                .build()
            }

        rule mutation_test_examined_path() -> Vec<String>
            = a:mutations_csv_word()? "/"? b:mutations_csv_word()? "/"?  c:mutations_csv_word()?
            {
                let mut res = vec![];
                if let Some(a) = a { res.push(a); }
                if let Some(b) = b { res.push(b); }
                if let Some(c) = c { res.push(c); }
                res
            }

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

                if test.len() >= 3 {
                    let raw_class = test.get(1).cloned().unwrap_or_default();
                    let raw_method = test.get(2).cloned().unwrap_or_default();

                    let splitter_class = if raw_class.contains("[runner:") { "[runner:" } else { "[class:" };
                    if let Some((_, rhs)) = raw_class.split_once(splitter_class) {
                        test_file_name = rhs.replace(']', "");
                    } else {
                        test_file_name = raw_class;
                    }

                    let splitter_method = if raw_method.contains("[test:") { "[test:" } else { "[method:" };
                    if let Some((_, rhs)) = raw_method.split_once(splitter_method) {
                        test_method_name = rhs.replace("()]", "");
                    } else {
                        test_method_name = raw_method;
                    }
                }

                let mutator = mutation
                    .to_string()
                    .split_once(".mutators.")
                    .map(|(_, r)| r.to_string())
                    .unwrap_or_else(|| mutation.to_string());

                MutationDiagnostic::builder()
                    .line_number(line_no)
                    .mutator(mutator)
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

#[cfg(test)]
mod tests {
    use super::parser;
    use crate::java::grade::DiagnosticSeverity;

    #[test]
    fn mutation_row_parses_none_marker() {
        let csv = "Foo,Bar,org.pitest.mutationtest.engine.gregor.mutators.MathMutator,method,42,\
                   SURVIVED,none";
        let diag = parser::mutation_report_row(csv).expect("parse mutation row without test info");
        let lr: crate::java::grade::LineRef = diag.clone().into();
        assert_eq!(lr.line_number, 42);
        assert_eq!(lr.file_name(), "Bar");
        assert_eq!(diag.result(), "SURVIVED");
        let snapshot = serde_json::to_value(&diag).unwrap();
        assert_eq!(snapshot["test_file_name"], "NA");
        assert_eq!(snapshot["test_method"], "None");
    }

    #[test]
    fn mutation_row_parses_class_and_method_hints() {
        let csv = "Foo,Source,org.pitest.mutationtest.engine.gregor.mutators.\
                   ConditionalsBoundaryMutator,method,7,KILLED,a/[class:MyTest]/[method:\
                   testAdds()]";
        let diag = parser::mutation_report_row(csv).expect("parse mutation row with hints");
        assert_eq!(diag.result(), "KILLED");
        let snapshot = serde_json::to_value(&diag).unwrap();
        assert_eq!(snapshot["test_file_name"], "MyTest");
        assert_eq!(snapshot["test_method"], "testAdds");
    }

    #[test]
    fn mutation_row_preserves_other_status() {
        let csv = "Foo,Source,org.pitest.mutationtest.engine.gregor.mutators.\
                   RemoveConditionalMutator,method,3,TIMED_OUT,none";
        let diag = parser::mutation_report_row(csv).expect("parse mutation row with other status");
        assert_eq!(diag.result(), "TIMED_OUT");
    }

    #[test]
    fn mutation_row_rejects_malformed_csv() {
        assert!(parser::mutation_report_row("too,few,columns").is_err());
    }

    #[test]
    fn javac_diag_parses_error_and_warning() {
        let error_line = "./Foo.java:12: error: missing semicolon";
        let warning_line = "./Foo.java:15: warning: unchecked call";

        let error_diag = parser::parse_diag(error_line).expect("parse javac error");
        assert_eq!(error_diag.file_name(), "Foo.java");
        assert_eq!(error_diag.path().display().to_string(), "./Foo.java");
        assert!(error_diag.severity().is_error());

        let warning_diag = parser::parse_diag(warning_line).expect("parse javac warning");
        assert_eq!(warning_diag.severity(), DiagnosticSeverity::Warning);
        let snapshot = serde_json::to_value(&warning_diag).unwrap();
        assert!(
            snapshot["message"]
                .as_str()
                .unwrap()
                .contains("unchecked call")
        );
    }

    #[test]
    fn javac_diag_rejects_invalid_line() {
        assert!(parser::parse_diag("not a diagnostic line").is_err());
    }
}
