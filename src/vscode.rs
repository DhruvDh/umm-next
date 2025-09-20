#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

/// Enum for VSCode task's type.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Type {
    /// If shell is specified, the command is interpreted as a shell command
    /// (for example: bash, cmd, or PowerShell).
    Shell,
    ///  If process is specified, the command is interpreted as a process to
    /// execute.
    Process,
}

/// enum for VSCode task's arg quoting.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArgQuoting {
    /// escape strings
    Escape,
    /// ses the shell's strong quoting mechanism, which suppresses all
    /// evaluations inside the string. Under PowerShell and for shells under
    /// Linux and macOS, single quotes are used (`'`). For cmd.exe, `"` is used.
    Strong,
    /// Uses the shell's weak quoting mechanism, which still evaluates
    /// expression inside the string (for example, environment variables). Under
    /// PowerShell and for shells under Linux and macOS, double quotes are used
    /// (`"`). cmd.exe doesn't support weak quoting so VS Code uses `"` as well.
    Weak,
}

/// Struct for VSCode task's args.
#[derive(Serialize, Deserialize, TypedBuilder)]
pub struct Args {
    /// value of arg
    #[builder(default, setter(into))]
    value:   String,
    /// specifies how to escape the arg value.
    #[builder(default=ArgQuoting::Escape)]
    quoting: ArgQuoting,
}

/// Enum for VSCode task's dependsOrder.
#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum DependsOrder {
    /// In parallel with other tasks.
    Parallel,
    /// In sequence with other tasks.
    Sequence,
}

/// Struct for VSCode task's presentation.
#[derive(Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(doc)]
#[builder(field_defaults(default, setter(into)))]
pub struct Presentation {
    /// Controls whether the Integrated Terminal panel is brought to front.
    /// Valid values are:
    /// * `always` - The panel is always brought to front. This is the default.
    /// * `never` - The user must explicitly bring the terminal panel to the
    ///   front using the  **View** > **Terminal** command
    ///   (`kb(workbench.action.terminal.toggleTerminal)`).
    /// * `silent` - The terminal panel is brought to front only if the output
    ///   is not scanned for errors and warnings.
    #[serde(skip_serializing_if = "Option::is_none")]
    reveal:             Option<String>,
    /// Controls whether the Problems panel is revealed when running this task
    /// or not. Takes precedence over option `reveal`. Default is `never`.
    ///   * `always` - Always reveals the Problems panel when this task is
    ///     executed.
    ///   * `onProblem` - Only reveals the Problems panel if a problem is found.
    ///   * `never` - Never reveals the Problems panel when this task is
    ///     executed.
    #[serde(skip_serializing_if = "Option::is_none")]
    reveal_problems:    Option<String>,
    /// Controls whether the terminal is taking input focus or not. Default is
    /// `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    focus:              Option<bool>,
    /// Controls whether the executed command is echoed in the terminal. Default
    /// is `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    echo:               Option<bool>,
    /// Controls whether to show the "Terminal will be reused by tasks, press
    /// any key to close it" message.
    #[serde(skip_serializing_if = "Option::is_none")]
    show_reuse_message: Option<bool>,
    /// Controls whether the terminal instance is shared between task runs.
    /// Possible values are:
    ///   * `shared` - The terminal is shared and the output of other task runs
    ///     are added to the same terminal.
    ///   * `dedicated` - The terminal is dedicated to a specific task. If that
    ///     task is executed again, the terminal is reused. However, the output
    ///     of a different task is presented in a different terminal.
    ///   * `new` - Every execution of that task is using a new clean terminal.
    #[serde(skip_serializing_if = "Option::is_none")]
    panel:              Option<String>,
    /// Controls whether the terminal is cleared before this task is run.
    /// Default is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    clear:              Option<bool>,
    /// Controls whether the terminal the task runs in is closed when the task
    /// exits.

    #[serde(skip_serializing_if = "Option::is_none")]
    close:              Option<bool>,
    /// Controls whether the task is executed in a specific terminal group using
    /// split panes. Tasks in the same group (specified by a string value) will
    /// use split terminals to present instead of a new terminal panel.
    #[serde(skip_serializing_if = "Option::is_none")]
    group:              Option<bool>,
}

