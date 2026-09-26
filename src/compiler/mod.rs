mod definitions;
use crate::bytecode::{ClassCode, FunctionCode};
use crate::object::Object;
use crate::parser::{Expr, Operator, Stmt};
use crate::symbol::{Interner, SymbolId};
use crate::types::Type;
use alloc::{collections::BTreeSet, vec, vec::Vec};
use alloc::{rc::Rc, string::String};

/// A bytecode instruction for the virtual machine.
///
/// This enum represents all operations that the VM can execute.
/// Instructions are stack-based: they operate on values pushed onto
/// the operand stack and may push results back onto the stack.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Call(usize),
    Return,
    GetAttribute(String),
    SetAttribute(String),
    CreateClass(Rc<ClassCode>),
    /// Pushes a constant value onto the stack.
    LoadConst(Object),

    /// Loads a global variable by symbol ID and pushes it onto the stack.
    LoadName(SymbolId),

    /// Pops a value from the stack and stores it in a global variable.
    StoreName(SymbolId),

    /// Loads a local variable by slot index and pushes it onto the stack.
    LoadLocal(usize),

    /// Pops a value from the stack and stores it in a local variable slot.
    StoreLocal(usize),

    /// Loads a local in an enclosing block (`depth` frames outward).
    LoadEnclosing {
        depth: usize,
        slot: usize,
    },

    /// Stores a local in an enclosing block (`depth` frames outward).
    StoreEnclosing {
        depth: usize,
        slot: usize,
    },

    /// Pops an expression statement's value into the run's result register.
    SetResult,

    /// Enters a new block with the specified number of local variables.
    /// The VM allocates space for these locals on the call stack.
    EnterBlock(usize),

    /// Exits the current block and deallocates its local variables.
    ExitBlock,

    /// Pops two integers, adds them, and pushes the result.
    Add,

    /// Pops two integers, subtracts the second from the first, and pushes the result.
    Subtract,

    /// Pops two integers, multiplies them, and pushes the result.
    Multiply,

    /// Pops two integers, divides the first by the second, and pushes the result.
    /// The VM checks for division by zero at runtime.
    Divide,

    /// Pops two values, compares them for equality, and pushes a boolean result.
    Eq,

    /// Pops two values, compares them for inequality, and pushes a boolean result.
    NotEq,

    /// Pops two integers, checks if the first is less than the second, and pushes a boolean.
    Less,

    /// Pops two integers, checks if the first is greater than the second, and pushes a boolean.
    Greater,

    /// Pops two integers, checks if the first is less than or equal to the second, and pushes a boolean.
    LessEq,

    /// Pops two integers, checks if the first is greater than or equal to the second, and pushes a boolean.
    GreaterEq,

    /// Unconditionally jumps to the instruction at the given address.
    Jump(usize),

    /// Pops a boolean from the stack and jumps to the given address if it is false.
    JumpIfFalse(usize),
}

/// A lexical scope during compilation.
///
/// Each scope tracks the local variables declared within it.
/// Local variables are stored in slots indexed from 0.
pub struct CompilerScope {
    /// The symbol IDs of local variables in declaration order.
    /// The index in this vector is the slot number.
    locals: Vec<SymbolId>,
}

