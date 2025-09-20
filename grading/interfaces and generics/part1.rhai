let project = new_project();

let req_1 = grade_docs(["Comparable"], project, 20, "1");

let req_2 = grade_by_hidden_tests(
    "https://www.dropbox.com/s/gcrsoao4gkydkdu/Part1Test.java?raw=1",
    "Part1Test",
    80.0,
    "2"
);

show_results([req_1, req_2]);