/// Struct for VSCode task's problem matcher.
#[derive(Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct ProblemMatcher {
    /// Controls if a problem reported on a text document is applied only to
    /// open, closed or all documents.
    /// Valid values are:
    ///  * `openDocuments` - Only applied to open documents.
    /// * `closedDocuments` - Only applied to closed documents.
    /// * `allDocuments` - Applied to all documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(default, setter(into))]
    apply_to:      Option<String>,
    /// Patterns to track the begin and end of a matcher active on a background
    /// task.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    background:    Option<String>,
    /// The name of a base problem matcher to use.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    base:          Option<String>,
    /// Defines how file names reported in a problem pattern should be
    /// interpreted. A relative fileLocation may be an array, where the second
    /// element of the array is the path the relative file location.
    /// Valid values are:
    ///  * `absolute` - File names are interpreted as absolute paths.
    /// * `relative` - File names are interpreted as relative paths.
    /// * `autoDetect` - automatically detects
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    file_location: Option<Vec<String>>,
    /// The owner of the problem inside Code. Can be omitted if base is
    /// specified. Defaults to 'external' if omitted and base is not specified.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    owner:         Option<String>,
    /// A problem pattern or the name of a contributed or predefined problem
    /// pattern. Can be omitted if base is specified.
    pattern:       Pattern,
    /// The default severity for captures problems. Is used if the pattern
    /// doesn't define a match group for severity.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    severity:      Option<String>,
    /// A human-readable string describing the source of this diagnostic, e.g.
    /// 'typescript' or 'super lint'.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    source:        Option<String>,
}

/// Struct for VSCode task's problem matcher's pattern.
#[derive(Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct Pattern {
    /// The match group index of the problem's code. Defaults to undefined
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    code:       Option<usize>,
    /// The match group index of the problem's line character. Defaults to 3
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    column:     Option<usize>,
    /// The match group index of the problem's end line character. Defaults to
    /// undefined
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    end_column: Option<usize>,
    /// The match group index of the problem's end line. Defaults to undefined
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line:   Option<usize>,
    /// The match group index of the filename. If omitted 1 is used.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    file:       Option<usize>,
    /// whether the pattern matches a location (file and line) or only a file.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    kind:       Option<String>,
    /// The match group index of the problem's line. Defaults to 2
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    line:       Option<usize>,
    /// The match group index of the problem's location. Valid location patterns
    /// are: (line), (line,column) and
    /// (startLine,startColumn,endLine,endColumn). If omitted (line,column) is
    /// assumed.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    location:   Option<String>,
    /// In a multi line matcher loop indicated whether this pattern is executed
    /// in a loop as long as it matches. Can only specified on a last pattern in
    /// a multi line pattern.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    r#loop:     Option<bool>,
    /// The match group index of the message. If omitted it defaults to 4 if
    /// location is specified. Otherwise it defaults to 5.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    message:    Option<usize>,
    /// The regular expression to find an error, warning or info in the output.
    #[builder(setter(into))]
    regexp:     String,
    /// The match group index of the problem's severity. Defaults to undefined
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    severity:   Option<usize>,
}

/// Struct to represent a VSCode task as JSON.
#[derive(Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    /// The task's label used in the user interface.
    label:           String,
    /// The task's type.
    #[builder(default=Some(Type::Shell), setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type:          Option<Type>,
    /// The actual command to execute.
    #[builder(default, setter(into))]
    command:         String,
    /// Any Windows specific properties. Will be used instead of the default
    /// properties when the command is executed on the Windows operating system.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    windows:         Option<String>,
    /// Defines to which group the task belongs. In the example, it belongs to
    /// the test group. Tasks that belong to the test group can be executed by
    /// running Run Test Task from the Command Palette.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    group:           Option<String>,
    /// Defines how the task output is handled in the user interface.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    presentation:    Option<Presentation>,
    /// Override the defaults for cwd (current working directory), env
    /// (environment variables), or shell (default shell). Options can be set
    /// per task but also globally or per platform. Environment variables
    /// configured here can only be referenced from within your task script or
    /// process and will not be resolved if they are part of your args, command,
    /// or other task attributes.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    options:         Option<String>,
    /// Arguments passed to the command when this task is invoked.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    args:            Option<Vec<Args>>,
    /// Either a string representing another task or an array of other tasks
    /// that this task depends on.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    depends_on:      Option<Vec<String>>,
    /// Run all dependsOn tasks in parallel.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    depends_order:   Option<DependsOrder>,
    /// An optional description of a task that shows in the Run Task quick pick
    /// as a detail.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    detail:          Option<String>,
    /// An optional icon path.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    icon:            Option<String>,
    /// Whether the executed task is kept alive and is running in the
    /// background.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    is_background:   Option<bool>,
    /// Any linux specific properties. Will be used instead of the default
    /// properties when the command is executed on the Linux operating system.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    linux:           Option<String>,
    /// Any macOS specific properties. Will be used instead of the default
    /// properties when the command is executed on the macOS operating system.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    osx:             Option<String>,
    /// The problem matcher(s) to use. Can either be a string or a problem
    /// matcher definition or an array of strings and problem matchers.
    #[builder(default=Some(Vec::new()))]
    problem_matcher: Option<Vec<ProblemMatcher>>,
    /// Whether the user is prompted when VS Code closes with a running task.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_on_close: Option<bool>,
    /// The task's run related options.
    /// Valid values are:
    /// * **reevaluateOnRerun**: Controls how variables are evaluated when a
    ///   task is executed through the **Rerun Last Task** command. The default
    ///   is `true`, meaning that variables will be reevaluated when a task is
    ///   rerun. When set to `false` the resolved variable values from the
    ///   previous run of the task will be used.
    /// * **runOn**: Specifies when a task is run.
    /// * `default` - The task will only be run when executed through the **Run
    ///   Task** command.
    /// * `folderOpen` - The task will be run when the containing folder is
    ///   opened. The first time you open a folder that contains a task with
    ///   `folderOpen`, you will be asked if you want to allow tasks to run
    ///   automatically in that folder. You can change your decision later using
    ///   the **Manage Automatic Tasks in Folder** command and selecting between
    ///   **Allow Automatic Tasks in Folder** and **Disallow Automatic Tasks in
    ///   Folder**.
    #[builder(default, setter(into))]
    #[serde(skip_serializing_if = "Option::is_none")]
    run_options:     Option<String>,
}