/// A compiler that translates AST nodes into bytecode instructions.
///
/// The compiler performs a single pass over the AST, emitting instructions
/// as it visits each node. It maintains a stack of scopes to support
/// nested blocks and local variables.
///
/// The compiler distinguishes between global variables (accessed via
/// `LoadName`/`StoreName`) and local variables (accessed via `LoadLocal`/
/// `StoreLocal`). Variables declared at the top level are global; variables
/// declared inside blocks are local.
pub struct Compiler {
    /// A stack of lexical scopes. The last element is the current scope.
    scopes: Vec<CompilerScope>,
    /// The bytecode instructions being generated.
    code: Vec<Instruction>,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Compiler {
    /// Creates a new compiler with an empty global scope.
    pub fn new() -> Self {
        Self {
            scopes: vec![CompilerScope { locals: Vec::new() }],
            code: Vec::new(),
        }
    }

    // --- Helpers ---

    /// Emits an instruction to the bytecode stream.
    fn emit(&mut self, instruction: Instruction) {
        self.code.push(instruction);
    }

    /// Returns the address (index) of the next instruction to be emitted.
    fn current_address(&self) -> usize {
        self.code.len()
    }

    /// Emits a jump instruction with a placeholder target address.
    ///
    /// Returns the index of the emitted instruction so it can be patched later.
    fn emit_jump_placeholder(&mut self, instruction: Instruction) -> usize {
        let idx = self.code.len();
        self.code.push(instruction);
        idx
    }

    /// Patches a previously emitted jump instruction with its target address.
    ///
    /// # Panics
    ///
    /// Panics if the instruction at `idx` is not a jump instruction.
    fn patch_jump(&mut self, idx: usize, target: usize) {
        match &mut self.code[idx] {
            Instruction::Jump(t) => *t = target,
            Instruction::JumpIfFalse(t) => *t = target,
            _ => panic!("Cannot patch non-jump instruction at index {}", idx),
        }
    }

    // --- Scope management ---

    /// Returns a mutable reference to the current (innermost) scope.
    ///
    /// # Panics
    ///
    /// Panics if the scope stack is empty, which should never happen.
    fn current_scope_mut(&mut self) -> &mut CompilerScope {
        self.scopes
            .last_mut()
            .expect("scope stack should never be empty")
    }

    /// Enters a new nested scope.
    fn enter_scope(&mut self) {
        self.scopes.push(CompilerScope { locals: Vec::new() });
    }

    /// Exits the current scope and returns to the parent scope.
    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    /// Declares a local variable in the current scope and returns its slot index.
    fn declare_local(&mut self, sym_id: SymbolId) -> usize {
        let scope = self.current_scope_mut();
        let index = scope.locals.len();
        scope.locals.push(sym_id);
        index
    }

    /// Resolves the nearest local to its lexical depth and slot, excluding globals.
    fn resolve_local(&self, sym_id: SymbolId) -> Option<(usize, usize)> {
        self.scopes
            .iter()
            .skip(1)
            .rev()
            .enumerate()
            .find_map(|(depth, scope)| {
                scope
                    .locals
                    .iter()
                    .position(|&id| id == sym_id)
                    .map(|slot| (depth, slot))
            })
    }

    /// Returns true if the compiler is currently inside a nested block.
    fn is_in_block(&self) -> bool {
        self.scopes.len() > 1
    }

    /// Counts the number of unique local variables declared in a block.
    ///
    /// This is used to determine how much space to allocate for locals
    /// when entering the block.
    fn count_block_locals(&self, stmts: &[Stmt], interner: &mut Interner) -> usize {
        let mut seen = BTreeSet::new();
        for stmt in stmts {
            if let Stmt::VariableDecl { name, .. } = stmt {
                let sym_id = interner.intern(name);
                seen.insert(sym_id);
            }
        }
        seen.len()
    }

    // --- Block compile ---

    /// Compiles a block of statements, emitting EnterBlock/ExitBlock instructions.
    ///
    /// This method counts the number of local variables in the block, emits
    /// an `EnterBlock` instruction to allocate space for them, compiles all
    /// statements in a new scope, and then emits an `ExitBlock` instruction
    /// to deallocate the locals.
    fn compile_block(&mut self, stmts: &[Stmt], interner: &mut Interner) {
        let num_locals = self.count_block_locals(stmts, interner);
        self.emit(Instruction::EnterBlock(num_locals));

        self.enter_scope();
        for stmt in stmts {
            self.compile_stmt(stmt, interner);
        }
        self.exit_scope();

        self.emit(Instruction::ExitBlock);
    }

    // --- Main entry point ---

    /// Compiles a list of statements into bytecode instructions.
    ///
    /// This is the main entry point for compilation. It processes each
    /// statement in order and returns the generated instruction stream.
    pub fn compile(mut self, stmts: &[Stmt], interner: &mut Interner) -> Vec<Instruction> {
        for stmt in stmts {
            self.compile_stmt(stmt, interner);
        }
        self.code
    }

    // --- Statement compilation ---

    /// Compiles a single statement.
    fn compile_stmt(&mut self, stmt: &Stmt, interner: &mut Interner) {
        match stmt {
            Stmt::Function(function) => {
                let code = Self::compile_function(function, interner);
                self.emit(Instruction::LoadConst(Object::function(code)));
                self.emit(Instruction::StoreName(interner.intern(&function.name)));
            }
            Stmt::Class { name, body } => self.compile_class(name, body, interner),
            Stmt::Return(value) => {
                if let Some(value) = value {
                    self.compile_expr(value, interner);
                } else {
                    self.emit(Instruction::LoadConst(Object::none()));
                }
                self.emit(Instruction::Return);
            }
            Stmt::SetAttribute {
                object,
                name,
                value,
            } => {
                self.compile_expr(object, interner);
                self.compile_expr(value, interner);
                self.emit(Instruction::SetAttribute(name.clone()));
            }
            Stmt::Pass => {}
            Stmt::Expr(expr) => {
                self.compile_expr(expr, interner);
                self.emit(Instruction::SetResult);
            }

            Stmt::VariableDecl {
                name,
                typ,
                initializer,
            } => {
                let sym_id = interner.intern(name);

                // Compile the initializer or a default value
                if let Some(init) = initializer {
                    self.compile_expr(init, interner);
                } else {
                    // Emit a default value based on the declared type
                    match typ {
                        Type::Int => self.emit(Instruction::LoadConst(Object::int(0))),
                        Type::Bool => self.emit(Instruction::LoadConst(Object::bool(false))),
                        _ => self.emit(Instruction::LoadConst(Object::none())),
                    }
                }

                // Store the value in the appropriate location
                if self.is_in_block() {
                    let slot = self.declare_local(sym_id);
                    self.emit(Instruction::StoreLocal(slot));
                } else {
                    self.emit(Instruction::StoreName(sym_id));
                }
            }

            Stmt::Assign { name, value } => {
                self.compile_expr(value, interner);
                let sym_id = interner.intern(name);

                // Store the value in the appropriate location
                if let Some((depth, slot)) = self.resolve_local(sym_id) {
                    self.emit(if depth == 0 {
                        Instruction::StoreLocal(slot)
                    } else {
                        Instruction::StoreEnclosing { depth, slot }
                    });
                } else {
                    self.emit(Instruction::StoreName(sym_id));
                }
            }

            Stmt::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
            } => {
                self.compile_if(condition, then_branch, elif_branches, else_branch, interner);
            }
        }
    }

