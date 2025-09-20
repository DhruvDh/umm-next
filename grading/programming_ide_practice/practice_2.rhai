let project = new_java_project();

let req_1 = new_by_hidden_test_grader()
    .url("https://www.dropbox.com/s/rdekvma3lx2gz17/ProgrammingPractice2Test.java?raw=1")
    .test_class_name("ProgrammingPractice2Test")
    .out_of(70.0)
    .req_name("Hidden Tests")
    .run();

let reqs = [req_1];

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
    generate_feedback(reqs);
}