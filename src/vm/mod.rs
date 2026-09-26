mod calls;
use crate::compiler::Instruction;
use crate::object::Object;
use crate::symbol::{Interner, SymbolId};
use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use calls::{Activation, Code};
use core::fmt;

/// Borrowed VM state after an instruction. Hosts choose where to send traces.
pub struct TraceEvent<'a> {
    pub instruction: &'a Instruction,
    pub ip: usize,
    pub stack: &'a [Object],
    pub globals: &'a BTreeMap<SymbolId, Object>,
    pub frames: &'a [CallFrame],
}

impl fmt::Display for TraceEvent<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "  [IP:{:03}] {:?} | Stack: {:?} | Globals: {:?} | Frames ({}): {:?}",
            self.ip,
            self.instruction,
            self.stack,
            self.globals,
            self.frames.len(),
            self.frames
        )
    }
}

/// A call frame representing a lexical scope during execution.
///
/// Each frame contains:
/// - Local variables stored in slots
/// - An instruction pointer offset (currently unused but reserved for future use)
///
/// Frames are pushed when entering a block and popped when exiting.
/// The global scope is represented by the bottom-most frame.
#[derive(Debug)]
pub struct CallFrame {
    /// Local variables stored in slots indexed from 0.
    locals: Vec<Object>,
    /// The instruction pointer offset when this frame was created.
    /// Currently unused but reserved for future stack traces or debugging.
    _ip_offset: usize,
}

impl CallFrame {
    /// Creates a new call frame with the specified number of local variable slots.
    ///
    /// All slots are initialized to `Object::none()`.
    pub fn new(ip_offset: usize, num_locals: usize) -> Self {
        Self {
            locals: vec![Object::none(); num_locals],
            _ip_offset: ip_offset,
        }
    }
}

