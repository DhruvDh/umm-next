let project = new_java_project();

let req_1 = new_docs_grader()
    .project(project)
    .files(["DataStructures.LinkedList"])
    .out_of(60.0)
    .req_name("LinkedList JavaDoc")
    .penalty(6.0)
    .run();

let req_2 = new_by_unit_test_grader()
    .project(project)
    .test_files(["DataStructures.LinkedListTest"])
    .expected_tests([])
    .out_of(60.0)
    .req_name("LinkedList Unit Tests")
    .run();

let req_3 = new_by_hidden_test_grader()
    .url("https://ummfiles.fra1.digitaloceanspaces.com/grading_scripts%2Flists%2FLinkedListHiddenTest.java")
    .test_class_name("LinkedListHiddenTest")
    .out_of(180.0)
    .req_name("LinkedList Hidden Tests")
    .run();


let reqs = [req_1, req_2, req_3];

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

generate_feedback([req_1, req_2]);