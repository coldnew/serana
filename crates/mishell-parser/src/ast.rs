use std::fmt;

/// A complete shell command pipeline
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub body: CommandBody,
    pub background: bool,
}

/// The main command structure
#[derive(Debug, Clone, PartialEq)]
pub enum CommandBody {
    /// Simple command: `ls -la /tmp`
    Simple(SimpleCommand),
    /// Pipeline: `cmd1 | cmd2 | cmd3`
    Pipeline(Pipeline),
    /// Logical AND: `cmd1 && cmd2`
    And(Box<CommandBody>, Box<CommandBody>),
    /// Logical OR: `cmd1 || cmd2`
    Or(Box<CommandBody>, Box<CommandBody>),
    /// Sequence: `cmd1 ; cmd2`
    Sequence(Box<CommandBody>, Box<CommandBody>),
    /// Subshell: `(cmd)`
    Subshell(Box<CommandBody>),
    /// Group: `{ cmd; }`
    Group(Box<CommandBody>),
    /// Function definition: `name() { body; }` or `function name { body; }`
    FunctionDef(FunctionDef),
    /// For loop: `for var in list; do body; done`
    ForLoop(ForLoop),
    /// While loop: `while cond; do body; done`
    WhileLoop(WhileLoop),
    /// Until loop: `until cond; do body; done`
    UntilLoop(UntilLoop),
    /// If statement: `if cond; then body; elif cond; then body; else body; fi`
    If(IfStatement),
    /// Case statement: `case word in pattern) body ;; esac`
    Case(CaseStatement),
    /// Return statement
    Return(Option<Word>),
}

/// A simple command with redirects
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleCommand {
    pub redirects: Vec<Redirect>,
    pub assignments: Vec<Assignment>,
    pub words: Vec<Word>,
}

/// A pipeline of commands
#[derive(Debug, Clone, PartialEq)]
pub struct Pipeline {
    pub commands: Vec<SimpleCommand>,
    pub negated: bool, // `! cmd1 | cmd2`
}

/// A word which can contain expansions
#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub parts: Vec<WordPart>,
}

/// Parameter expansion operation type
#[derive(Debug, Clone, PartialEq)]
pub enum ParamOp {
    /// `${var:-default}` - use default if unset or null
    UseDefault(String),
    /// `${var:=default}` - assign default if unset or null
    AssignDefault(String),
    /// `${var:+value}` - use alternative value if set and non-null
    UseAlternative(String),
    /// `${var:?error}` - error if unset or null
    ShowError(String),
    /// `${var:+value}` with no colon - use alternative if set (even if null)
    UseAlternativeIfSet(String),
    /// `${#var}` - string length
    StringLength,
    /// `${var#pattern}` - remove shortest prefix match
    RemovePrefixShortest(String),
    /// `${var##pattern}` - remove longest prefix match
    RemovePrefixLongest(String),
    /// `${var%pattern}` - remove shortest suffix match
    RemoveSuffixShortest(String),
    /// `${var%%pattern}` - remove longest suffix match
    RemoveSuffixLongest(String),
    /// `${var/pattern/replacement}` - replace first match
    ReplaceFirst(String, String),
    /// `${var//pattern/replacement}` - replace all matches
    ReplaceAll(String, String),
    /// `${var,}` - lowercase first char
    LowercaseFirst,
    /// `${var,,}` - lowercase all
    LowercaseAll,
    /// `${var^}` - uppercase first char
    UppercaseFirst,
    /// `${var^^}` - uppercase all
    UppercaseAll,
    /// `${var:start:length}` - substring
    Substring(usize, Option<usize>),
}

/// Parts of a word
#[derive(Debug, Clone, PartialEq)]
pub enum WordPart {
    /// Literal text
    Literal(String),
    /// Variable expansion: `$VAR`
    Variable(String),
    /// Parameter expansion: `${var:-default}`, `${#var}`, etc.
    ParamExpansion {
        name: String,
        op: ParamOp,
    },
    /// Command substitution: `$(cmd)` or `` `cmd` ``
    CommandSub(CommandBody),
    /// Arithmetic expansion: `$((expr))`
    Arithmetic(String),
    /// Glob pattern: `*`, `?`, `[...]`
    Glob(GlobPattern),
    /// Tilde expansion: `~` or `~user`
    Tilde(Option<String>),
    /// Escape sequence: `\x`
    Escape(char),
    /// Double-quoted string
    DoubleQuoted(Vec<WordPart>),
    /// Single-quoted string
    SingleQuoted(String),
}

/// Glob patterns
#[derive(Debug, Clone, PartialEq)]
pub enum GlobPattern {
    /// `*` - matches any string
    Star,
    /// `?` - matches single char
    Question,
    /// `[...]` - character class
    Class(String),
    /// Literal glob text
    Literal(String),
}

/// I/O Redirect
#[derive(Debug, Clone, PartialEq)]
pub struct Redirect {
    pub fd: Option<u8>,
    pub op: RedirectOp,
    pub target: RedirectTarget,
}

/// Redirect operators
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectOp {
    /// `>` - output
    Output,
    /// `>>` - append
    Append,
    /// `<` - input
    Input,
    /// `>&` - duplicate output
    DupOutput,
    /// `<&` - duplicate input
    DupInput,
    /// `&>` - redirect both
    BothOutput,
    /// `&>>` - append both
    BothAppend,
    /// `<<<` - here string
    HereString,
    /// `<<` - here doc
    HereDoc,
    /// `<<-` - here doc (strip tabs)
    HereDocStrip,
}

/// Redirect target
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectTarget {
    /// File path
    File(Word),
    /// File descriptor
    Fd(u8),
    /// Here doc content
    HereDoc(String),
}

/// Variable assignment
#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub name: String,
    pub value: Word,
}

/// Function definition
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub body: Vec<Command>,
}

/// For loop
#[derive(Debug, Clone, PartialEq)]
pub struct ForLoop {
    pub variable: String,
    pub list: Vec<Word>,
    pub body: Vec<Command>,
}

/// While loop
#[derive(Debug, Clone, PartialEq)]
pub struct WhileLoop {
    pub condition: Vec<Command>,
    pub body: Vec<Command>,
}

/// Until loop
#[derive(Debug, Clone, PartialEq)]
pub struct UntilLoop {
    pub condition: Vec<Command>,
    pub body: Vec<Command>,
}

/// If statement
#[derive(Debug, Clone, PartialEq)]
pub struct IfStatement {
    pub condition: Vec<Command>,
    pub then_body: Vec<Command>,
    pub elif_branches: Vec<ElifBranch>,
    pub else_body: Option<Vec<Command>>,
}

/// Elif branch
#[derive(Debug, Clone, PartialEq)]
pub struct ElifBranch {
    pub condition: Vec<Command>,
    pub body: Vec<Command>,
}

/// Case statement (bash `case ... esac`)
#[derive(Debug, Clone, PartialEq)]
pub struct CaseStatement {
    pub value: Word,
    pub cases: Vec<CaseItem>,
}

/// Case item in case statement
#[derive(Debug, Clone, PartialEq)]
pub struct CaseItem {
    pub patterns: Vec<Word>,
    pub body: Vec<Command>,
}

impl fmt::Display for Word {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for part in &self.parts {
            match part {
                WordPart::Literal(s) => write!(f, "{}", s)?,
                WordPart::Variable(v) => write!(f, "${}", v)?,
                WordPart::SingleQuoted(s) => write!(f, "'{}'", s)?,
                _ => write!(f, "...")?,
            }
        }
        Ok(())
    }
}
