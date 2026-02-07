use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use serde_json::json;
use umm::python::{Project, grade::CodeReviewGrader, paths::ProjectPaths};

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("python")
        .join(name)
}

fn project(name: &str) -> Project {
    let root = fixture_root(name);
    let paths = ProjectPaths::from_parts(root, None, None, None, None, None, None);
    Project::from_paths(paths).expect("build project")
}

fn mock_completion_payload(content: &str) -> String {
    json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "created": 0,
        "model": "test-model",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }
        ],
        "usage": {
            "prompt_tokens": 1,
            "completion_tokens": 1,
            "total_tokens": 2
        }
    })
    .to_string()
}

fn read_http_request(stream: &mut TcpStream) {
    let mut buffer = Vec::new();
    let mut headers_end = None;
    let mut content_length = 0usize;

    loop {
        let mut chunk = [0u8; 1024];
        let bytes_read = stream.read(&mut chunk).expect("read request");
        if bytes_read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..bytes_read]);

        if headers_end.is_none()
            && let Some(pos) = buffer.windows(4).position(|window| window == b"\r\n\r\n")
        {
            headers_end = Some(pos + 4);
            let headers = String::from_utf8_lossy(&buffer[..pos + 4]).to_ascii_lowercase();
            for line in headers.lines() {
                if let Some(rest) = line.strip_prefix("content-length:") {
                    content_length = rest.trim().parse::<usize>().unwrap_or(0);
                    break;
                }
            }
        }

        if let Some(end) = headers_end
            && buffer.len() >= end + content_length
        {
            break;
        }
    }
}

fn start_mock_openai(responses: Vec<String>) -> (String, thread::JoinHandle<usize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    listener
        .set_nonblocking(true)
        .expect("set nonblocking listener");
    let address = listener.local_addr().expect("listener addr");

    let handle = thread::spawn(move || {
        let mut sent = 0usize;
        let deadline = Instant::now() + Duration::from_secs(15);

        while sent < responses.len() && Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    read_http_request(&mut stream);
                    let payload = mock_completion_payload(&responses[sent]);
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: \
                         {}\r\nConnection: close\r\n\r\n{}",
                        payload.len(),
                        payload
                    );
                    stream
                        .write_all(response.as_bytes())
                        .expect("write mock response");
                    sent += 1;
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(err) => panic!("mock accept failed: {err}"),
            }
        }

        sent
    });

    (format!("http://{address}/v1"), handle)
}

async fn run_code_review(project: Project, requirement: &str) -> umm::python::grade::GradeResult {
    CodeReviewGrader::builder()
        .project(project)
        .files(["main.py"])
        .req_name(requirement)
        .out_of(2.0)
        .build()
        .run()
        .await
        .expect("run code review grader")
}

#[tokio::test]
async fn code_review_grader_uses_pass_fail_and_retry_gates() {
    let responses = vec![
        r#"{"pass":true,"feedback_markdown":"Looks good.","reasons":[]}"#.to_string(),
        r#"{"pass":false,"feedback_markdown":"Needs changes.","reasons":["missing clarity"]}"#
            .to_string(),
        "not-json".to_string(),
        r#"{"pass":true,"feedback_markdown":"Recovered after retry.","reasons":[]}"#.to_string(),
        "still-not-json".to_string(),
        "still-not-json-again".to_string(),
        "} malformed-order {".to_string(),
        "still-not-json-final".to_string(),
    ];
    let (endpoint, handle) = start_mock_openai(responses);

    // SAFETY: tests intentionally control process env for config loading.
    unsafe {
        std::env::set_var("OPENAI_ENDPOINT", endpoint);
        std::env::set_var("OPENAI_API_KEY_SLO", "test-key");
        std::env::set_var("OPENAI_MODEL", "test-model");
    }

    let pass = run_code_review(project("diff-ok"), "pass").await;
    assert_eq!(pass.grade_value(), 2.0);
    assert!(pass.reason().contains("Looks good."));

    let fail = run_code_review(project("diff-ok"), "fail").await;
    assert_eq!(fail.grade_value(), 0.0);
    assert!(fail.reason().contains("LLM pass/fail gate failed"));

    let recovered = run_code_review(project("diff-ok"), "recovered").await;
    assert_eq!(recovered.grade_value(), 2.0);
    assert!(recovered.reason().contains("Recovered after retry."));

    let malformed_twice = run_code_review(project("diff-ok"), "malformed-twice").await;
    assert_eq!(malformed_twice.grade_value(), 0.0);
    assert!(
        malformed_twice
            .reason()
            .contains("Failed to parse structured code-review decision after one retry.")
    );

    let malformed_order = run_code_review(project("diff-ok"), "malformed-order").await;
    assert_eq!(malformed_order.grade_value(), 0.0);
    assert!(
        malformed_order
            .reason()
            .contains("Invalid JSON object bounds in model response: start index")
    );

    let request_count = handle.join().expect("join mock server");
    assert_eq!(request_count, 8, "expected one request per pass/fail and two per retry case");
}
