use std::fmt::Display;

use anyhow::{Context, Result};
use async_openai::types::ChatCompletionRequestMessage;
use tabled::Tabled;

#[derive(Clone, Default)]
/// A struct representing a grade
pub struct Grade {
    /// The actual grade received
    pub grade:  f64,
    /// The maximum grade possible
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

    /// a getter for the grade
    pub fn grade(&self) -> f64 {
        self.grade
    }

    /// a getter for the out_of
    pub fn out_of(&self) -> f64 {
        self.out_of
    }

    /// a setter for the grade
    pub fn set_grade(mut self, grade: f64) -> Self {
        self.grade = grade;
        self
    }

    /// a setter for the out_of
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.out_of = out_of;
        self
    }
}

impl Display for Grade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}/{:.2}", self.grade, self.out_of)
    }
}

#[derive(Tabled, Clone, Default)]
/// A struct to store grading results and display them
pub struct GradeResult {
    #[tabled(rename = "Requirement")]
    /// * `requirement`: refers to Requirement ID
    pub(crate) requirement: String,
    #[tabled(rename = "Grade")]
    /// * `grade`: grade received for above Requirement
    pub(crate) grade:       Grade,
    #[tabled(rename = "Reason")]
    /// * `reason`: the reason for penalties applied, if any
    pub(crate) reason:      String,
    #[tabled(skip)]
    /// * `prompt`: the prompt for the AI TA
    pub(crate) prompt:      Option<Vec<ChatCompletionRequestMessage>>,
}

impl GradeResult {
    /// a getter for Requirement
    pub fn requirement(&self) -> String {
        self.requirement.clone()
    }

    /// a setter for Requirement
    pub fn set_requirement(mut self, requirement: String) -> Self {
        self.requirement = requirement;
        self
    }

    /// a getter for Reason
    pub fn reason(&self) -> String {
        self.reason.clone()
    }

    /// a setter for Reason
    pub fn set_reason(mut self, reason: String) -> Self {
        self.reason = reason;
        self
    }

    /// a getter for the self.grade.grade
    pub fn grade(&self) -> f64 {
        self.grade.grade()
    }

    /// a getter for the self.grade.out_of
    pub fn out_of(&self) -> f64 {
        self.grade.out_of()
    }

    /// a setter for the self.grade.grade
    pub fn set_grade(mut self, grade: f64) -> Self {
        self.grade = self.grade.set_grade(grade);
        self
    }

    /// a setter for the self.grade.out_of
    pub fn set_out_of(mut self, out_of: f64) -> Self {
        self.grade = self.grade.set_out_of(out_of);
        self
    }

    /// a getter for the prompt
    pub fn prompt(&self) -> Option<Vec<ChatCompletionRequestMessage>> {
        self.prompt.clone()
    }

    /// a setter for the prompt
    pub fn set_prompt(mut self, prompt: Option<Vec<ChatCompletionRequestMessage>>) -> Self {
        self.prompt = prompt;
        self
    }

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
