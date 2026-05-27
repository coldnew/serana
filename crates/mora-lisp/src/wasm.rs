use crate::types::Value;

#[derive(Debug, thiserror::Error)]
pub enum WasmError {
    #[error("unsupported form: {0}")]
    UnsupportedForm(String),
    #[error("compilation error: {0}")]
    CompilationError(String),
    #[error("{0}")]
    Custom(String),
}

pub struct WasmCompiler {
    module: WasmModule,
}

struct WasmModule {
    functions: Vec<WasmFunction>,
    memory: Option<WasmMemory>,
    exports: Vec<WasmExport>,
    imports: Vec<WasmImport>,
}

struct WasmFunction {
    name: String,
    params: Vec<WasmType>,
    results: Vec<WasmType>,
    locals: Vec<WasmType>,
    body: Vec<WasmInstruction>,
}

#[derive(Clone, Debug)]
enum WasmType {
    I32,
    I64,
    F32,
    F64,
}

struct WasmMemory {
    initial: u32,
    maximum: Option<u32>,
}

struct WasmExport {
    name: String,
    kind: WasmExportKind,
    index: u32,
}

enum WasmExportKind {
    Function,
    Memory,
    Table,
    Global,
}

struct WasmImport {
    module: String,
    name: String,
    kind: WasmImportKind,
}

enum WasmImportKind {
    Function {
        params: Vec<WasmType>,
        results: Vec<WasmType>,
    },
    Memory {
        initial: u32,
        maximum: Option<u32>,
    },
}

#[derive(Clone, Debug)]
enum WasmInstruction {
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),
    Call(u32),
    CallIndirect(u32),
    Block(Vec<WasmInstruction>),
    Loop(Vec<WasmInstruction>),
    If {
        condition: Box<WasmInstruction>,
        then_branch: Vec<WasmInstruction>,
        else_branch: Option<Vec<WasmInstruction>>,
    },
    Return,
    Drop,
    Nop,
    Unreachable,
    MemorySize,
    MemoryGrow,
    I32Load {
        offset: u32,
        align: u32,
    },
    I32Store {
        offset: u32,
        align: u32,
    },
    I64Load {
        offset: u32,
        align: u32,
    },
    I64Store {
        offset: u32,
        align: u32,
    },
}

impl WasmCompiler {
    pub fn new() -> Self {
        Self {
            module: WasmModule {
                functions: Vec::new(),
                memory: None,
                exports: Vec::new(),
                imports: Vec::new(),
            },
        }
    }

    pub fn compile(&mut self, forms: &[Value]) -> Result<Vec<u8>, WasmError> {
        for form in forms {
            self.compile_form(form)?;
        }
        self.emit_binary()
    }

    fn compile_form(&mut self, form: &Value) -> Result<(), WasmError> {
        match form {
            Value::List(list) if !list.is_empty() => {
                if let Value::Symbol(sym) = &list[0] {
                    match sym.name.as_str() {
                        "defn" => self.compile_defn(&list[1..]),
                        "def" => self.compile_def(&list[1..]),
                        "fn" => self.compile_lambda(&list[1..]),
                        "do" => {
                            for form in &list[1..] {
                                self.compile_form(form)?;
                            }
                            Ok(())
                        }
                        _ => self.compile_call(list),
                    }
                } else {
                    self.compile_call(list)
                }
            }
            _ => Ok(()),
        }
    }

    fn compile_defn(&mut self, args: &[Value]) -> Result<(), WasmError> {
        if args.len() < 3 {
            return Err(WasmError::CompilationError(
                "defn requires at least 3 arguments".to_string(),
            ));
        }
        let name = match &args[0] {
            Value::Symbol(s) => s.name.to_string(),
            _ => {
                return Err(WasmError::CompilationError(
                    "defn name must be a symbol".to_string(),
                ))
            }
        };
        let params = self.parse_wasm_params(&args[1])?;
        let body = &args[2..];

        let mut func = WasmFunction {
            name,
            params,
            results: vec![WasmType::I64], // Default to i64 for Lisp values
            locals: Vec::new(),
            body: Vec::new(),
        };

        for form in body {
            self.compile_expr(form, &mut func)?;
        }

        self.module.functions.push(func);
        Ok(())
    }

    fn compile_def(&mut self, _args: &[Value]) -> Result<(), WasmError> {
        // Global definitions become globals in WASM
        Ok(())
    }

    fn compile_lambda(&mut self, _args: &[Value]) -> Result<(), WasmError> {
        // Anonymous functions
        Ok(())
    }

    fn compile_call(&mut self, _list: &[Value]) -> Result<(), WasmError> {
        // Function calls
        Ok(())
    }

