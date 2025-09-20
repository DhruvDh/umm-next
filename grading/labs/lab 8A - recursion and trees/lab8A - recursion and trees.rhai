let project = new_project();

let req_1 = grade_by_tests(
    [("DataStructures.LinkedBinaryTreeTest")],
    [],
    project,
    40.0,
    "Student-written Tests",
);

let req_2 = grade_by_hidden_tests(
    "https://ummfiles.fra1.digitaloceanspaces.com/grading_scripts/labs/Lab8ALinkedBInaryTreeTestHidden.java",
    "Lab8ALinkedBInaryTreeTestHidden",
    60.0,
    "Hidden Tests",
);

show_results([req_1, req_2]);