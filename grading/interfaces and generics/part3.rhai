let project = new_java_project();

let source_class = "UnitTesting.CoOrds";
let test_class = "UnitTesting.CoOrdsTest";
let req_1 = new_docs_grader()
    .project(project)
    .files([source_class])
    .out_of(5.0)
    .req_name(`9L. Javadoc for ${source_class}`)
    .penalty(1.0)
    .run();

let req_2 = if (
    project
        .identify(test_class)
        .test_methods()
        .len() > 0
    &&
    new_by_unit_test_grader()
        .project(project)
        .test_files([test_class])
        .expected_tests([])
        .out_of(60.0)
        .req_name(`9L. ${test_class} unit tests`)
        .run()
        .grade() == 60.0
) {
    new_unit_test_grader()
        .req_name(`9L. ${test_class} mutation tests`)
        .out_of(20.0)
        .target_test([test_class])
        .target_class([source_class])
        .run()
} else {
    new_grade_result()
        .grade(0.0)
        .requirement(`9L. ${test_class} mutation tests`)
        .out_of(20.0)
        .reason(`No test methods found in ${test_class}, or unit tests failed`)
};

let source_class = "UnitTesting.Student";
let test_class = "UnitTesting.StudentTest";
let req_3 = new_docs_grader()
    .project(project)
    .files([source_class])
    .out_of(5.0)
    .req_name(`9L. Javadoc for ${source_class}`)
    .penalty(1.0)
    .run();

let req_4 = if (
    project
        .identify(test_class)
        .test_methods()
        .len() > 0
    &&
    new_by_unit_test_grader()
        .project(project)
        .test_files([test_class])
        .expected_tests([])
        .out_of(60.0)
        .req_name(`9L. ${test_class} unit tests`)
        .run()
        .grade() == 60.0
) {
    new_unit_test_grader()
        .req_name(`9L. ${test_class} mutation tests`)
        .out_of(20.0)
        .target_test([test_class])
        .target_class([source_class])
        .run()
} else {
    new_grade_result()
        .grade(0.0)
        .requirement(`9L. ${test_class} mutation tests`)
        .out_of(20.0)
        .reason(`No test methods found in ${test_class}, or unit tests failed`)
};

let reqs = [req_1, req_2, req_3, req_4];

show_results(reqs);
generate_feedback(reqs);

if (
    project
        .identify("UnitTesting.StudentTest")
        .test_methods()
        .len() == 0
) {
    print("np");
} else if req_1.grade() >= 20 {
    print("p;" + (req_2.grade() + 20).to_int());
} else {
    print("np");
}