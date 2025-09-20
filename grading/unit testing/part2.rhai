let project = new_java_project();

let req_1 = new_by_unit_test_grader()
    .project(project)
    .req_name("1")
    .out_of(10.0)
    .test_files(["Calculator.CalculatorTest"])
    .expected_tests([
            "Calculator.CalculatorTest#testAdd",
            "Calculator.CalculatorTest#testSubtract",
            "Calculator.CalculatorTest#testMultiply",
            "Calculator.CalculatorTest#testDivide"
        ]
    )
    .run();

let req_2 = new_unit_test_grader()
    .req_name("2")
    .out_of(40.0)
    .target_test(["Calculator.CalculatorTest"])
    .target_class(["Calculator.Calculator"])
    .run();

let reqs = [req_1, req_2];

show_results(reqs);
generate_feedback(reqs);

let total = 0.0;
let out_of = 0.0;

for req in reqs {
    total = total + req.grade();
    out_of = out_of + req.out_of();
}

if total < 50.0 {
    print("np");
} else {
    print("p;50");
}