# Reading Sqlite databases

Below is an example Rust snippet using **rusqlite** to query Cosmic Ray’s tables—**mutation_specs**, **work_items**, and **work_results**—and then join them to get a summary of each mutation, its location, and the outcome. This code assumes:

1. You have a **Cargo.toml** with `rusqlite` as a dependency.
2. Your **session.sqlite** is in the same directory you run this from, or you pass the correct path to `Connection::open("path/to/session.sqlite")`.
3. You want to retrieve (for example) the fields: `module_path`, `operator_name`, `occurrence`, `start_pos_row`, `start_pos_col`, plus the mutation’s `worker_outcome` and `test_outcome` from **work_results**.

Feel free to rename fields or retrieve additional columns (e.g., `output`, `diff`) as needed.

---

## 1. `Cargo.toml`

```toml
[package]
name = "read_cosmic_ray"
version = "0.1.0"
edition = "2021"

[dependencies]
rusqlite = "0.29.0"
serde = "1.0"
serde_json = "1.0"

```

*(Including `serde`/`serde_json` if you want to parse the `operator_args` JSON. Omit them if you don’t need that.)*

---

## 2. Rust Code (e.g., `src/main.rs`)

```rust
use rusqlite::{Connection, Result, Row};
use std::path::Path;

// A struct capturing all relevant columns from the joined tables
#[derive(Debug)]
struct MutationSummary {
    module_path: String,
    operator_name: String,
    occurrence: i64,
    start_pos_row: i64,
    start_pos_col: i64,
    worker_outcome: String,
    test_outcome: String,
}

fn main() -> Result<()> {
    // 1. Open the database. Adjust path if needed.
    let db_path = Path::new("session.sqlite");
    let conn = Connection::open(db_path)?;

    // 2. Prepare a SQL statement that joins mutation_specs with work_results
    //    on job_id. We pull out the columns we want.
    //
    //    For example, you might want:
    //      - module_path, operator_name, occurrence, start_pos_row, start_pos_col
    //        from mutation_specs
    //      - worker_outcome, test_outcome from work_results
    //
    //    If you also want the job_id itself, or the raw diff, etc., you can
    //    select them too.
    let mut stmt = conn.prepare(
        r#"
        SELECT
            m.module_path,
            m.operator_name,
            m.occurrence,
            m.start_pos_row,
            m.start_pos_col,
            r.worker_outcome,
            r.test_outcome
        FROM mutation_specs AS m
        JOIN work_results AS r
          ON m.job_id = r.job_id
        ORDER BY m.module_path, m.start_pos_row, m.start_pos_col
        "#,
    )?;

    // 3. Map each row into a Rust struct
    let mutation_iter = stmt.query_map([], |row: &Row| {
        Ok(MutationSummary {
            module_path: row.get(0)?,      // m.module_path
            operator_name: row.get(1)?,    // m.operator_name
            occurrence: row.get(2)?,       // m.occurrence
            start_pos_row: row.get(3)?,    // m.start_pos_row
            start_pos_col: row.get(4)?,    // m.start_pos_col
            worker_outcome: row.get(5)?,   // r.worker_outcome
            test_outcome: row.get(6)?,     // r.test_outcome
        })
    })?;

    // 4. Print or otherwise process the results
    for mutation_res in mutation_iter {
        let mutation = mutation_res?;
        println!("{:?}", mutation);
    }

    Ok(())
}

```

### Potential Queries or Fields

- If you want to see the actual `output` or `diff` from `work_results`, just include them in the SELECT statement and add them to your struct.
- If you want the `operator_args` (JSON) from `mutation_specs`, you could parse it using `serde_json`:
    
    ```rust
    // Suppose you have:
    // operator_args JSON,
    // ...
    let operator_args: String = row.get(2)?;
    // Then parse:
    let parsed_args: serde_json::Value = serde_json::from_str(&operator_args).unwrap_or_else(|_| serde_json::Value::Null);
    
    ```
    
- The `work_items` table only has `job_id`; typically, you don’t need to join it unless you have additional data stored there. cosmic-ray uses it as a base reference table.

---

## 3. Running

If the code is in `src/main.rs`, do:

```bash
cargo run

```

It will open `session.sqlite`, run the query, and print each `MutationSummary` row. Adjust `ORDER BY` or `WHERE` clauses in the SQL if you need to filter or sort differently.

---

### In Summary

1. **Add `rusqlite`** to `Cargo.toml`.
2. **Check the actual table/column names** (as you did with `.schema`).
3. **Write a SELECT statement** that joins `mutation_specs` and `work_results` on `job_id`.
4. **Map rows into a struct** in Rust.
5. **Compile & run** to retrieve, filter, or display your Cosmic Ray mutation data in a typed, robust way.