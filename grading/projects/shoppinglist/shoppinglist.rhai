
let project = new_java_project();

let req_1 = new_docs_grader()
    .project(project)
    .files(["Shopping.ShoppingListArrayList"])
    .out_of(15.0)
    .penalty(1.5)
    .req_name("ShoppingListArrayList JavaDoc")
    .run();

let req_2 = new_by_unit_test_grader()
    .project(project)
    .test_files(["Shopping.ShoppingListArrayListTest"])
    .expected_tests(    [
        "Shopping.ShoppingListArrayListTest#testAdd1",
        "Shopping.ShoppingListArrayListTest#testAdd2",
        "Shopping.ShoppingListArrayListTest#testAdd3",
        "Shopping.ShoppingListArrayListTest#testAddExpands",
        "Shopping.ShoppingListArrayListTest#testRemove1",
        "Shopping.ShoppingListArrayListTest#testRemove2",
        "Shopping.ShoppingListArrayListTest#testRemove3",
        "Shopping.ShoppingListArrayListTest#testFind1",
        "Shopping.ShoppingListArrayListTest#testFind2",
        "Shopping.ShoppingListArrayListTest#testFind3",
        "Shopping.ShoppingListArrayListTest#testFind4",
        "Shopping.ShoppingListArrayListTest#testIndexOf1",
        "Shopping.ShoppingListArrayListTest#testIndexOf2",
        "Shopping.ShoppingListArrayListTest#testIndexOf3",
        "Shopping.ShoppingListArrayListTest#testIndexOf4",
        "Shopping.ShoppingListArrayListTest#testContains1",
        "Shopping.ShoppingListArrayListTest#testContains2",
        "Shopping.ShoppingListArrayListTest#testContains3",
        "Shopping.ShoppingListArrayListTest#testContains4",
        "Shopping.ShoppingListArrayListTest#testSize",
        "Shopping.ShoppingListArrayListTest#testIsEmpty",
    ])
    .out_of(15.0)
    .req_name("ShoppingListArrayList Unit Tests")
    .run();

let req_3 = new_by_hidden_test_grader()
    .url("https://ummfiles.fra1.digitaloceanspaces.com/grading_scripts%2Fprojects%2Fshoppinglist%2FShoppingListHiddenArrayListTest.java")
    .test_class_name("ShoppingListHiddenArrayListTest")
    .out_of(70.0)
    .req_name("ShoppingListArrayList Hidden Tests")
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

if (total != out_of) {
    generate_feedback([req_1, req_2]);
}