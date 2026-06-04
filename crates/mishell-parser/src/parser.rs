use crate::ast::*;
use crate::lexer::{Lexer, Token};
use anyhow::{anyhow, Result};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        while !self.at_end() {
            self.skip_sep();
            if self.at_end() {
                break;
            }
            commands.push(self.parse_command()?);
        }
        Ok(commands)
    }

    fn parse_command(&mut self) -> Result<Command> {
        let body = self.parse_command_body()?;
        let background = if self.check(&Token::Ampersand) {
            self.advance();
            true
        } else {
            false
        };
        Ok(Command { body, background })
    }

    fn parse_command_body(&mut self) -> Result<CommandBody> {
        match self.current() {
            Some(Token::Function) => return self.parse_function_def_keyword(),
            Some(Token::For) => return self.parse_for_loop(),
            Some(Token::While) => return self.parse_while_loop(),
            Some(Token::Until) => return self.parse_until_loop(),
            Some(Token::If) => return self.parse_if_statement(),
            Some(Token::Case) => return self.parse_case_statement(),
            Some(Token::LeftParen) => return self.parse_subshell(),
            Some(Token::LeftBrace) => return self.parse_group(),
            Some(Token::Return) => return self.parse_return(),
            _ => {}
        }

        // Check for name() { ... } function syntax
        if self.is_function_def() {
            return self.parse_function_def_parens();
        }

        let mut left = self.parse_pipeline()?;

        loop {
            if self.check(&Token::And) {
                self.advance();
                let right = self.parse_pipeline()?;
                left = CommandBody::And(Box::new(left), Box::new(right));
            } else if self.check(&Token::Or) {
                self.advance();
                let right = self.parse_pipeline()?;
                left = CommandBody::Or(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_pipeline(&mut self) -> Result<CommandBody> {
        let negated = if self.check(&Token::Bang) {
            self.advance();
            true
        } else {
            false
        };

        let mut commands = vec![self.parse_simple_command()?];

        while self.check(&Token::Pipe) {
            self.advance();
            self.skip_sep();
            commands.push(self.parse_simple_command()?);
        }

        if commands.len() == 1 && !negated {
            Ok(CommandBody::Simple(commands.into_iter().next().unwrap()))
        } else {
            Ok(CommandBody::Pipeline(Pipeline { commands, negated }))
        }
    }

    fn parse_simple_command(&mut self) -> Result<SimpleCommand> {
        let mut cmd = SimpleCommand {
            redirects: Vec::new(),
            assignments: Vec::new(),
            words: Vec::new(),
        };

        while !self.at_end() && !self.is_command_terminator() {
            if self.is_redirect() {
                cmd.redirects.push(self.parse_redirect()?);
            } else if self.is_assignment() {
                cmd.assignments.push(self.parse_assignment()?);
            } else {
                cmd.words.push(self.parse_word()?);
            }
        }

        Ok(cmd)
    }

    fn parse_redirect(&mut self) -> Result<Redirect> {
        let fd = self.extract_fd();
        let op = self.parse_redirect_op()?;
        self.advance();
        let target = self.parse_redirect_target(&op)?;
        Ok(Redirect { fd, op, target })
    }

    fn extract_fd(&mut self) -> Option<u8> {
        if let Some(Token::Word(s)) = self.current() {
            if s.len() == 1 && s.chars().next().unwrap().is_ascii_digit() {
                let fd = s.chars().next().unwrap() as u8 - b'0';
                self.advance();
                return Some(fd);
            }
        }
        None
    }

    fn parse_redirect_op(&mut self) -> Result<RedirectOp> {
        match self.current() {
            Some(Token::RedirectOut) => Ok(RedirectOp::Output),
            Some(Token::RedirectAppend) => Ok(RedirectOp::Append),
            Some(Token::RedirectIn) => Ok(RedirectOp::Input),
            Some(Token::RedirectDupOut) => Ok(RedirectOp::DupOutput),
            Some(Token::RedirectDupIn) => Ok(RedirectOp::DupInput),
            Some(Token::RedirectBoth) => Ok(RedirectOp::BothOutput),
            Some(Token::RedirectBothAppend) => Ok(RedirectOp::BothAppend),
            Some(Token::HereString) => Ok(RedirectOp::HereString),
            Some(Token::HereDoc) => Ok(RedirectOp::HereDoc),
            Some(Token::HereDocStrip) => Ok(RedirectOp::HereDocStrip),
            _ => Err(anyhow!("Expected redirect operator")),
        }
    }

    fn parse_redirect_target(&mut self, op: &RedirectOp) -> Result<RedirectTarget> {
        match op {
            RedirectOp::HereDoc | RedirectOp::HereDocStrip => {
                let delimiter = self.expect_word()?;
                let mut content = String::new();
                if self.check(&Token::Newline) {
                    self.advance();
                }
                loop {
                    let mut line = String::new();
                    while !self.at_end() && !self.check(&Token::Newline) {
                        if let Some(Token::Word(s)) = self.current() {
                            if s == &delimiter {
                                break;
                            }
                            if !line.is_empty() {
                                line.push(' ');
                            }
                            line.push_str(s);
                            self.advance();
                        } else if let Some(Token::SingleQuoted(s)) = self.current() {
                            if !line.is_empty() {
                                line.push(' ');
                            }
                            line.push_str(s);
                            self.advance();
                        } else if let Some(Token::DoubleQuoted(s)) = self.current() {
                            if !line.is_empty() {
                                line.push(' ');
                            }
                            line.push_str(s);
                            self.advance();
                        } else {
                            break;
                        }
                    }

                    if self.check_word(&delimiter) {
                        self.advance();
                        break;
                    }

                    if self.at_end() {
                        return Err(anyhow!("Unterminated here document"));
                    }

                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(&line);

                    if self.check(&Token::Newline) {
                        self.advance();
                    }
                }
                Ok(RedirectTarget::HereDoc(content))
            }
            _ => {
                let word = self.parse_word()?;
                Ok(RedirectTarget::File(word))
            }
        }
    }

    fn parse_assignment(&mut self) -> Result<Assignment> {
        let word = self.expect_word()?;
        let eq_pos = word
            .find('=')
            .ok_or_else(|| anyhow!("Expected assignment"))?;
        let name = word[..eq_pos].to_string();
        let value_str = &word[eq_pos + 1..];

        if !value_str.is_empty() {
            Ok(Assignment {
                name,
                value: Word {
                    parts: vec![WordPart::Literal(value_str.to_string())],
                },
            })
        } else {
            let value = match self.current() {
                Some(Token::DollarParen)
                | Some(Token::DollarBrace)
                | Some(Token::DollarArith)
                | Some(Token::SingleQuoted(_))
                | Some(Token::DoubleQuoted(_))
                | Some(Token::Backtick)
                | Some(Token::Word(_)) => self.parse_word()?,
                _ => Word { parts: vec![] },
            };
            Ok(Assignment { name, value })
        }
    }

    fn parse_word(&mut self) -> Result<Word> {
        let mut parts = Vec::new();

        match self.current() {
            Some(Token::Word(s)) => {
                if s.starts_with('$') && s.len() > 1 {
                    parts.push(WordPart::Variable(s[1..].to_string()));
                } else {
                    parts.push(WordPart::Literal(s.clone()));
                }
                self.advance();
            }
            Some(Token::SingleQuoted(s)) => {
                parts.push(WordPart::SingleQuoted(s.clone()));
                self.advance();
            }
            Some(Token::DoubleQuoted(s)) => {
                parts.push(WordPart::DoubleQuoted(Self::parse_double_quoted_parts(s)));
                self.advance();
            }
            Some(Token::DollarParen) => {
                self.advance();
                let body = self.parse_command_body()?;
                self.expect(&Token::RightParen)?;
                parts.push(WordPart::CommandSub(body));
            }
            Some(Token::DollarBrace) => {
                self.advance();
                let part = self.parse_param_expansion()?;
                parts.push(part);
            }
            Some(Token::DollarArith) => {
                self.advance();
                let expr = self.parse_arithmetic()?;
                parts.push(WordPart::Arithmetic(expr));
            }
            Some(Token::Backtick) => {
                self.advance();
                let body = self.parse_command_body()?;
                self.expect(&Token::Backtick)?;
                parts.push(WordPart::CommandSub(body));
            }
            _ => return Err(anyhow!("Expected word")),
        }

        Ok(Word { parts })
    }

    fn parse_param_expansion(&mut self) -> Result<WordPart> {
        // We're after ${, parse the parameter expansion
        let name = self.expect_word()?;

        match self.current().cloned() {
            Some(Token::RightBrace) => {
                // ${var} - simple expansion, treat as variable
                self.advance();
                Ok(WordPart::Variable(name))
            }
            Some(Token::Word(ref op)) if op.starts_with(':') => {
                // Handle :- := :+ :?
                let op_char = op.chars().nth(1);
                let rest = if op.len() > 2 {
                    op[2..].to_string()
                } else {
                    String::new()
                };

                match op_char {
                    Some('-') => {
                        // ${var:-default}
                        self.advance();
                        let default = self.read_until_brace()?;
                        self.expect(&Token::RightBrace)?;
                        Ok(WordPart::ParamExpansion {
                            name,
                            op: ParamOp::UseDefault(if rest.is_empty() {
                                default
                            } else {
                                format!("{}{}", rest, default)
                            }),
                        })
                    }
                    Some('=') => {
                        // ${var:=default}
                        self.advance();
                        let default = self.read_until_brace()?;
                        self.expect(&Token::RightBrace)?;
                        Ok(WordPart::ParamExpansion {
                            name,
                            op: ParamOp::AssignDefault(if rest.is_empty() {
                                default
                            } else {
                                format!("{}{}", rest, default)
                            }),
                        })
                    }
                    Some('+') => {
                        // ${var:+value}
                        self.advance();
                        let value = self.read_until_brace()?;
                        self.expect(&Token::RightBrace)?;
                        Ok(WordPart::ParamExpansion {
                            name,
                            op: ParamOp::UseAlternative(if rest.is_empty() {
                                value
                            } else {
                                format!("{}{}", rest, value)
                            }),
                        })
                    }
                    Some('?') => {
                        // ${var:?error}
                        self.advance();
                        let error = self.read_until_brace()?;
                        self.expect(&Token::RightBrace)?;
                        Ok(WordPart::ParamExpansion {
                            name,
                            op: ParamOp::ShowError(if rest.is_empty() {
                                error
                            } else {
                                format!("{}{}", rest, error)
                            }),
                        })
                    }
                    _ => {
                        self.advance();
                        self.expect(&Token::RightBrace)?;
                        Ok(WordPart::Variable(name))
                    }
                }
            }
            Some(Token::Word(ref op)) if op == "#" => {
                // ${#var} - string length
                self.advance();
                self.expect(&Token::RightBrace)?;
                Ok(WordPart::ParamExpansion {
                    name,
                    op: ParamOp::StringLength,
                })
            }
            Some(Token::Word(ref op)) if op.starts_with('#') || op.starts_with('%') => {
                // ${var#pattern} or ${var%pattern}
                let is_hash = op.starts_with('#');
                let is_long = op.len() > 1
                    && (op.chars().nth(1) == Some('#') || op.chars().nth(1) == Some('%'));
                let op_suffix = op[if is_long { 2 } else { 1 }..].to_string();
                self.advance();
                let pattern = self.read_until_brace()?;
                self.expect(&Token::RightBrace)?;

                let full_pattern = if !op_suffix.is_empty() {
                    format!("{}{}", op_suffix, pattern)
                } else {
                    pattern
                };

                let op = if is_hash {
                    if is_long {
                        ParamOp::RemovePrefixLongest(full_pattern)
                    } else {
                        ParamOp::RemovePrefixShortest(full_pattern)
                    }
                } else {
                    if is_long {
                        ParamOp::RemoveSuffixLongest(full_pattern)
                    } else {
                        ParamOp::RemoveSuffixShortest(full_pattern)
                    }
                };

                Ok(WordPart::ParamExpansion { name, op })
            }
            Some(Token::Word(ref op)) if op.starts_with('/') || op == "/" => {
                // ${var/pattern/replacement}
                let op_suffix = if op.len() > 1 {
                    op[1..].to_string()
                } else {
                    String::new()
                };
                let is_all = op.starts_with("//");
                self.advance();
                let rest = self.read_until_brace()?;
                self.expect(&Token::RightBrace)?;
                let full = if !op_suffix.is_empty() {
                    format!("{}{}", op_suffix, rest)
                } else {
                    rest
                };

                // Split on / to get pattern and replacement
                let parts: Vec<&str> = full.splitn(2, '/').collect();
                let pattern = parts.first().unwrap_or(&"").to_string();
                let replacement = parts.get(1).unwrap_or(&"").to_string();

                Ok(WordPart::ParamExpansion {
                    name,
                    op: if is_all {
                        ParamOp::ReplaceAll(pattern, replacement)
                    } else {
                        ParamOp::ReplaceFirst(pattern, replacement)
                    },
                })
            }
            Some(Token::Word(ref op)) if op == "," || op == ",," || op == "^" || op == "^^" => {
                // Case modification
                let param_op = match op.as_str() {
                    "," => ParamOp::LowercaseFirst,
                    ",," => ParamOp::LowercaseAll,
                    "^" => ParamOp::UppercaseFirst,
                    "^^" => ParamOp::UppercaseAll,
                    _ => unreachable!(),
                };
                self.advance();
                self.expect(&Token::RightBrace)?;
                Ok(WordPart::ParamExpansion { name, op: param_op })
            }
            Some(Token::Word(ref op)) if op.starts_with(':') => {
                // ${var:start:length} - substring
                let rest = if op.len() > 1 {
                    op[1..].to_string()
                } else {
                    String::new()
                };
                self.advance();
                let more = self.read_until_brace()?;
                self.expect(&Token::RightBrace)?;
                let full = format!("{}{}", rest, more);

                let parts: Vec<&str> = full.splitn(2, ':').collect();
                let start: usize = parts.first().unwrap_or(&"0").parse().unwrap_or(0);
                let length = parts.get(1).and_then(|s| s.parse().ok());

                Ok(WordPart::ParamExpansion {
                    name,
                    op: ParamOp::Substring(start, length),
                })
            }
            _ => {
                // Unknown or just ${var}, close and return variable
                if self.check(&Token::RightBrace) {
                    self.advance();
                }
                Ok(WordPart::Variable(name))
            }
        }
    }

    fn read_until_brace(&mut self) -> Result<String> {
        let mut result = String::new();
        while !self.at_end() && !self.check(&Token::RightBrace) {
            match self.current() {
                Some(Token::Word(s)) => {
                    if !result.is_empty() {
                        result.push(' ');
                    }
                    result.push_str(s);
                    self.advance();
                }
                Some(Token::SingleQuoted(s)) => {
                    if !result.is_empty() {
                        result.push(' ');
                    }
                    result.push('\'');
                    result.push_str(s);
                    result.push('\'');
                    self.advance();
                }
                Some(Token::DoubleQuoted(s)) => {
                    if !result.is_empty() {
                        result.push(' ');
                    }
                    result.push('"');
                    result.push_str(s);
                    result.push('"');
                    self.advance();
                }
                Some(Token::Semi) => {
                    result.push(';');
                    self.advance();
                }
                Some(Token::Pipe) => {
                    result.push('|');
                    self.advance();
                }
                _ => break,
            }
        }
        Ok(result)
    }

    fn parse_arithmetic(&mut self) -> Result<String> {
        // We're after $((, read until ))
        let mut depth = 1;
        let mut expr = String::new();

        while !self.at_end() && depth > 0 {
            match self.current() {
                Some(Token::LeftParen) => {
                    depth += 1;
                    expr.push('(');
                    self.advance();
                }
                Some(Token::RightParen) => {
                    depth -= 1;
                    if depth > 0 {
                        expr.push(')');
                    }
                    self.advance();
                    if depth > 0 && self.check(&Token::RightParen) {
                        // Second closing paren for $((...))
                        depth -= 1;
                        expr.push(')');
                        self.advance();
                    }
                }
                Some(Token::Word(s)) => {
                    if !expr.is_empty() {
                        expr.push(' ');
                    }
                    expr.push_str(s);
                    self.advance();
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_double_quoted_parts(s: &str) -> Vec<WordPart> {
        let mut parts = Vec::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        let mut literal = String::new();

        while i < chars.len() {
            if chars[i] == '$'
                && i + 1 < chars.len()
                && (chars[i + 1].is_alphanumeric() || chars[i + 1] == '_')
            {
                if !literal.is_empty() {
                    parts.push(WordPart::Literal(literal.clone()));
                    literal.clear();
                }
                i += 1; // skip $
                let mut var_name = String::new();
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    var_name.push(chars[i]);
                    i += 1;
                }
                parts.push(WordPart::Variable(var_name));
            } else {
                literal.push(chars[i]);
                i += 1;
            }
        }

        if !literal.is_empty() {
            parts.push(WordPart::Literal(literal));
        }

        parts
    }

    // --- Control structures ---

    fn parse_if_statement(&mut self) -> Result<CommandBody> {
        self.advance(); // consume 'if'
        self.skip_sep();

        let condition = self.parse_condition_list()?;
        self.skip_sep();
        self.expect(&Token::Then)?;
        self.skip_sep();

        let then_body = self.parse_body_until(&[&Token::Elif, &Token::Else, &Token::Fi])?;
        self.skip_sep();

        let mut elif_branches = Vec::new();
        while self.check(&Token::Elif) {
            self.advance();
            self.skip_sep();
            let elif_condition = self.parse_condition_list()?;
            self.skip_sep();
            self.expect(&Token::Then)?;
            self.skip_sep();
            let elif_body = self.parse_body_until(&[&Token::Elif, &Token::Else, &Token::Fi])?;
            self.skip_sep();
            elif_branches.push(ElifBranch {
                condition: elif_condition,
                body: elif_body,
            });
        }

        let else_body = if self.check(&Token::Else) {
            self.advance();
            self.skip_sep();
            let body = self.parse_body_until(&[&Token::Fi])?;
            self.skip_sep();
            Some(body)
        } else {
            None
        };

        self.expect(&Token::Fi)?;

        Ok(CommandBody::If(IfStatement {
            condition,
            then_body,
            elif_branches,
            else_body,
        }))
    }

    fn parse_for_loop(&mut self) -> Result<CommandBody> {
        self.advance(); // consume 'for'
        self.skip_sep();
        let variable = self.expect_word()?;
        self.skip_sep();

        // Optional 'in list'
        let list = if self.check(&Token::In) {
            self.advance();
            self.skip_sep();
            let mut list = Vec::new();
            while !self.at_end()
                && !self.is_command_terminator()
                && !self.check(&Token::Do)
                && !self.check(&Token::Semi)
                && !self.check(&Token::Newline)
            {
                list.push(self.parse_word()?);
            }
            list
        } else {
            Vec::new()
        };
        self.skip_sep_optional();

        // Expect 'do'
        self.expect(&Token::Do)?;
        self.skip_sep();

        let body = self.parse_body_until(&[&Token::Done])?;
        self.skip_sep();
        self.expect(&Token::Done)?;

        Ok(CommandBody::ForLoop(ForLoop {
            variable,
            list,
            body,
        }))
    }

    fn parse_while_loop(&mut self) -> Result<CommandBody> {
        self.advance(); // consume 'while'
        self.skip_sep();

        let condition = self.parse_condition_list()?;
        self.skip_sep();

        self.expect(&Token::Do)?;
        self.skip_sep();

        let body = self.parse_body_until(&[&Token::Done])?;
        self.skip_sep();
        self.expect(&Token::Done)?;

        Ok(CommandBody::WhileLoop(WhileLoop { condition, body }))
    }

    fn parse_until_loop(&mut self) -> Result<CommandBody> {
        self.advance(); // consume 'until'
        self.skip_sep();

        let condition = self.parse_condition_list()?;
        self.skip_sep();

        self.expect(&Token::Do)?;
        self.skip_sep();

        let body = self.parse_body_until(&[&Token::Done])?;
        self.skip_sep();
        self.expect(&Token::Done)?;

        Ok(CommandBody::UntilLoop(UntilLoop { condition, body }))
    }

    fn parse_case_statement(&mut self) -> Result<CommandBody> {
        self.advance(); // consume 'case'
        self.skip_sep();

        let value = self.parse_word()?;
        self.skip_sep();

        // Expect 'in'
        self.expect(&Token::In)?;
        self.skip_sep();

        let mut cases = Vec::new();

        while !self.at_end() && !self.check(&Token::Esac) {
            // Parse patterns: pattern1 | pattern2 | pattern3 )
            let mut patterns = Vec::new();
            loop {
                patterns.push(self.parse_word()?);
                self.skip_sep_optional();
                if self.check(&Token::Pipe) {
                    self.advance();
                    self.skip_sep_optional();
                } else if self.check(&Token::RightParen) {
                    self.advance();
                    break;
                } else {
                    // Might be just pattern) without pipe
                    break;
                }
            }
            self.skip_sep();

            // Parse body until ;; or esac
            let mut body = Vec::new();
            while !self.at_end() && !self.check(&Token::Esac) && !self.is_case_terminator() {
                self.skip_sep();
                if self.check(&Token::Esac) || self.is_case_terminator() {
                    break;
                }
                body.push(self.parse_command()?);
                self.skip_sep_optional();
            }

            // Consume ;;
            if self.check(&Token::Semi) {
                self.advance();
                if self.check(&Token::Semi) {
                    self.advance();
                }
            }
            self.skip_sep();

            cases.push(CaseItem { patterns, body });
        }

        self.expect(&Token::Esac)?;

        Ok(CommandBody::Case(CaseStatement { value, cases }))
    }

    fn is_case_terminator(&self) -> bool {
        // Check for ;; followed by another case or esac
        if !self.check(&Token::Semi) {
            return false;
        }
        // We need to look ahead for ;;
        // The lexer produces two Semi tokens for ;;
        true
    }

    fn parse_function_def_keyword(&mut self) -> Result<CommandBody> {
        self.advance(); // consume 'function'
        self.skip_sep();
        let name = self.expect_word()?;
        self.skip_sep_optional();

        // Optional ()
        if self.check(&Token::LeftParen) {
            self.advance();
            self.expect(&Token::RightParen)?;
            self.skip_sep_optional();
        }

        // Expect {
        self.expect(&Token::LeftBrace)?;
        self.skip_sep();

        let body = self.parse_body_until(&[&Token::RightBrace])?;
        self.skip_sep_optional();
        self.expect(&Token::RightBrace)?;

        Ok(CommandBody::FunctionDef(FunctionDef { name, body }))
    }

    fn is_function_def(&self) -> bool {
        // Check for name() pattern
        if let Some(Token::Word(_)) = self.current() {
            if self.pos + 2 < self.tokens.len() {
                return matches!(
                    (&self.tokens[self.pos + 1], &self.tokens[self.pos + 2]),
                    (Token::LeftParen, Token::RightParen)
                );
            }
        }
        false
    }

    fn parse_function_def_parens(&mut self) -> Result<CommandBody> {
        let name = self.expect_word()?;
        self.expect(&Token::LeftParen)?;
        self.expect(&Token::RightParen)?;
        self.skip_sep_optional();

        // Expect {
        self.expect(&Token::LeftBrace)?;
        self.skip_sep();

        let body = self.parse_body_until(&[&Token::RightBrace])?;
        self.skip_sep_optional();
        self.expect(&Token::RightBrace)?;

        Ok(CommandBody::FunctionDef(FunctionDef { name, body }))
    }

    fn parse_subshell(&mut self) -> Result<CommandBody> {
        self.advance(); // consume '('
        self.skip_sep();
        let body = self.parse_command_body()?;
        self.skip_sep();
        self.expect(&Token::RightParen)?;
        Ok(CommandBody::Subshell(Box::new(body)))
    }

    fn parse_group(&mut self) -> Result<CommandBody> {
        self.advance(); // consume '{'
        self.skip_sep();
        let body = self.parse_command_body()?;
        self.skip_sep();
        self.expect(&Token::RightBrace)?;
        Ok(CommandBody::Group(Box::new(body)))
    }

    fn parse_return(&mut self) -> Result<CommandBody> {
        self.advance(); // consume 'return'
        let value = if !self.at_end() && !self.is_command_terminator() && !self.check(&Token::Semi)
        {
            Some(self.parse_word()?)
        } else {
            None
        };
        Ok(CommandBody::Return(value))
    }

    /// Parse a condition list (commands until 'then' or 'do')
    fn parse_condition_list(&mut self) -> Result<Vec<Command>> {
        let mut condition = Vec::new();
        while !self.at_end()
            && !self.check(&Token::Then)
            && !self.check(&Token::Do)
            && !self.check(&Token::Semi)
        {
            condition.push(self.parse_command()?);
            self.skip_sep_optional();
            if self.check(&Token::Semi) {
                self.advance();
                self.skip_sep();
            }
        }
        Ok(condition)
    }

    /// Parse body commands until one of the terminator tokens
    fn parse_body_until(&mut self, terminators: &[&Token]) -> Result<Vec<Command>> {
        let mut body = Vec::new();
        while !self.at_end() {
            self.skip_sep();
            if self.at_end() || terminators.iter().any(|t| self.check(t)) {
                break;
            }
            body.push(self.parse_command()?);
            self.skip_sep_optional();
            if self.check(&Token::Semi) {
                self.advance();
            }
        }
        Ok(body)
    }

    // --- Helpers ---

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn check(&self, token: &Token) -> bool {
        self.current() == Some(token)
    }

    fn check_word(&self, expected: &str) -> bool {
        matches!(self.current(), Some(Token::Word(s)) if s == expected)
    }

    fn expect(&mut self, token: &Token) -> Result<()> {
        if self.check(token) {
            self.advance();
            Ok(())
        } else {
            Err(anyhow!("Expected {:?}, got {:?}", token, self.current()))
        }
    }

    fn expect_word(&mut self) -> Result<String> {
        let s = match self.current() {
            Some(Token::Word(s)) => s.clone(),
            other => return Err(anyhow!("Expected word, got {:?}", other)),
        };
        self.advance();
        Ok(s)
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.current(), Some(Token::Eof))
    }

    fn skip_sep(&mut self) {
        while self.check(&Token::Newline) || self.check(&Token::Semi) {
            self.advance();
        }
    }

    fn skip_sep_optional(&mut self) {
        self.skip_sep();
    }

    fn is_command_terminator(&self) -> bool {
        matches!(
            self.current(),
            Some(Token::Pipe)
                | Some(Token::And)
                | Some(Token::Or)
                | Some(Token::Semi)
                | Some(Token::Ampersand)
                | Some(Token::Newline)
                | Some(Token::Eof)
                | Some(Token::RightParen)
                | Some(Token::RightBrace)
                | Some(Token::Fi)
                | Some(Token::Done)
                | Some(Token::Esac)
                | Some(Token::Then)
                | Some(Token::Do)
                | Some(Token::Elif)
                | Some(Token::Else)
        )
    }

    fn is_redirect(&self) -> bool {
        matches!(
            self.current(),
            Some(Token::RedirectOut)
                | Some(Token::RedirectAppend)
                | Some(Token::RedirectIn)
                | Some(Token::RedirectBoth)
                | Some(Token::RedirectBothAppend)
                | Some(Token::RedirectDupOut)
                | Some(Token::RedirectDupIn)
                | Some(Token::HereString)
                | Some(Token::HereDoc)
                | Some(Token::HereDocStrip)
        ) || matches!(self.current(), Some(Token::Word(s)) if s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) && self.pos + 1 < self.tokens.len() && matches!(self.tokens.get(self.pos + 1), Some(Token::RedirectOut) | Some(Token::RedirectAppend) | Some(Token::RedirectIn)))
    }

    fn is_assignment(&self) -> bool {
        matches!(self.current(), Some(Token::Word(s)) if {
            if let Some(eq_pos) = s.find('=') {
                let name = &s[..eq_pos];
                !name.is_empty() && name.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false)
                    && name.chars().all(|c| c.is_alphanumeric() || c == '_')
            } else {
                false
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let mut parser = Parser::new("ls -la /tmp");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::Simple(cmd) => {
                assert_eq!(cmd.words.len(), 3);
            }
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_pipeline() {
        let mut parser = Parser::new("ls | grep foo");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::Pipeline(pipeline) => {
                assert_eq!(pipeline.commands.len(), 2);
            }
            _ => panic!("Expected pipeline"),
        }
    }

    #[test]
    fn test_redirect() {
        let mut parser = Parser::new("echo hello > file.txt");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::Simple(cmd) => {
                assert_eq!(cmd.redirects.len(), 1);
                assert_eq!(cmd.words.len(), 2);
            }
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_command_substitution() {
        let mut parser = Parser::new("echo $(echo hello)");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::Simple(cmd) => {
                assert_eq!(cmd.words.len(), 2);
                match &cmd.words[1].parts[0] {
                    WordPart::CommandSub(_) => {}
                    _ => panic!("Expected CommandSub, got {:?}", cmd.words[1].parts[0]),
                }
            }
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_if_statement() {
        let mut parser = Parser::new("if true; then echo yes; fi");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::If(stmt) => {
                assert_eq!(stmt.condition.len(), 1);
                assert_eq!(stmt.then_body.len(), 1);
                assert!(stmt.elif_branches.is_empty());
                assert!(stmt.else_body.is_none());
            }
            _ => panic!("Expected If"),
        }
    }

    #[test]
    fn test_if_else() {
        let mut parser = Parser::new("if false; then echo yes; else echo no; fi");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::If(stmt) => {
                assert!(stmt.else_body.is_some());
                assert_eq!(stmt.else_body.as_ref().unwrap().len(), 1);
            }
            _ => panic!("Expected If"),
        }
    }

    #[test]
    fn test_if_elif_else() {
        let mut parser =
            Parser::new("if false; then echo a; elif true; then echo b; else echo c; fi");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::If(stmt) => {
                assert_eq!(stmt.elif_branches.len(), 1);
                assert!(stmt.else_body.is_some());
            }
            _ => panic!("Expected If"),
        }
    }

    #[test]
    fn test_for_loop() {
        let mut parser = Parser::new("for i in 1 2 3; do echo $i; done");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::ForLoop(f) => {
                assert_eq!(f.variable, "i");
                assert_eq!(f.list.len(), 3);
                assert_eq!(f.body.len(), 1);
            }
            _ => panic!("Expected ForLoop, got {:?}", commands[0].body),
        }
    }

    #[test]
    fn test_while_loop() {
        let mut parser = Parser::new("while true; do echo loop; done");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::WhileLoop(w) => {
                assert_eq!(w.condition.len(), 1);
                assert_eq!(w.body.len(), 1);
            }
            _ => panic!("Expected WhileLoop, got {:?}", commands[0].body),
        }
    }

    #[test]
    fn test_until_loop() {
        let mut parser = Parser::new("until false; do echo loop; done");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::UntilLoop(u) => {
                assert_eq!(u.condition.len(), 1);
                assert_eq!(u.body.len(), 1);
            }
            _ => panic!("Expected UntilLoop"),
        }
    }

    #[test]
    fn test_case_statement() {
        let mut parser = Parser::new("case $x in foo) echo bar;; esac");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::Case(c) => {
                assert_eq!(c.cases.len(), 1);
                assert_eq!(c.cases[0].patterns.len(), 1);
            }
            _ => panic!("Expected Case, got {:?}", commands[0].body),
        }
    }

    #[test]
    fn test_function_def_parens() {
        let mut parser = Parser::new("foo() { echo hello; }");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::FunctionDef(f) => {
                assert_eq!(f.name, "foo");
                assert_eq!(f.body.len(), 1);
            }
            _ => panic!("Expected FunctionDef, got {:?}", commands[0].body),
        }
    }

    #[test]
    fn test_function_def_keyword() {
        let mut parser = Parser::new("function foo { echo hello; }");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::FunctionDef(f) => {
                assert_eq!(f.name, "foo");
                assert_eq!(f.body.len(), 1);
            }
            _ => panic!("Expected FunctionDef, got {:?}", commands[0].body),
        }
    }

    #[test]
    fn test_subshell() {
        let mut parser = Parser::new("(echo hello)");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::Subshell(_) => {}
            _ => panic!("Expected Subshell"),
        }
    }

    #[test]
    fn test_group() {
        let mut parser = Parser::new("{ echo hello; }");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::Group(_) => {}
            _ => panic!("Expected Group"),
        }
    }

    #[test]
    fn test_background_command() {
        let mut parser = Parser::new("sleep 10 &");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        assert!(commands[0].background);
    }

    #[test]
    fn test_and_chain() {
        let mut parser = Parser::new("true && echo yes");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::And(_, _) => {}
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn test_or_chain() {
        let mut parser = Parser::new("false || echo fallback");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::Or(_, _) => {}
            _ => panic!("Expected Or"),
        }
    }

    #[test]
    fn test_variable() {
        let mut parser = Parser::new("echo $VAR");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::Simple(cmd) => {
                assert_eq!(
                    cmd.words[1].parts,
                    vec![WordPart::Variable("VAR".to_string())]
                );
            }
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_param_expansion_default() {
        let mut parser = Parser::new("echo ${x:-default}");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::Simple(cmd) => match &cmd.words[1].parts[0] {
                WordPart::ParamExpansion { name, op } => {
                    assert_eq!(name, "x");
                    assert_eq!(*op, ParamOp::UseDefault("default".to_string()));
                }
                other => panic!("Expected ParamExpansion, got {:?}", other),
            },
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_param_expansion_length() {
        let mut parser = Parser::new("echo ${#var}");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::Simple(cmd) => match &cmd.words[1].parts[0] {
                WordPart::ParamExpansion { name, op } => {
                    assert_eq!(name, "var");
                    assert_eq!(*op, ParamOp::StringLength);
                }
                other => panic!("Expected ParamExpansion, got {:?}", other),
            },
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_empty_input() {
        let mut parser = Parser::new("");
        let commands = parser.parse().unwrap();
        assert!(commands.is_empty());
    }

    #[test]
    fn test_multiple_commands_semicolon() {
        let mut parser = Parser::new("echo a; echo b; echo c");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 3);
    }

    #[test]
    fn test_redirect_append() {
        let mut parser = Parser::new("echo hello >> log.txt");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::Simple(cmd) => {
                assert_eq!(cmd.redirects[0].op, RedirectOp::Append);
            }
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_redirect_both() {
        let mut parser = Parser::new("cmd &> /dev/null");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::Simple(cmd) => {
                assert_eq!(cmd.redirects[0].op, RedirectOp::BothOutput);
            }
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_assignment() {
        let mut parser = Parser::new("FOO=bar");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::Simple(cmd) => {
                assert!(cmd.words.is_empty());
                assert_eq!(cmd.assignments.len(), 1);
                assert_eq!(cmd.assignments[0].name, "FOO");
            }
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_single_quoted() {
        let mut parser = Parser::new("echo 'hello world'");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::Simple(cmd) => {
                assert_eq!(
                    cmd.words[1].parts,
                    vec![WordPart::SingleQuoted("hello world".to_string())]
                );
            }
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_double_quoted() {
        let mut parser = Parser::new(r#"echo "hello world""#);
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::Simple(cmd) => match &cmd.words[1].parts[0] {
                WordPart::DoubleQuoted(parts) => {
                    assert_eq!(parts, &vec![WordPart::Literal("hello world".to_string())]);
                }
                _ => panic!("Expected DoubleQuoted"),
            },
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_heredoc() {
        let mut parser = Parser::new("cat <<EOF\nhello world\nEOF");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::Simple(cmd) => {
                assert_eq!(cmd.redirects[0].op, RedirectOp::HereDoc);
            }
            _ => panic!("Expected simple command"),
        }
    }

    #[test]
    fn test_function_with_multiple_commands() {
        let mut parser = Parser::new("foo() { echo a; echo b; echo c; }");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::FunctionDef(f) => {
                assert_eq!(f.body.len(), 3);
            }
            _ => panic!("Expected FunctionDef"),
        }
    }

    #[test]
    fn test_chained_and_or() {
        let mut parser = Parser::new("a && b || c");
        let commands = parser.parse().unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0].body {
            CommandBody::Or(left, _right) => match left.as_ref() {
                CommandBody::And(_, _) => {}
                _ => panic!("Expected And inside Or"),
            },
            _ => panic!("Expected Or"),
        }
    }

    #[test]
    fn test_three_stage_pipeline() {
        let mut parser = Parser::new("cat file | grep err | wc -l");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::Pipeline(p) => {
                assert_eq!(p.commands.len(), 3);
            }
            _ => panic!("Expected pipeline"),
        }
    }

    #[test]
    fn test_negated_pipeline() {
        let mut parser = Parser::new("! true | false");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::Pipeline(p) => {
                assert!(p.negated);
            }
            _ => panic!("Expected pipeline"),
        }
    }

    #[test]
    fn test_case_multiple_patterns() {
        let mut parser = Parser::new("case $x in a|b|c) echo match;; esac");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::Case(c) => {
                assert_eq!(c.cases[0].patterns.len(), 3);
            }
            _ => panic!("Expected Case"),
        }
    }

    #[test]
    fn test_return() {
        let mut parser = Parser::new("return 0");
        let commands = parser.parse().unwrap();
        match &commands[0].body {
            CommandBody::Return(val) => {
                assert!(val.is_some());
            }
            _ => panic!("Expected Return"),
        }
    }
}
