let project = new_java_project();

let req_1 = new_docs_grader()
    .project(project)
    .files(["DataStructures.LinkedQueue"])
    .out_of(20.0)
    .req_name("JavaDoc for LinkedQueue")
    .penalty(3.0)
    .run();
    
let req_2 = new_by_hidden_test_grader()
    .url("https://ummfiles.fra1.digitaloceanspaces.com/grading_scripts%2Fqueues%2FLinkedQueueHiddenTest.java")
    .test_class_name("LinkedQueueHiddenTest")
    .out_of(80.0)
    .req_name("LinkedQueue Hidden Tests")
    .run();



let total = req_1.grade() + req_2.grade();
let out_of = req_1.out_of() + req_2.out_of();

show_results([req_1, req_2]);

if total > (0.7 * out_of) {
    print("p;" + total.to_int())
} else {
    print("np")
}

if (total != out_of) {
    generate_feedback([req_1, req_2]);
}