    fn compile_expr(&self, form: &Value, func: &mut WasmFunction) -> Result<(), WasmError> {
        match form {
            Value::Int(n) => {
                func.body.push(WasmInstruction::I64Const(*n));
            }
            Value::Float(n) => {
                func.body.push(WasmInstruction::F64Const(*n));
            }
            Value::Bool(b) => {
                func.body
                    .push(WasmInstruction::I64Const(if *b { 1 } else { 0 }));
            }
            Value::List(list) if !list.is_empty() => {
                if let Value::Symbol(sym) = &list[0] {
                    match sym.name.as_str() {
                        "+" => {
                            for arg in &list[1..] {
                                self.compile_expr(arg, func)?;
                            }
                            func.body.push(WasmInstruction::I64Add);
                        }
                        "-" => {
                            for arg in &list[1..] {
                                self.compile_expr(arg, func)?;
                            }
                            func.body.push(WasmInstruction::I64Sub);
                        }
                        "*" => {
                            for arg in &list[1..] {
                                self.compile_expr(arg, func)?;
                            }
                            func.body.push(WasmInstruction::I64Mul);
                        }
                        "/" => {
                            for arg in &list[1..] {
                                self.compile_expr(arg, func)?;
                            }
                            func.body.push(WasmInstruction::I64DivS);
                        }
                        "if" => {
                            if list.len() >= 3 {
                                self.compile_expr(&list[1], func)?;
                                let then_body = Vec::new();
                                let mut then_func = WasmFunction {
                                    name: String::new(),
                                    params: Vec::new(),
                                    results: Vec::new(),
                                    locals: Vec::new(),
                                    body: then_body,
                                };
                                self.compile_expr(&list[2], &mut then_func)?;

                                let else_body = if list.len() > 3 {
                                    let mut else_func = WasmFunction {
                                        name: String::new(),
                                        params: Vec::new(),
                                        results: Vec::new(),
                                        locals: Vec::new(),
                                        body: Vec::new(),
                                    };
                                    self.compile_expr(&list[3], &mut else_func)?;
                                    Some(else_func.body)
                                } else {
                                    None
                                };

                                func.body.push(WasmInstruction::If {
                                    condition: Box::new(WasmInstruction::I64Const(0)), // Placeholder
                                    then_branch: then_func.body,
                                    else_branch: else_body,
                                });
                            }
                        }
                        _ => {
                            // Compile arguments
                            for arg in &list[1..] {
                                self.compile_expr(arg, func)?;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn parse_wasm_params(&self, form: &Value) -> Result<Vec<WasmType>, WasmError> {
        let params = match form {
            Value::Vector(v) => v.as_ref(),
            Value::List(l) => l.as_ref(),
            _ => {
                return Err(WasmError::CompilationError(
                    "parameter list must be a vector".to_string(),
                ))
            }
        };
        // Default all params to i64 for Lisp value representation
        Ok(params.iter().map(|_| WasmType::I64).collect())
    }

    fn emit_binary(&self) -> Result<Vec<u8>, WasmError> {
        let mut bytes = Vec::new();

        // WASM magic number and version
        bytes.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]); // \0asm
        bytes.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // version 1

        // Type section
        if !self.module.functions.is_empty() {
            bytes.push(0x01); // section id
            let mut type_section = Vec::new();
            type_section.push(self.module.functions.len() as u8);
            for func in &self.module.functions {
                type_section.push(0x60); // func type
                type_section.push(func.params.len() as u8);
                for param in &func.params {
                    type_section.push(wasm_type_byte(param));
                }
                type_section.push(func.results.len() as u8);
                for result in &func.results {
                    type_section.push(wasm_type_byte(result));
                }
            }
            bytes.push(type_section.len() as u8);
            bytes.extend_from_slice(&type_section);
        }

        // Function section
        if !self.module.functions.is_empty() {
            bytes.push(0x03); // section id
            let mut func_section = Vec::new();
            func_section.push(self.module.functions.len() as u8);
            for i in 0..self.module.functions.len() {
                func_section.push(i as u8);
            }
            bytes.push(func_section.len() as u8);
            bytes.extend_from_slice(&func_section);
        }

        // Code section
        if !self.module.functions.is_empty() {
            bytes.push(0x0A); // section id
            let mut code_section = Vec::new();
            code_section.push(self.module.functions.len() as u8);
            for func in &self.module.functions {
                let mut func_body = Vec::new();
                func_body.push(func.locals.len() as u8);
                for local in &func.locals {
                    func_body.push(1); // count
                    func_body.push(wasm_type_byte(local));
                }
                for instruction in &func.body {
                    emit_instruction(instruction, &mut func_body);
                }
                func_body.push(0x0B); // end
                code_section.push(func_body.len() as u8);
                code_section.extend_from_slice(&func_body);
            }
            bytes.push(code_section.len() as u8);
            bytes.extend_from_slice(&code_section);
        }

        Ok(bytes)
    }
}

fn wasm_type_byte(ty: &WasmType) -> u8 {
    match ty {
        WasmType::I32 => 0x7F,
        WasmType::I64 => 0x7E,
        WasmType::F32 => 0x7D,
        WasmType::F64 => 0x7C,
    }
}

fn emit_instruction(instr: &WasmInstruction, bytes: &mut Vec<u8>) {
    match instr {
        WasmInstruction::I32Const(n) => {
            bytes.push(0x41);
            bytes.extend_from_slice(&(*n as i32).to_le_bytes());
        }
        WasmInstruction::I64Const(n) => {
            bytes.push(0x42);
            bytes.extend_from_slice(&n.to_le_bytes());
        }
        WasmInstruction::F32Const(n) => {
            bytes.push(0x43);
            bytes.extend_from_slice(&n.to_le_bytes());
        }
        WasmInstruction::F64Const(n) => {
            bytes.push(0x44);
            bytes.extend_from_slice(&n.to_le_bytes());
        }
        WasmInstruction::I32Add => bytes.push(0x6A),
        WasmInstruction::I32Sub => bytes.push(0x6B),
        WasmInstruction::I32Mul => bytes.push(0x6C),
        WasmInstruction::I32DivS => bytes.push(0x6D),
        WasmInstruction::I64Add => bytes.push(0x7C),
        WasmInstruction::I64Sub => bytes.push(0x7D),
        WasmInstruction::I64Mul => bytes.push(0x7E),
        WasmInstruction::I64DivS => bytes.push(0x7F),
        WasmInstruction::F32Add => bytes.push(0x92),
        WasmInstruction::F32Sub => bytes.push(0x93),
        WasmInstruction::F32Mul => bytes.push(0x94),
        WasmInstruction::F32Div => bytes.push(0x95),
        WasmInstruction::F64Add => bytes.push(0xA0),
        WasmInstruction::F64Sub => bytes.push(0xA1),
        WasmInstruction::F64Mul => bytes.push(0xA2),
        WasmInstruction::F64Div => bytes.push(0xA3),
        WasmInstruction::LocalGet(idx) => {
            bytes.push(0x20);
            bytes.push(*idx as u8);
        }
        WasmInstruction::LocalSet(idx) => {
            bytes.push(0x21);
            bytes.push(*idx as u8);
        }
        WasmInstruction::LocalTee(idx) => {
            bytes.push(0x22);
            bytes.push(*idx as u8);
        }
        WasmInstruction::GlobalGet(idx) => {
            bytes.push(0x23);
            bytes.push(*idx as u8);
        }
        WasmInstruction::GlobalSet(idx) => {
            bytes.push(0x24);
            bytes.push(*idx as u8);
        }
        WasmInstruction::Call(idx) => {
            bytes.push(0x10);
            bytes.push(*idx as u8);
        }
        WasmInstruction::Return => bytes.push(0x0F),
        WasmInstruction::Drop => bytes.push(0x1A),
        WasmInstruction::Nop => bytes.push(0x01),
        WasmInstruction::Unreachable => bytes.push(0x00),
        WasmInstruction::MemorySize => {
            bytes.push(0x3F);
            bytes.push(0x00);
        }
        WasmInstruction::MemoryGrow => {
            bytes.push(0x40);
            bytes.push(0x00);
        }
        WasmInstruction::I32Load { offset, align } => {
            bytes.push(0x28);
            bytes.push(*align as u8);
            bytes.push(*offset as u8);
        }
        WasmInstruction::I32Store { offset, align } => {
            bytes.push(0x36);
            bytes.push(*align as u8);
            bytes.push(*offset as u8);
        }
        WasmInstruction::I64Load { offset, align } => {
            bytes.push(0x29);
            bytes.push(*align as u8);
            bytes.push(*offset as u8);
        }
        WasmInstruction::I64Store { offset, align } => {
            bytes.push(0x37);
            bytes.push(*align as u8);
            bytes.push(*offset as u8);
        }
        WasmInstruction::Block(instrs) => {
            bytes.push(0x02);
            bytes.push(0x40); // void block type
            for instr in instrs {
                emit_instruction(instr, bytes);
            }
            bytes.push(0x0B); // end
        }
        WasmInstruction::Loop(instrs) => {
            bytes.push(0x03);
            bytes.push(0x40); // void block type
            for instr in instrs {
                emit_instruction(instr, bytes);
            }
            bytes.push(0x0B); // end
        }
        WasmInstruction::If {
            condition,
            then_branch,
            else_branch,
        } => {
            emit_instruction(condition, bytes);
            bytes.push(0x04);
            bytes.push(0x40); // void block type
            for instr in then_branch {
                emit_instruction(instr, bytes);
            }
            if let Some(else_instrs) = else_branch {
                bytes.push(0x05); // else
                for instr in else_instrs {
                    emit_instruction(instr, bytes);
                }
            }
            bytes.push(0x0B); // end
        }
        WasmInstruction::CallIndirect(idx) => {
            bytes.push(0x11);
            bytes.push(*idx as u8);
            bytes.push(0x00); // table index
        }
    }
}

pub fn compile_to_wasm(forms: &[Value]) -> Result<Vec<u8>, WasmError> {
    let mut compiler = WasmCompiler::new();
    compiler.compile(forms)
}