/// A stack-based virtual machine that executes bytecode instructions.
///
/// The VM maintains:
/// - An operand stack for intermediate values
/// - A stack of call frames for lexical scoping
/// - A global variable table
///
/// Instructions operate on the operand stack, pushing results and popping
/// operands. Local variables are accessed via frame slots, while global
/// variables are accessed via symbol IDs.
///
/// The VM executes instructions sequentially, incrementing the instruction
/// pointer after each instruction unless a jump instruction modifies it.
pub struct VM {
    /// The operand stack for intermediate values.
    stack: Vec<Object>,
    /// Last expression statement evaluated in the current run.
    result: Option<Object>,
    /// A stack of call frames. The last element is the current frame.
    frames: Vec<CallFrame>,
    /// Global variables indexed by symbol ID.
    globals: BTreeMap<SymbolId, Object>,
    writes: Vec<(Object, String, Object)>,
    call_limit: usize,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    /// Creates a new virtual machine with an empty stack and a global frame.
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            result: None,
            frames: vec![CallFrame::new(0, 0)],
            globals: BTreeMap::new(),
            writes: Vec::new(),
            call_limit: 128,
        }
    }

    /// Resolves a lexical address without permitting an invalid frame index.
    fn local_frame(&mut self, depth: usize) -> Result<&mut CallFrame, String> {
        let index = self
            .frames
            .len()
            .checked_sub(depth)
            .and_then(|n| n.checked_sub(1))
            .ok_or("SystemError: local frame depth out of bounds")?;
        self.frames
            .get_mut(index)
            .ok_or_else(|| "SystemError: local frame depth out of bounds".to_string())
    }

    /// Executes a sequence of bytecode instructions and returns the final result.
    ///
    /// The VM processes instructions sequentially, maintaining an instruction
    /// pointer that advances after each instruction unless modified by a jump.
    ///
    /// Failed runs roll back all global writes. Operands and temporary frames are
    /// discarded on every exit; the result is the last executed expression statement.
    ///
    /// Use [`VM::run_with_trace`] for host-controlled diagnostic output.
    pub fn run(&mut self, bytecode: &[Instruction], interner: &Interner) -> Result<Object, String> {
        self.run_with_trace(bytecode, interner, |_| {})
    }

    /// Executes with a callback after each successfully executed instruction.
    /// The callback only borrows state; no I/O or OS services are required.
    pub fn run_with_trace(
        &mut self,
        bytecode: &[Instruction],
        interner: &Interner,
        mut trace: impl FnMut(&TraceEvent<'_>),
    ) -> Result<Object, String> {
        let globals = self.globals.clone();
        let outcome = self.run_instructions(bytecode, interner, &mut trace);
        self.stack.clear();
        self.frames.truncate(1);
        self.result = None;
        if outcome.is_err() {
            self.globals = globals;
            for (object, name, old) in self.writes.drain(..).rev() {
                // Both values have the same immutable descriptor; the field
                // cannot disappear. Rollback never runs user code.
                let _ = object.replace_field(&name, old);
            }
        }
        self.writes.clear();
        outcome
    }

    /// Bound recursive calls by VM frames, not by the host's native stack.
    pub fn set_call_limit(&mut self, limit: usize) {
        self.call_limit = limit;
    }

    fn run_instructions(
        &mut self,
        bytecode: &[Instruction],
        interner: &Interner,
        trace: &mut impl FnMut(&TraceEvent<'_>),
    ) -> Result<Object, String> {
        let mut activations = vec![Activation::root(bytecode)];
        loop {
            let activation = activations
                .last_mut()
                .ok_or("SystemError: missing activation")?;
            let Some(instruction) = activation.code.instructions().get(activation.ip).cloned()
            else {
                if activations.len() == 1 {
                    return Ok(self
                        .stack
                        .pop()
                        .or_else(|| self.result.take())
                        .unwrap_or_else(Object::none));
                }
                return Err("SystemError: function ended without RETURN".to_string());
            };
            activation.ip += 1;
            let ip = activation.ip;
            match &instruction {
                Instruction::Jump(target) => activation.ip = *target,
                Instruction::JumpIfFalse(target) => {
                    if let Some(target) = self.execute_jump_if_false(*target)? {
                        activation.ip = target;
                    }
                }
                Instruction::Call(arity) => {
                    if let Some(invocation) = self.prepare_call(*arity)? {
                        if activations.len() > self.call_limit {
                            return Err("RecursionError: call limit exceeded".to_string());
                        }
                        let frames = self.frames.len();
                        let stack = self.stack.len();
                        let mut frame = CallFrame::new(0, invocation.function.locals);
                        if invocation.arguments.len() > frame.locals.len() {
                            return Err("SystemError: invalid function frame".to_string());
                        }
                        for (slot, argument) in frame.locals.iter_mut().zip(invocation.arguments) {
                            *slot = argument;
                        }
                        self.frames.push(frame);
                        activations.push(Activation {
                            code: Code::Function(invocation.function),
                            ip: 0,
                            frame_base: frames,
                            stack_base: stack,
                            saved_result: self.result.take(),
                            constructor: invocation.constructor,
                        });
                    }
                }
                Instruction::Return => {
                    if activations.len() == 1 {
                        return Err("SystemError: RETURN outside function".to_string());
                    }
                    let value = self
                        .stack
                        .pop()
                        .ok_or("SystemError: missing return value")?;
                    let activation = activations.pop().ok_or("SystemError: missing activation")?;
                    self.frames.truncate(activation.frame_base);
                    self.stack.truncate(activation.stack_base);
                    self.result = activation.saved_result;
                    self.stack.push(activation.constructor.unwrap_or(value));
                }
                _ => self.execute_instruction(&instruction, interner, ip - 1)?,
            }
            self.trace_state(&instruction, ip, trace);
        }
    }

    /// Executes a single instruction.
    ///
    /// This method dispatches to specialized handlers for each instruction type.
    fn execute_instruction(
        &mut self,
        instruction: &Instruction,
        interner: &Interner,
        ip: usize,
    ) -> Result<(), String> {
        match instruction {
            Instruction::CreateClass(code) => {
                if self.stack.len() < code.fields.len() {
                    return Err("SystemError: missing class defaults".to_string());
                }
                let values = self.stack.split_off(self.stack.len() - code.fields.len());
                let defaults = code.fields.iter().cloned().zip(values).collect();
                self.stack.push(Object::class(code.clone(), defaults));
            }
            Instruction::GetAttribute(name) => {
                let object = self
                    .stack
                    .pop()
                    .ok_or("SystemError: missing attribute receiver")?;
                self.stack.push(object.get_attribute(name)?);
            }
            Instruction::SetAttribute(name) => {
                let value = self.stack.pop().ok_or("SystemError: missing field value")?;
                let object = self
                    .stack
                    .pop()
                    .ok_or("SystemError: missing field receiver")?;
                let old = object.replace_field(name, value)?;
                self.writes.push((object, name.clone(), old));
            }
            Instruction::Call(_) | Instruction::Return => {
                return Err("SystemError: misplaced call instruction".to_string());
            }
            Instruction::LoadConst(val) => {
                self.stack.push(val.clone());
            }

            Instruction::LoadName(sym_id) => {
                self.execute_load_name(*sym_id, interner)?;
            }

            Instruction::StoreName(sym_id) => {
                self.execute_store_name(*sym_id)?;
            }

            Instruction::LoadLocal(slot) => {
                self.execute_load_local(0, *slot)?;
            }

            Instruction::StoreLocal(slot) => {
                self.execute_store_local(0, *slot)?;
            }

            Instruction::LoadEnclosing { depth, slot } => self.execute_load_local(*depth, *slot)?,
            Instruction::StoreEnclosing { depth, slot } => {
                self.execute_store_local(*depth, *slot)?
            }
            Instruction::SetResult => {
                self.result = Some(
                    self.stack
                        .pop()
                        .ok_or("SystemError: stack underflow at SET_RESULT")?,
                );
            }

            Instruction::EnterBlock(num_locals) => {
                self.frames.push(CallFrame::new(ip, *num_locals));
            }

            Instruction::ExitBlock => {
                self.execute_exit_block()?;
            }

            Instruction::Add => self.execute_binary_op(|a, b| a.add(b))?,
            Instruction::Subtract => self.execute_binary_op(|a, b| a.sub(b))?,
            Instruction::Multiply => self.execute_binary_op(|a, b| a.mul(b))?,
            Instruction::Divide => self.execute_binary_op(|a, b| a.div(b))?,

            Instruction::Eq => self.execute_compare_op(|a, b| a.eq(b))?,
            Instruction::NotEq => self.execute_compare_op(|a, b| a.ne(b))?,
            Instruction::Greater => self.execute_compare_op(|a, b| a.gt(b))?,
            Instruction::GreaterEq => self.execute_compare_op(|a, b| a.ge(b))?,
            Instruction::Less => self.execute_compare_op(|a, b| a.lt(b))?,
            Instruction::LessEq => self.execute_compare_op(|a, b| a.le(b))?,

            Instruction::Jump(_target) => {
                // Jump is handled in the main loop to modify ip
                unreachable!("Jump should be handled in main loop");
            }

            Instruction::JumpIfFalse(target) => {
                self.execute_jump_if_false(*target)?;
            }
        }

        Ok(())
    }

    /// Executes a LOAD_NAME instruction.
    ///
    /// Loads a global variable by symbol ID and pushes it onto the stack.
    fn execute_load_name(&mut self, sym_id: SymbolId, interner: &Interner) -> Result<(), String> {
        let value = self.globals.get(&sym_id).ok_or_else(|| {
            let name = interner.resolve(sym_id);
            format!("NameError: name '{}' is not defined", name)
        })?;
        self.stack.push(value.clone());
        Ok(())
    }

    /// Executes a STORE_NAME instruction.
    ///
    /// Pops a value from the stack and stores it in a global variable.
    fn execute_store_name(&mut self, sym_id: SymbolId) -> Result<(), String> {
        let value = self
            .stack
            .pop()
            .ok_or("SystemError: stack underflow at STORE_NAME")?;
        self.globals.insert(sym_id, value);
        Ok(())
    }

    /// Executes a LOAD_LOCAL instruction.
    ///
    /// Loads a local variable by slot index and pushes it onto the stack.
    fn execute_load_local(&mut self, depth: usize, slot: usize) -> Result<(), String> {
        let frame = self.local_frame(depth)?;
        if slot >= frame.locals.len() {
            return Err(format!("SystemError: local slot {} out of bounds", slot));
        }
        let value = frame.locals[slot].clone();
        self.stack.push(value);
        Ok(())
    }

    /// Executes a STORE_LOCAL instruction.
    ///
    /// Pops a value from the stack and stores it in a local variable slot.
    fn execute_store_local(&mut self, depth: usize, slot: usize) -> Result<(), String> {
        let value = self
            .stack
            .pop()
            .ok_or("SystemError: stack underflow at STORE_LOCAL")?;

        let frame = self.local_frame(depth)?;
        if slot >= frame.locals.len() {
            return Err(format!("SystemError: local slot {} out of bounds", slot));
        }
        frame.locals[slot] = value;
        Ok(())
    }

    /// Executes an EXIT_BLOCK instruction.
    ///
    /// Pops the current call frame and returns to the parent scope.
    fn execute_exit_block(&mut self) -> Result<(), String> {
        if self.frames.len() <= 1 {
            return Err("SystemError: cannot exit global frame".to_string());
        }
        self.frames.pop();
        Ok(())
    }

    /// Executes a binary operation on the top two stack values.
    ///
    /// Pops two values, applies the operation, and pushes the result.
    fn execute_binary_op<F>(&mut self, op: F) -> Result<(), String>
    where
        F: FnOnce(&Object, &Object) -> Result<Object, String>,
    {
        let right = self
            .stack
            .pop()
            .ok_or("SystemError: stack underflow (right operand)")?;
        let left = self
            .stack
            .pop()
            .ok_or("SystemError: stack underflow (left operand)")?;
        let result = op(&left, &right)?;
        self.stack.push(result);
        Ok(())
    }

    /// Executes a comparison operation on the top two stack values.
    ///
    /// Pops two values, applies the comparison, and pushes a boolean result.
    fn execute_compare_op<F>(&mut self, op: F) -> Result<(), String>
    where
        F: FnOnce(&Object, &Object) -> Result<bool, String>,
    {
        let right = self
            .stack
            .pop()
            .ok_or("SystemError: stack underflow (right operand)")?;
        let left = self
            .stack
            .pop()
            .ok_or("SystemError: stack underflow (left operand)")?;
        let result = op(&left, &right)?;
        self.stack.push(Object::bool(result));
        Ok(())
    }

    /// Executes a JUMP_IF_FALSE instruction.
    ///
    /// Pops a boolean from the stack and jumps to the target if it is false.
    /// Returns the new instruction pointer, or None if no jump occurred.
    fn execute_jump_if_false(&mut self, target: usize) -> Result<Option<usize>, String> {
        let condition = self
            .stack
            .pop()
            .ok_or("SystemError: stack underflow at JUMP_IF_FALSE")?;

        match condition.as_bool() {
            Some(false) => Ok(Some(target)),
            Some(true) => Ok(None),
            _ => Err(format!(
                "TypeError: condition must be bool, got {}",
                condition.type_name()
            )),
        }
    }

    fn trace_state(
        &self,
        instruction: &Instruction,
        ip: usize,
        trace: &mut impl FnMut(&TraceEvent<'_>),
    ) {
        trace(&TraceEvent {
            instruction,
            ip,
            stack: &self.stack,
            globals: &self.globals,
            frames: &self.frames,
        });
    }
}
