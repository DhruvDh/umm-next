let project = new_java_project();

let req_1 = new_docs_grader()
    .project(project)
    .files(["pyramid_scheme.LinkedTree"])
    .out_of(10.0)
    .req_name("1")
    .penalty(3.0)
    .run();

let req_2 = new_by_unit_test_grader()
    .project(project)
    .test_files(["pyramid_scheme.LinkedTreeTest"])
    .expected_tests([
        "pyramid_scheme.LinkedTreeTest#testGetRootElement",
        "pyramid_scheme.LinkedTreeTest#testAddChild",
        "pyramid_scheme.LinkedTreeTest#testFindNode",
        "pyramid_scheme.LinkedTreeTest#testContains",
        "pyramid_scheme.LinkedTreeTest#testSize",
    ])
    .out_of(20.0)
    .req_name("2")
    .run();

let req_3 = new_unit_test_grader()
    .req_name("2")
    .out_of(20.0)
    .target_test(["pyramid_scheme.LinkedTreeTest"])
    .target_class(["pyramid_scheme.LinkedTree"])
    .excluded_methods([])
    .avoid_calls_to([])
    .run();

let req_4 = new_docs_grader()
    .project(project)
    .files(["pyramid_scheme.PyramidScheme"])
    .out_of(10.0)
    .req_name("3")
    .penalty(3.0)
    .run();

let req_5 = new_by_unit_test_grader()
    .project(project)
    .test_files(["pyramid_scheme.PyramidSchemeTest"])
    .expected_tests([
        "pyramid_scheme.PyramidSchemeTest#testWhoBenefits",
        "pyramid_scheme.PyramidSchemeTest#testAddChild",
        "pyramid_scheme.PyramidSchemeTest#testInitiateCollapse",
    ])
    .out_of(30.0)
    .req_name("3")
    .run();

let req_6 = new_by_hidden_test_grader()
    .url("https://www.dropbox.com/s/47jd1jru1f1i0cc/ABCTest.java?raw=1")
    .test_class_name("ABCTest")
    .out_of(30.0)
    .req_name("4")
    .run();

let reqs = [req_1, req_2, req_3, req_4, req_5, req_6];

// arguements: 
// - array of grade results
show_results(reqs);

let total = 0.0;
let out_of = 0.0;
for req in reqs {
    total = total + req.grade();
    out_of = out_of + req.out_of();
}
if total > (0.6 * out_of) {
    print(total + ";p")
} else {
    print(total + ";f")
}

let tests = project.identify("LinkedTreeTest").query(`
(method_declaration
	(modifiers
        (annotation
            name: (identifier) @annotation
            arguments: (annotation_argument_list)
        )
    )
    name: (identifier) @name
)

(method_declaration
	(modifiers
	(marker_annotation
    	name: (identifier) @annotation)
    )
    name: (identifier) @name
    (#eq? @annotation "Test")
)
`);

for t in tests {
    print(t.name);
}

// ╭──────────────────────────────────────────────────────────╮
// │                  SAMPLE SCRIPT OUTPUT                    │
// ╰──────────────────────────────────────────────────────────╯
// ┌────────────────────────────────────────────────────────────┬
// │        Check javadoc for pyramid_scheme.LinkedTree         │
// ├────────────────────────────────────────────────────────────┼
// │           File           │ Line │         Message          │
// ├──────────────────────────┼──────┼──────────────────────────┤
// │ ./src/pyramid_scheme/Lin │  14  │   no main description    │
// │       kedTree.java       │      │                          │
// ├──────────────────────────┼──────┼──────────────────────────┤
// │ ./src/pyramid_scheme/Lin │  15  │ no description for @para │
// │       kedTree.java       │      │            m             │
// ├──────────────────────────┼──────┼──────────────────────────┤
// │ ./src/pyramid_scheme/Lin │  29  │ no description for @para │
// │       kedTree.java       │      │            m             │
// ├──────────────────────────┼──────┼──────────────────────────┤
// │ ./src/pyramid_scheme/Lin │  56  │  Error: unknown tag: T   │
// │       kedTree.java       │      │                          │
// ├──────────────────────────┼──────┼──────────────────────────┤
// │ ./src/pyramid_scheme/Lin │  72  │ no description for @thro │
// │       kedTree.java       │      │            ws            │
// ├──────────────────────────┼──────┼──────────────────────────┤
// │ ./src/pyramid_scheme/Lin │ 251  │ no description for @para │
// │       kedTree.java       │      │            m             │
// ├──────────────────────────┼──────┼──────────────────────────┤
// │                     -18 due to 6 nits                      │
// └────────────────────────────────────────────────────────────┴

