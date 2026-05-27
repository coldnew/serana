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
    /// Function definition: `function name; body; end` or `name() { body; }`
    FunctionDef(FunctionDef),
    /// For loop: `for var in list; body; end`
    ForLoop(ForLoop),
    /// While loop: `while cond; body; end`
    WhileLoop(WhileLoop),
    /// If statement: `if cond; then body; elif cond; else body; end`
    If(IfStatement),
    /// Switch statement: `switch $var; case pattern; body; end`
    Switch(SwitchStatement),
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

/// Parts of a word
#[derive(Debug, Clone, PartialEq)]
pub enum WordPart {
    /// Literal text
    Literal(String),
    /// Variable expansion: `$VAR` or `${VAR}`
    Variable(String),
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
    pub on_event: Option<String>,
    pub on_variable: Option<String>,
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

/// Switch statement
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchStatement {
    pub value: Word,
    pub cases: Vec<CaseBranch>,
}

/// Case branch in switch
#[derive(Debug, Clone, PartialEq)]
pub struct CaseBranch {
    pub patterns: Vec<Word>,
    pub body: Vec<Command>,
}

impl fmt::Display for Word {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for part in &self.parts {
            match part {
                WordPart::Literal(s) => write!(f, "{}", s)?,
                WordPart::Variable(v) => write!(f, "${{{}}}", v)?,
                WordPart::SingleQuoted(s) => write!(f, "'{}'", s)?,
                _ => write!(f, "...")?,
            }
        }
        Ok(())
    }
}
