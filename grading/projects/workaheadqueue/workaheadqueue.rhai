let project = new_java_project();

let req_1 = new_by_unit_test_grader()
    .project(project)
    .test_files(["DataStructures.WorkAheadQueueTest"])
    .expected_tests(    [
        "DataStructures.WorkAheadQueueTest#testEnqueue",
        "DataStructures.WorkAheadQueueTest#testIsEmpty",
        "DataStructures.WorkAheadQueueTest#testSize",
        "DataStructures.WorkAheadQueueTest#testFirst_int",
        "DataStructures.WorkAheadQueueTest#testFirst_0args",
        "DataStructures.WorkAheadQueueTest#testFirstThreeNodes",
        "DataStructures.WorkAheadQueueTest#testFirstThreeElements",
        "DataStructures.WorkAheadQueueTest#testDequeue_0args",
        "DataStructures.WorkAheadQueueTest#testDequeue_int",
        "DataStructures.WorkAheadQueueTest#testDequeue_int2",
        "DataStructures.WorkAheadQueueTest#testDequeue_int_ECE",
        "DataStructures.WorkAheadQueueTest#testDequeue_int_IAE",
    ])
    .out_of(30.0)
    .req_name("Student-written Tests")
    .run();

let req_2 = new_by_hidden_test_grader()
    .url("https://ummfiles.fra1.digitaloceanspaces.com/grading_scripts%2Fprojects%2Fworkaheadqueue%2FWorkAheadQueueHiddenTest.java")
    .test_class_name("WorkAheadQueueHiddenTest")
    .out_of(50.0)
    .req_name("Hidden Tests")
    .run();

  let reqs = [req_1, req_2];

show_results(reqs);

let total = 0.0;
let out_of = 0.0;
for req in reqs {
    total = total + req.grade();
    out_of = out_of + req.out_of();
}

if total > (0.7 * out_of) {
    print("p;" + total.to_int())
} else {
    print("np")
}

if (total != out_of) {
    generate_feedback(req_1);
}