// Running Mutation tests -
// 11:37:54 PM PIT >> INFO : Verbose logging is disabled. If you encounter a problem, please enable it before reporting an issue.
// 11:37:54 PM PIT >> INFO : Incremental analysis reduced number of mutations by 0
// 11:37:54 PM PIT >> INFO : Created  1 mutation test units in pre scan
// 11:37:54 PM PIT >> INFO : Sending 1 test classes to minion
// 11:37:54 PM PIT >> INFO : Sent tests to minion
// 11:37:54 PM PIT >> SEVERE : Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testSize(pyramid_scheme.LinkedTreeTest)]] did not pass without mutation.
// 11:37:54 PM PIT >> SEVERE : Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testAddChild(pyramid_scheme.LinkedTreeTest)]] did not pass without mutation.
// 11:37:54 PM PIT >> SEVERE : Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testFindNode(pyramid_scheme.LinkedTreeTest)]] did not pass without mutation.
// 11:37:54 PM PIT >> SEVERE : Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testContains(pyramid_scheme.LinkedTreeTest)]] did not pass without mutation.
// 11:37:54 PM PIT >> SEVERE : Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testGetRootElement(pyramid_scheme.LinkedTreeTest)]] did not pass without mutation.
// 11:37:54 PM PIT >> INFO : Calculated coverage in 0 seconds.
// 11:37:54 PM PIT >> SEVERE : Tests failing without mutation: 
// Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testSize(pyramid_scheme.LinkedTreeTest)]]
// Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testAddChild(pyramid_scheme.LinkedTreeTest)]]
// Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testFindNode(pyramid_scheme.LinkedTreeTest)]]
// Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testContains(pyramid_scheme.LinkedTreeTest)]]
// Description [testClass=pyramid_scheme.LinkedTreeTest, name=[engine:junit-vintage]/[runner:pyramid_scheme.LinkedTreeTest]/[test:testGetRootElement(pyramid_scheme.LinkedTreeTest)]]
// Exception in thread "main" org.pitest.help.PitHelpError: 5 tests did not pass without mutation when calculating line coverage. Mutation testing requires a green suite.
// See http://pitest.org for more details.
// 	at org.pitest.coverage.execute.DefaultCoverageGenerator.verifyBuildSuitableForMutationTesting(DefaultCoverageGenerator.java:115)
// 	at org.pitest.coverage.execute.DefaultCoverageGenerator.calculateCoverage(DefaultCoverageGenerator.java:97)
// 	at org.pitest.coverage.execute.DefaultCoverageGenerator.calculateCoverage(DefaultCoverageGenerator.java:52)
// 	at org.pitest.mutationtest.tooling.MutationCoverage.runAnalysis(MutationCoverage.java:148)
// 	at org.pitest.mutationtest.tooling.MutationCoverage.runReport(MutationCoverage.java:138)
// 	at org.pitest.mutationtest.tooling.EntryPoint.execute(EntryPoint.java:129)
// 	at org.pitest.mutationtest.tooling.EntryPoint.execute(EntryPoint.java:57)
// 	at org.pitest.mutationtest.commandline.MutationCoverageReport.runReport(MutationCoverageReport.java:98)
// 	at org.pitest.mutationtest.commandline.MutationCoverageReport.main(MutationCoverageReport.java:45)
// /
// ┌────────────────────────────────────────────────────────────┬
// │       Check javadoc for pyramid_scheme.PyramidScheme       │
// ├────────────────────────────────────────────────────────────┼
// │           File           │ Line │         Message          │
// ├──────────────────────────┼──────┼──────────────────────────┤
// │ ./src/pyramid_scheme/Pyr │  10  │ Error: unknown tag: Pers │
// │     amidScheme.java      │      │            on            │
// ├──────────────────────────┼──────┼──────────────────────────┤
// │ ./src/pyramid_scheme/Pyr │  18  │        no comment        │
// │     amidScheme.java      │      │                          │
// ├──────────────────────────┼──────┼──────────────────────────┤
// │ ./src/pyramid_scheme/Pyr │  19  │        no comment        │
// │     amidScheme.java      │      │                          │
// ├──────────────────────────┼──────┼──────────────────────────┤
// │ ./src/pyramid_scheme/Pyr │ 165  │ no description for @thro │
// │     amidScheme.java      │      │            ws            │
// ├──────────────────────────┼──────┼──────────────────────────┤
// │ ./src/pyramid_scheme/Pyr │ 241  │ no description for @retu │
// │     amidScheme.java      │      │            rn            │
// ├──────────────────────────┼──────┼──────────────────────────┤
// │                     -15 due to 5 nits                      │
// └────────────────────────────────────────────────────────────┴

// ┌─────────────────────────────────────────────────────┬
// │                  Grading Overview                   │
// ├─────────────────────────────────────────────────────┼
// │ Requirement │   Grade    │          Reason          │
// ├─────────────┼────────────┼──────────────────────────┤
// │      1      │    0/10    │        See above.        │
// ├─────────────┼────────────┼──────────────────────────┤
// │      2      │ 0.00/20.00 │   - 0/5 tests passing.   │
// ├─────────────┼────────────┼──────────────────────────┤
// │      2      │    0/20    │ Something went wrong whi │
// │             │            │ le running mutation test │
// │             │            │       s, skipping.       │
// ├─────────────┼────────────┼──────────────────────────┤
// │      3      │    0/10    │        See above.        │
// ├─────────────┼────────────┼──────────────────────────┤
// │      3      │ 0.00/30.00 │   - 0/3 tests passing.   │
// ├─────────────┼────────────┼──────────────────────────┤
// │      4      │ 0.00/30.00 │   - 0/5 tests passing.   │
// ├─────────────┼────────────┼──────────────────────────┤
// │                 Total: 0.00/120.00                  │
// └─────────────────────────────────────────────────────┴

// 0.0;f
// testGetRootElement
// testAddChild
// testFindNode
// testContains
// testSize
