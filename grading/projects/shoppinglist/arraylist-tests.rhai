let project = new_project();

let req_1 = grade_by_tests(
    [("Shopping.ShoppingListArrayListTest")],
    [
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
    ],
    project,
    1.5,
    "Student-written Tests",
);

let req_2 = grade_by_hidden_tests(
    "https://ummfiles.fra1.digitaloceanspaces.com/grading_scripts/projects/shoppinglist/ShoppingListHiddenArrayListTest.java",
    "ShoppingListHiddenArrayListTest",
    1.5,
    "Hidden Tests",
);

show_results([req_1, req_2]);