/// default run task action
fn run_task_action() -> Option<String> {
    Some("workbench.action.tasks.runTask".to_string())
}

/// default run task action
fn when_keybindings() -> Option<String> {
    Some("config:workspaceKeybindings.ummTasksKeys.enabled".to_string())
}

/// A struct to represent a keybinding for tasks in VSCode.
#[derive(Serialize, Deserialize, TypedBuilder)]
pub struct KeyBindings {
    /// The keybinding
    key:     String,
    /// The command to execute, defaults to `workbench.action.tasks.runTask`
    #[serde(default = "run_task_action")]
    #[builder(default, setter(into))]
    command: Option<String>,
    /// The command's arguments - name of task, etc.
    args:    String,
    /// when to activate keybinding
    #[serde(default = "when_keybindings")]
    #[builder(default, setter(into))]
    when:    Option<String>,
}

/// Enum to represent the type of a task input.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Input {
    /// Shows an input box to get a string from the user.
    PromptString {
        /// ID for input
        id:          String,
        /// Shown in the quick input, provides context for the input.
        description: String,
        /// Default value that will be used if the user doesn't enter something
        /// else.
        default:     String,
        ///  Set to true to input with a password prompt that will not show the
        /// typed value.
        password:    Option<bool>,
    },
    /// Shows a Quick Pick dropdown to let the user select from several options.
    PickString {
        /// ID for input
        id:          String,
        /// Shown in the quick input, provides context for the input.
        description: String,
        /// A list of strings to pick from.
        options:     Vec<String>,
        /// Default value that will be used if the user doesn't enter something
        /// else. It must be one of the option values.
        default:     String,
    },
}

/// Struct representing a tasks.json file
#[derive(Serialize, Deserialize, TypedBuilder)]
pub struct TasksFile {
    /// The tasks.json version.
    #[builder(default = "2.0.0".to_string())]
    version: String,
    /// The tasks.json tasks.
    #[builder(default = vec![])]
    tasks:   Vec<Task>,
    /// The tasks.json keybindings.
    #[builder(default = vec![])]
    inputs:  Vec<Input>,
}

/// Struct representing vscode settings.json file
/// Only the properties that we need.
#[derive(Serialize, Deserialize, TypedBuilder)]
pub struct SettingsFile<'a> {
    /// javac source path
    #[serde(rename = "java.project.sourcePaths")]
    java_source_path:     Vec<String>,
    /// javac target path
    #[serde(rename = "java.project.outputPath")]
    java_output_path:     String,
    /// javac classpath
    #[serde(rename = "java.project.referencedLibraries")]
    java_referenced_libs: Vec<String>,
    /// whether to use keybindings or not
    #[serde(rename = "workspaceKeybindings.ummTasksKeys.enabled")]
    #[builder(default = true)]
    keybindings_enabled:  bool,
    /// path to umm binary
    #[serde(rename = "ummBinaryPath")]
    umm_binary_path:      String,
    /// word wrap setting
    #[serde(rename = "editor.wordWrap")]
    #[builder(default = "on")]
    word_wrap:            &'a str,
    /// minimap setting
    #[serde(rename = "editor.minimap.enabled")]
    #[builder(default = false)]
    minimap:              bool,
}
