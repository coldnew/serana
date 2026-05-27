use async_trait::async_trait;
use serde_json::{json, Value};

use serana_core::{Result, Tool};

pub struct CalcTool;

#[async_trait]
impl Tool for CalcTool {
    fn name(&self) -> &'static str {
        "calc"
    }

    fn description(&self) -> &'static str {
        "Evaluate a mathematical expression. Supports basic arithmetic, powers, roots, trig functions. Input: {\"expression\": \"2 + 3 * 4\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "Mathematical expression to evaluate"
                }
            },
            "required": ["expression"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let expr = input
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'expression' field"))?;

        let result = eval_expr(expr)?;
        Ok(json!({ "result": result, "expression": expr }))
    }
}

fn eval_expr(expr: &str) -> Result<f64> {
    let expr = expr.replace(' ', "");
    let tokens = tokenize(&expr)?;
    let result = parse_expr(&tokens, &mut 0)?;
    Ok(result)
}

#[derive(Debug, Clone)]
enum Token {
    Num(f64),
    Op(char),
    LParen,
    RParen,
    Func(String),
}

fn tokenize(expr: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '0'..='9' | '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::Num(s.parse::<f64>()?));
                continue;
            }
            '+' | '-' | '*' | '/' | '%' | '^' => tokens.push(Token::Op(chars[i])),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            'a'..='z' | 'A'..='Z' => {
                let start = i;
                while i < chars.len() && chars[i].is_alphanumeric() {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::Func(s));
                continue;
            }
            _ => anyhow::bail!("Unexpected character: {}", chars[i]),
        }
        i += 1;
    }
    Ok(tokens)
}

fn parse_expr(tokens: &[Token], pos: &mut usize) -> Result<f64> {
    let mut left = parse_term(tokens, pos)?;
    while *pos < tokens.len() {
        if let Token::Op(op) = &tokens[*pos] {
            if *op == '+' || *op == '-' {
                *pos += 1;
                let right = parse_term(tokens, pos)?;
                left = match op {
                    '+' => left + right,
                    '-' => left - right,
                    _ => unreachable!(),
                };
            } else {
                break;
            }
        } else {
            break;
        }
    }
    Ok(left)
}

fn parse_term(tokens: &[Token], pos: &mut usize) -> Result<f64> {
    let mut left = parse_power(tokens, pos)?;
    while *pos < tokens.len() {
        if let Token::Op(op) = &tokens[*pos] {
            if *op == '*' || *op == '/' || *op == '%' {
                *pos += 1;
                let right = parse_power(tokens, pos)?;
                left = match op {
                    '*' => left * right,
                    '/' => {
                        if right == 0.0 {
                            anyhow::bail!("Division by zero");
                        }
                        left / right
                    }
                    '%' => left % right,
                    _ => unreachable!(),
                };
            } else {
                break;
            }
        } else {
            break;
        }
    }
    Ok(left)
}

fn parse_power(tokens: &[Token], pos: &mut usize) -> Result<f64> {
    let mut base = parse_unary(tokens, pos)?;
    if *pos < tokens.len() {
        if let Token::Op('^') = &tokens[*pos] {
            *pos += 1;
            let exp = parse_power(tokens, pos)?;
            base = base.powf(exp);
        }
    }
    Ok(base)
}

fn parse_unary(tokens: &[Token], pos: &mut usize) -> Result<f64> {
    if *pos < tokens.len() {
        if let Token::Op('-') = &tokens[*pos] {
            *pos += 1;
            let val = parse_atom(tokens, pos)?;
            return Ok(-val);
        }
        if let Token::Op('+') = &tokens[*pos] {
            *pos += 1;
            return parse_atom(tokens, pos);
        }
    }
    parse_atom(tokens, pos)
}

fn parse_atom(tokens: &[Token], pos: &mut usize) -> Result<f64> {
    if *pos >= tokens.len() {
        anyhow::bail!("Unexpected end of expression");
    }
    match &tokens[*pos] {
        Token::Num(n) => {
            *pos += 1;
            Ok(*n)
        }
        Token::LParen => {
            *pos += 1;
            let val = parse_expr(tokens, pos)?;
            if *pos < tokens.len() && matches!(&tokens[*pos], Token::RParen) {
                *pos += 1;
            }
            Ok(val)
        }
        Token::Func(name) => {
            let name = name.clone();
            *pos += 1;
            if *pos < tokens.len() && matches!(&tokens[*pos], Token::LParen) {
                *pos += 1;
                let arg = parse_expr(tokens, pos)?;
                if *pos < tokens.len() && matches!(&tokens[*pos], Token::RParen) {
                    *pos += 1;
                }
                apply_func(&name, arg)
            } else {
                anyhow::bail!("Expected '(' after function '{}'", name)
            }
        }
        _ => anyhow::bail!("Unexpected token: {:?}", tokens[*pos]),
    }
}

fn apply_func(name: &str, arg: f64) -> Result<f64> {
    match name {
        "sqrt" => Ok(arg.sqrt()),
        "abs" => Ok(arg.abs()),
        "sin" => Ok(arg.sin()),
        "cos" => Ok(arg.cos()),
        "tan" => Ok(arg.tan()),
        "asin" => Ok(arg.asin()),
        "acos" => Ok(arg.acos()),
        "atan" => Ok(arg.atan()),
        "ln" => Ok(arg.ln()),
        "log" | "log10" => Ok(arg.log10()),
        "log2" => Ok(arg.log2()),
        "exp" => Ok(arg.exp()),
        "ceil" => Ok(arg.ceil()),
        "floor" => Ok(arg.floor()),
        "round" => Ok(arg.round()),
        "deg" => Ok(arg.to_degrees()),
        "rad" => Ok(arg.to_radians()),
        _ => anyhow::bail!("Unknown function: {}", name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_arithmetic() {
        assert_eq!(eval_expr("2 + 3").unwrap(), 5.0);
        assert_eq!(eval_expr("10 - 4").unwrap(), 6.0);
        assert_eq!(eval_expr("3 * 7").unwrap(), 21.0);
        assert_eq!(eval_expr("15 / 3").unwrap(), 5.0);
    }

    #[test]
    fn operator_precedence() {
        assert_eq!(eval_expr("2 + 3 * 4").unwrap(), 14.0);
        assert_eq!(eval_expr("(2 + 3) * 4").unwrap(), 20.0);
    }

    #[test]
    fn power_and_functions() {
        assert_eq!(eval_expr("2 ^ 3").unwrap(), 8.0);
        assert_eq!(eval_expr("sqrt(16)").unwrap(), 4.0);
        assert!(eval_expr("abs(-5)").unwrap() - 5.0 < 1e-10);
    }
}
