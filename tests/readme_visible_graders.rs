use std::path::PathBuf;

use umm::java::{
    grade::{
        diff::DiffGrader,
        docs::DocsGrader,
        query::{Query, QueryGrader},
        tests::ByUnitTestGrader,
    },
    paths::project_paths,
    project::Project,
};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/java/readme-all")
}

fn jar_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("jar_files")
}

#[tokio::test]
async fn readme_visible_graders_succeed() -> anyhow::Result<()> {
    let paths = project_paths()
        .root_dir(fixture_root())
        .lib_dir(jar_dir())
        .build();

    let project = Project::from_paths(paths)?;

    let docs = DocsGrader::builder()
        .project(project.clone())
        .files(["Main"])
        .out_of(5.0)
        .req_name("docs")
        .penalty(1.0)
        .build()
        .run()
        .await?;

    let diff = DiffGrader::builder()
        .project(project.clone())
        .file("Main")
        .req_name("diff")
        .out_of(5.0)
        .cases([("Hello from Rune\n", None::<String>)])
        .build()
        .run()
        .await?;

    let visible_tests = ByUnitTestGrader::builder()
        .project(project.clone())
        .test_files(["MainTest"])
        .expected_tests(["MainTest#greets", "MainTest#sums"])
        .req_name("visible tests")
        .out_of(5.0)
        .build()
        .run()
        .await?;

    let query = {
        let q = Query::new()
            .set_query("((for_statement) @loop)".to_string())
            .set_capture("loop".to_string());

        QueryGrader::builder()
            .project(project)
            .file("Main")
            .queries([q])
            .req_name("query")
            .out_of(5.0)
            .reason("Should contain a for loop")
            .build()
            .run()?
    };

    for result in [&docs, &diff, &visible_tests, &query] {
        assert_eq!(result.grade_struct().grade, result.grade_struct().out_of);
    }

    Ok(())
}
