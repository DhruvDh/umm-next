pub fn normalize_rune_output(input: String) -> String {
    // Expand escaped newlines and NBSPs so timestamps inside prompt JSON are on
    // their own lines.
    let expanded = input
        .replace("\\n", "\n")
        .replace(['\u{202f}', '\u{00a0}'], " ");
    // Drop spinner/backspace noise.
    let no_spinner = expanded.replace('\u{08}', "");

    no_spinner
        .lines()
        .filter_map(|line| {
            let pit_stripped = if let Some((_, rest)) = line.split_once("PIT >>") {
                format!("PIT >>{rest}")
            } else {
                line.to_string()
            };
            // Strip leading timestamps like "2:31:00 PM " or "14:31:00 " (with optional
            // AM/PM and space)
            let mut trimmed = pit_stripped.as_str();
            if trimmed.len() >= 8
                && trimmed.as_bytes().get(1) == Some(&b':')
                && trimmed.as_bytes().get(4) == Some(&b':')
            {
                // Skip HH:MM:SS
                if let Some(after_time) = trimmed.get(8..) {
                    let after_time = after_time.trim_start();
                    // Optionally skip AM/PM token
                    let after_ampm =
                        if after_time.starts_with("AM ") || after_time.starts_with("PM ") {
                            &after_time[3..]
                        } else {
                            after_time
                        };
                    trimmed = after_ampm.trim_start();
                }
            }

            // Drop unstable PIT line count summaries to prevent snapshot churn.
            if trimmed.starts_with(">> Ran ") && trimmed.contains(" tests (") {
                return None;
            }

            // Drop volatile timing lines that change run-to-run.
            if trimmed.contains("Slowest test (") || trimmed.contains("Largest test (") {
                return None;
            }

            // Drop noisy PIT minion stack traces; they vary by JVM and sandbox.
            if trimmed.contains("PIT >> INFO : MINION :") {
                return None;
            }

            Some(trimmed.to_string())
        })
        .collect::<Vec<_>>()
        .join("\n")
}
