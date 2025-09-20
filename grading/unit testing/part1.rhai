let project = new_java_project();

let req_1 = new_by_unit_test_grader()
    .project(project)
    .test_files(["ConcertTicket.WalletTest"])
    .expected_tests(
        [
            "ConcertTicket.WalletTest#testAdd",
            "ConcertTicket.WalletTest#testRemove",
            "ConcertTicket.WalletTest#testResize",
            "ConcertTicket.WalletTest#testGetSize",
            "ConcertTicket.WalletTest#testGetLength",
            "ConcertTicket.WalletTest#testToString",
        ]
    )
    .out_of(25.0)
    .req_name("1")
    .run();

show_results([req_1]);

if req_1.grade() < 25.0 {
    print("np");
} else {
    print("p;25");
}