    /// Compiles an if statement with optional elif and else branches.
    ///
    /// This method generates code for the condition, emits a conditional jump
    /// to skip the then-branch if false, compiles the then-branch, and then
    /// handles elif and else branches similarly. All branches jump to a common
    /// end point after execution.
    fn compile_if(
        &mut self,
        condition: &Expr,
        then_branch: &[Stmt],
        elif_branches: &[(Expr, Vec<Stmt>)],
        else_branch: &Option<Vec<Stmt>>,
        interner: &mut Interner,
    ) {
        let mut end_jumps = Vec::new();

        // Compile the main if condition
        self.compile_expr(condition, interner);
        let mut false_jump_idx = self.emit_jump_placeholder(Instruction::JumpIfFalse(0));

        // Compile the then branch
        self.compile_block(then_branch, interner);

        // Jump to the end after the then branch
        let jump_end_idx = self.emit_jump_placeholder(Instruction::Jump(0));
        end_jumps.push(jump_end_idx);

        // Compile each elif branch
        for (elif_condition, elif_branch) in elif_branches {
            let elif_start = self.current_address();
            self.patch_jump(false_jump_idx, elif_start);

            self.compile_expr(elif_condition, interner);
            false_jump_idx = self.emit_jump_placeholder(Instruction::JumpIfFalse(0));

            self.compile_block(elif_branch, interner);

            let jump_end_idx = self.emit_jump_placeholder(Instruction::Jump(0));
            end_jumps.push(jump_end_idx);
        }

        // Compile the else branch (if present) or mark the end
        let fallback_start = self.current_address();
        self.patch_jump(false_jump_idx, fallback_start);

        if let Some(else_block) = else_branch {
            self.compile_block(else_block, interner);
        }

        // Patch all end jumps to point here
        let end = self.current_address();
        for jump_idx in end_jumps {
            self.patch_jump(jump_idx, end);
        }
    }

    // --- Expression compilation ---

    /// Compiles an expression, pushing its result onto the stack.
    fn compile_expr(&mut self, expr: &Expr, interner: &mut Interner) {
        match expr {
            Expr::None => self.emit(Instruction::LoadConst(Object::none())),
            Expr::Call { callee, arguments } => {
                self.compile_expr(callee, interner);
                for argument in arguments {
                    self.compile_expr(argument, interner);
                }
                self.emit(Instruction::Call(arguments.len()));
            }
            Expr::Attribute { object, name } => {
                self.compile_expr(object, interner);
                self.emit(Instruction::GetAttribute(name.clone()));
            }
            Expr::Number(n) => {
                self.emit(Instruction::LoadConst(Object::int(*n)));
            }
            Expr::Bool(b) => {
                self.emit(Instruction::LoadConst(Object::bool(*b)));
            }
            Expr::Name(name) => {
                let sym_id = interner.intern(name);
                // Try to load as a local first, fall back to global
                if let Some((depth, slot)) = self.resolve_local(sym_id) {
                    self.emit(if depth == 0 {
                        Instruction::LoadLocal(slot)
                    } else {
                        Instruction::LoadEnclosing { depth, slot }
                    });
                } else {
                    self.emit(Instruction::LoadName(sym_id));
                }
            }
            Expr::BinaryOp { left, op, right } => {
                // Compile operands
                self.compile_expr(left, interner);
                self.compile_expr(right, interner);

                // Emit the appropriate operator instruction
                match op {
                    Operator::Plus => self.emit(Instruction::Add),
                    Operator::Minus => self.emit(Instruction::Subtract),
                    Operator::Star => self.emit(Instruction::Multiply),
                    Operator::Slash => self.emit(Instruction::Divide),
                    Operator::Eq => self.emit(Instruction::Eq),
                    Operator::NotEq => self.emit(Instruction::NotEq),
                    Operator::Greater => self.emit(Instruction::Greater),
                    Operator::GreaterEq => self.emit(Instruction::GreaterEq),
                    Operator::Less => self.emit(Instruction::Less),
                    Operator::LessEq => self.emit(Instruction::LessEq),
                }
            }
        }
    }
}
