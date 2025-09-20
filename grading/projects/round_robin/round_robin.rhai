let project = new_project();

let req_1 = grade_by_tests(
    [("DataStructures.CircularDoublyLinkedListTest")],
    [
        "DataStructures.CircularDoublyLinkedListTest#testFirst1",
        "DataStructures.CircularDoublyLinkedListTest#testFirst2",
        "DataStructures.CircularDoublyLinkedListTest#testCurrent",
        "DataStructures.CircularDoublyLinkedListTest#testLast",
        "DataStructures.CircularDoublyLinkedListTest#testNext",
        "DataStructures.CircularDoublyLinkedListTest#testIsEmpty",
        "DataStructures.CircularDoublyLinkedListTest#testPrintList",
        "DataStructures.CircularDoublyLinkedListTest#testAddFirst",
        "DataStructures.CircularDoublyLinkedListTest#testAddLast",
        "DataStructures.CircularDoublyLinkedListTest#testAddAfter",
        "DataStructures.CircularDoublyLinkedListTest#testRemove1",
        "DataStructures.CircularDoublyLinkedListTest#testRemove2",
        "DataStructures.CircularDoublyLinkedListTest#testRemove3",
        "DataStructures.CircularDoublyLinkedListTest#testRemoveFirst1",
        "DataStructures.CircularDoublyLinkedListTest#testRemoveFirst2",
        "DataStructures.CircularDoublyLinkedListTest#testRemoveLast1",
        "DataStructures.CircularDoublyLinkedListTest#testRemoveLast2",
        "DataStructures.CircularDoublyLinkedListTest#testContains1",
        "DataStructures.CircularDoublyLinkedListTest#testContains2",
        "DataStructures.CircularDoublyLinkedListTest#testFind1",
        "DataStructures.CircularDoublyLinkedListTest#testFind2",
        "DataStructures.CircularDoublyLinkedListTest#testGetNode1",
        "DataStructures.CircularDoublyLinkedListTest#testGetNode2",
        "DataStructures.CircularDoublyLinkedListTest#testGetNode3",
        "DataStructures.CircularDoublyLinkedListTest#testGet1",
        "DataStructures.CircularDoublyLinkedListTest#testGet2",
        "DataStructures.CircularDoublyLinkedListTest#testGet3",
        "DataStructures.CircularDoublyLinkedListTest#testSet1",
        "DataStructures.CircularDoublyLinkedListTest#testSet2",
        "DataStructures.CircularDoublyLinkedListTest#testSet3",
        "DataStructures.CircularDoublyLinkedListTest#testSize",
    ],
    project,
    30.0,
    "Student-written Tests",
);

let req_2 = grade_by_hidden_tests(
    "https://ummfiles.fra1.digitaloceanspaces.com/grading_scripts/projects/round_robin/CircularDoublyLinkedListTest.java",
    "CircularDoublyLinkedListHiddenTest",
    50.0,
    "Hidden Tests",
);

show_results([req_1, req_2]);