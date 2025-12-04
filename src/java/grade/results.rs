#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::fmt::Display;

use anyhow::{Context, Result};
use async_openai::types::chat::ChatCompletionRequestMessage;
use bon::Builder;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

#[derive(Clone, Default, Builder, Serialize, Deserialize)]
/// A struct representing a grade
pub struct Grade {
    /// The actual grade received
    #[builder(getter)]
    pub grade:  f64,
    /// The maximum grade possible
    #[builder(getter)]
    pub out_of: f64,
}

impl Grade {
    /// Creates a new grade -
    /// * `grade` - The actual grade received
    /// * `out_of` - The maximum grade possible
    pub fn new(grade: f64, out_of: f64) -> Self {
        Self { grade, out_of }
    }

    /// Creates a new grade from a string -
    /// * `grade_string` - A string in the format `grade/out_of`, eg. `10/20`
    pub fn grade_from_string(grade_string: String) -> Result<Grade> {
        let (grade, out_of) = grade_string.split_once('/').unwrap_or(("0", "0"));
        Ok(Grade::new(
            grade.parse::<f64>().context("Failed to parse grade")?,
            out_of.parse::<f64>().context("Failed to parse out of")?,
        ))
    }
}

impl Display for Grade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}/{:.2}", self.grade, self.out_of)
    }
}

#[derive(Tabled, Clone, Default, Builder, Serialize, Deserialize)]
#[builder(on(String, into))]
/// A struct to store grading results and display them
pub struct GradeResult {
    #[tabled(rename = "Requirement")]
    /// * `requirement`: refers to Requirement ID
    #[builder(getter)]
    pub(crate) requirement: String,
    #[tabled(rename = "Grade")]
    /// * `grade`: grade received for above Requirement
    #[builder(default)]
    #[builder(getter)]
    pub(crate) grade:       Grade,
    #[tabled(rename = "Reason")]
    /// * `reason`: the reason for penalties applied, if any
    #[builder(getter)]
    pub(crate) reason:      String,
    #[tabled(skip)]
    /// * `prompt`: the prompt for the AI TA
    #[builder(getter)]
    pub(crate) prompt:      Option<Vec<ChatCompletionRequestMessage>>,
}

impl GradeResult {
    /// Returns the underlying grade struct.
    pub fn grade_struct(&self) -> &Grade {
        &self.grade
    }

    /// Returns the numeric grade value.
    pub fn grade_value(&self) -> f64 {
        self.grade.grade
    }

    /// Returns the numeric out-of value.
    pub fn out_of_value(&self) -> f64 {
        self.grade.out_of
    }
}
