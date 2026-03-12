use std::fmt;

use indexmap::IndexMap;
use itertools::{Either, Itertools};
use mcltl::ltl::expression::Literal;

use crate::{
    ast::{
        AExpr, AOp, Array, BExpr, BufferSize, CG, Channel, ChannelFormula, Command, CommandKind,
        Commands, Field, Function, Int, LTLFormula, Locator, LogicOp, Operation, RelOp, Target,
        TupleSpace, TupleSpaceType, Variable,
    },
    ast_ext::FreeVariables,
    parse::SourceSpan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InstrPtr(u32);
impl InstrPtr {
    fn bump(&self) -> InstrPtr {
        InstrPtr(self.0 + 1)
    }
}

#[derive(Debug)]
enum Instr {
    Nop,
    Assign(Target<Box<AExpr>>, AExpr),
    Branch {
        choices: Vec<(CG, InstrPtr)>,
        otherwise: Option<InstrPtr>,
    },
    Goto(InstrPtr),
    Halt,
    Put(BufferSize, u32, Vec<AExpr>),
    Get(TupleSpaceType, u32, Vec<Field>),
    Query(TupleSpaceType, u32, Vec<Field>),
    Send(BufferSize, u32, AExpr),
    Receive(u32, Target<Box<AExpr>>),
    SyncSend {
        channel: String,
        expr: AExpr,
    },
    SyncReceive {
        channel: String,
        target: Target<Box<AExpr>>,
    },
    Broadcast {
        channel: String,
        k: Int,
        expr: AExpr,
    },
    Gather {
        channel: String,
        k: Int,
        array: Array,
        target: Target<Box<AExpr>>,
    },
}

#[derive(Debug)]
pub struct Program {
    variables: Vec<Variable>,
    arrays: Vec<ArrayMeta>,
    tuple_spaces: Vec<TupleSpaceMeta>,
    channels: Vec<ChannelMeta>,
    instrs: Vec<Instr>,
    entry_points: Vec<InstrPtr>,
    source_map: Vec<Option<SourceSpan>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayMeta {
    pub name: Array,
    pub base_index: u32,
    pub length: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleSpaceMeta {
    pub name: Variable,
    pub space_type: TupleSpaceType,
    pub size: BufferSize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelMeta {
    pub name: Variable,
    pub size: BufferSize,
}

impl std::ops::Index<InstrPtr> for Program {
    type Output = Instr;

    fn index(&self, ptr: InstrPtr) -> &Instr {
        &self.instrs[ptr.0 as usize]
    }
}

impl Program {
    pub fn compile(
        cmdss: &[Commands<(), ()>],
        additional_vars: impl IntoIterator<Item = Variable>,
        tuple_spaces: IndexMap<Variable, TupleSpace>,
        channels: IndexMap<Variable, Channel>,
        arrays: IndexMap<Array, Vec<i32>>,
    ) -> Program {
        let variables: Vec<_> = cmdss
            .iter()
            .flat_map(|cmds| {
                cmds.fv().into_iter().filter_map(|t| match t {
                    Target::Variable(var) => Some(var),
                    Target::Array(_, _) => None,
                })
            })
            .chain(additional_vars)
            .sorted()
            .dedup()
            .collect();

        let mut array_meta = Vec::new();
        let mut current_index = variables.len() as u32;

        for (name, values) in arrays {
            array_meta.push(ArrayMeta {
                name,
                base_index: current_index,
                length: values.len() as u32,
            });
            current_index += values.len() as u32;
        }

        let mut p = Program {
            variables,
            arrays: array_meta,
            instrs: Vec::new(),
            entry_points: Vec::new(),
            source_map: Vec::new(),
            tuple_spaces: tuple_spaces
                .into_iter()
                .map(|(var, ts)| TupleSpaceMeta {
                    name: var,
                    space_type: ts.space_type,
                    size: ts.size,
                })
                .collect(),
            channels: channels
                .into_iter()
                .map(|(var, ch)| ChannelMeta {
                    name: var,
                    size: ch.size,
                })
                .collect(),
        };

        for cmds in cmdss {
            let entry = p.current();
            p.entry_points.push(entry);
            p.compile_commands(cmds);
            p.push(
                Instr::Halt,
                cmds.0.last().map(|cmd| cmd.span.cursor_at_end()),
            );
        }

        p
    }

    pub fn initial_state(
        &self,
        var_init: impl Fn(&Variable) -> i32,
        arr_init: impl Fn(&Array) -> Vec<i32>,
        tuple_space_memory: Vec<Vec<Vec<Int>>>,
        channel_memory: Vec<Vec<Int>>,
    ) -> State {
        let mut memory = self.variables.iter().map(var_init).collect::<Vec<_>>();

        for ArrayMeta { name, .. } in self.arrays.iter() {
            let arr_values = arr_init(name);
            memory.extend_from_slice(&arr_values);
        }

        State {
            ptrs: self.entry_points.clone(),
            memory,
            tuple_spaces: tuple_space_memory,
            channels: channel_memory,
        }
    }

    pub fn variables(&self) -> impl Iterator<Item = &'_ Variable> {
        self.variables.iter()
    }

    fn variable_index(&self, name: &str) -> Option<u32> {
        self.variables
            .iter()
            .position(|v| v.0 == name)
            .map(|idx| idx as _)
    }

    fn array_meta(&self, arr: &Array) -> Option<(u32, u32)> {
        self.arrays
            .iter()
            .find(|ArrayMeta { name, .. }| name == arr)
            .map(
                |ArrayMeta {
                     base_index, length, ..
                 }| (*base_index, *length),
            )
    }

    fn tuple_space_index(&self, name: &str) -> Option<u32> {
        self.tuple_spaces
            .iter()
            .position(|v| v.name.0 == name)
            .map(|idx| idx as _)
    }

    fn channel_index(&self, name: &str) -> Option<u32> {
        self.channels
            .iter()
            .position(|v| v.name.0 == name)
            .map(|idx| idx as _)
    }

    fn current(&self) -> InstrPtr {
        InstrPtr(self.instrs.len() as _)
    }

    fn push(&mut self, instr: Instr, src: Option<SourceSpan>) -> InstrPtr {
        let ptr = self.current();
        self.instrs.push(instr);
        self.source_map.push(src);
        ptr
    }

    fn set(&mut self, ptr: InstrPtr, instr: Instr) {
        self.instrs[ptr.0 as usize] = instr;
    }

    fn compile_commands(&mut self, cmds: &Commands<(), ()>) {
        for cmd in &cmds.0 {
            self.compile_command(cmd);
        }
    }

    fn compile_command(&mut self, cmd: &Command<(), ()>) {
        match &cmd.kind {
            CommandKind::Assignment(t, e) => {
                self.push(Instr::Assign(t.clone(), e.clone()), Some(cmd.span));
            }
            CommandKind::Skip => {
                self.push(Instr::Nop, Some(cmd.span));
            }
            CommandKind::Placeholder => {
                self.push(Instr::Nop, Some(cmd.span));
            }
            CommandKind::If(guards) => {
                let head = self.push(Instr::Nop, Some(cmd.span));
                let mut choices = Vec::new();
                let mut exits = Vec::new();
                for guard in guards {
                    choices.push((CG::BoolExpression(guard.guard.clone()), self.current()));
                    self.compile_commands(&guard.cmds);
                    exits.push(self.current());
                    self.push(Instr::Nop, Some(cmd.span));
                }
                self.set(
                    head,
                    Instr::Branch {
                        choices,
                        otherwise: None,
                    },
                );
                for exit in exits {
                    self.set(exit, Instr::Goto(self.current()));
                }
            }
            CommandKind::IfCG(guards) => {
                let head = self.push(Instr::Nop, Some(cmd.span));
                let mut choices = Vec::new();
                let mut exits = Vec::new();
                for guard in guards {
                    choices.push((guard.guard.clone(), self.current()));
                    self.compile_commands(&guard.cmds);
                    exits.push(self.current());
                    self.push(Instr::Nop, Some(cmd.span));
                }
                self.set(
                    head,
                    Instr::Branch {
                        choices,
                        otherwise: None,
                    },
                );
                for exit in exits {
                    self.set(exit, Instr::Goto(self.current()));
                }
            }
            CommandKind::Loop(_, guards) => {
                let head = self.push(Instr::Nop, Some(cmd.span));
                let mut choices = Vec::new();
                for guard in guards {
                    choices.push((CG::BoolExpression(guard.guard.clone()), self.current()));
                    self.compile_commands(&guard.cmds);
                    self.push(Instr::Goto(head), Some(cmd.span));
                }
                self.set(
                    head,
                    Instr::Branch {
                        choices,
                        otherwise: Some(self.current()),
                    },
                );
            }
            CommandKind::LoopCG(_, guards) => {
                let head = self.push(Instr::Nop, Some(cmd.span));
                let mut choices = Vec::new();
                for guard in guards {
                    choices.push((guard.guard.clone(), self.current()));
                    self.compile_commands(&guard.cmds);
                    self.push(Instr::Goto(head), Some(cmd.span));
                }
                self.set(
                    head,
                    Instr::Branch {
                        choices,
                        otherwise: None,
                    },
                );
            }
            CommandKind::O(Operation::Put(target, args)) => {
                let index = self.tuple_space_index(target.name()).unwrap();
                let tuple_max_size = self.tuple_spaces[index as usize].size.clone();
                self.push(
                    Instr::Put(tuple_max_size, index, args.clone()),
                    Some(cmd.span),
                );
            }
            CommandKind::O(Operation::Get(target, args)) => {
                let index = self.tuple_space_index(target.name()).unwrap();
                let tuple_type = self.tuple_spaces[index as usize].space_type.clone();
                self.push(Instr::Get(tuple_type, index, args.clone()), Some(cmd.span));
            }
            CommandKind::O(Operation::Query(target, args)) => {
                let index = self.tuple_space_index(target.name()).unwrap();
                let tuple_type = self.tuple_spaces[index as usize].space_type.clone();
                self.push(
                    Instr::Query(tuple_type, index, args.clone()),
                    Some(cmd.span),
                );
            }
            CommandKind::Send(ch, e) => {
                if let Some(index) = self.channel_index(ch.name()) {
                    let size = self.channels[index as usize].size.clone();
                    self.push(Instr::Send(size, index, e.clone()), Some(cmd.span));
                } else {
                    self.push(
                        Instr::SyncSend {
                            channel: ch.name().to_string(),
                            expr: e.clone(),
                        },
                        Some(cmd.span),
                    );
                }
            }
            CommandKind::Receive(ch, target) => {
                if let Some(ch_index) = self.channel_index(ch.name()) {
                    self.push(Instr::Receive(ch_index, target.clone()), Some(cmd.span));
                } else {
                    self.push(
                        Instr::SyncReceive {
                            channel: ch.name().to_string(),
                            target: target.clone(),
                        },
                        Some(cmd.span),
                    );
                }
            }
            CommandKind::Broadcast(ch, k, e) => {
                self.push(
                    Instr::Broadcast {
                        channel: ch.name().to_string(),
                        k: k.clone(),
                        expr: e.clone(),
                    },
                    Some(cmd.span),
                );
            }
            CommandKind::Gather(ch, k, arr, x) => {
                self.push(
                    Instr::Gather {
                        channel: ch.name().to_string(),
                        k: k.clone(),
                        array: arr.clone(),
                        target: x.clone(),
                    },
                    Some(cmd.span),
                );
            }
        }
    }
}

type Memory = Vec<i32>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    ptrs: Vec<InstrPtr>,
    memory: Memory,
    tuple_spaces: Vec<Vec<Vec<Int>>>,
    channels: Vec<Vec<Int>>,
}

#[derive(Debug)]
pub enum StepError {
    DivisionByZero,
    NegativeFactorial,
    NegativeFibonacci,
    NegativePower,
    Stuck,
    Halt,
    HitOld,
    ArrayIndexNegative,
    ArrayIndexOutOfBounds,
}

impl State {
    pub fn step<'a>(&'a self, p: &'a Program) -> impl Iterator<Item = State> + 'a {
        self.step_inner(p).flatten()
    }
    fn step_inner<'a>(
        &'a self,
        p: &'a Program,
    ) -> impl Iterator<Item = Result<State, StepError>> + 'a {
        let regular_steps =
            self.ptrs
                .iter()
                .enumerate()
                .flat_map(|(i, _)| match self.step_exe(p, i) {
                    Ok(inner) => Either::Left(inner.map(Ok)),
                    Err(err) => Either::Right([Err(err)].into_iter()),
                });

        let sync_steps = match self.find_sync_pairs(p) {
            Ok(states) => states.into_iter().map(Ok).collect::<Vec<_>>(),
            Err(e) => vec![Err(e)],
        };

        regular_steps
            .chain(sync_steps)
            .map(|s| Ok(s?.follow_gotos(p)))
    }
    fn find_sync_pairs(&self, p: &Program) -> Result<Vec<State>, StepError> {
        let mut transitions = Vec::new();

        for i in 0..self.ptrs.len() {
            match &p[self.ptrs[i]] {
                Instr::Broadcast {
                    channel: c1,
                    k,
                    expr,
                } => {
                    let value = expr.evaluate(p, self)?;
                    let mut counter = 0;
                    let mut receivers = Vec::new();
                    for j in 0..self.ptrs.len() {
                        if i == j {
                            continue;
                        }
                        match &p[self.ptrs[j]] {
                            Instr::SyncReceive {
                                channel: c2,
                                target,
                            } if c1 == c2 => {
                                receivers.push((j, target.clone(), None));
                                counter += 1;
                            }
                            Instr::Branch { choices, .. } => {
                                for (cg, t) in choices {
                                    if let CG::Receive(c2, target) = cg {
                                        if c1 == c2.name() {
                                            receivers.push((j, target.clone(), Some(*t)));
                                            counter += 1;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    if *k <= counter {
                        let mut new_state = self.clone();
                        for (j, target, branch_ptr) in receivers {
                            let index = match target {
                                Target::Variable(var) => p.variable_index(&var.0).unwrap(),
                                Target::Array(arr, idx) => self.array_index(&arr, &idx, p)?,
                            };
                            new_state.memory[index as usize] = value;

                            if let Some(ptr) = branch_ptr {
                                new_state.ptrs[j] = ptr;
                            } else {
                                new_state.ptrs[j] = new_state.ptrs[j].bump();
                            }
                        }
                        new_state.ptrs[i] = self.ptrs[i].bump();
                        transitions.push(new_state);
                    }
                }
                _ => {}
            }
            for j in 0..self.ptrs.len() {
                if i == j {
                    continue;
                }
                match (&p[self.ptrs[i]], &p[self.ptrs[j]]) {
                    (
                        Instr::SyncSend { channel: c1, expr },
                        Instr::SyncReceive {
                            channel: c2,
                            target,
                        },
                    ) => {
                        if c1 == c2 {
                            if let Ok(value) = expr.evaluate(p, self) {
                                let mut new_state = self.clone();
                                let index = match target {
                                    Target::Variable(var) => p.variable_index(&var.0).unwrap(),
                                    Target::Array(arr, idx) => self.array_index(arr, idx, p)?,
                                };
                                new_state.memory[index as usize] = value;
                                new_state.ptrs[i] = new_state.ptrs[i].bump();
                                new_state.ptrs[j] = new_state.ptrs[j].bump();
                                transitions.push(new_state);
                            }
                        }
                    }
                    (Instr::Branch { choices: ci, .. }, Instr::Branch { choices: cj, .. }) => {
                        for (cgi, target_i) in ci {
                            for (cgj, target_j) in cj {
                                if let (CG::Send(c1, expr), CG::Receive(c2, t)) = (cgi, cgj) {
                                    if c1 == c2 {
                                        if let Ok(value) = expr.evaluate(p, self) {
                                            let index = match t {
                                                Target::Variable(var) => {
                                                    p.variable_index(&var.0).unwrap()
                                                }
                                                Target::Array(arr, idx) => {
                                                    self.array_index(arr, idx, p)?
                                                }
                                            };
                                            let mut new_state = self.clone();
                                            new_state.memory[index as usize] = value;
                                            new_state.ptrs[i] = *target_i;
                                            new_state.ptrs[j] = *target_j;
                                            transitions.push(new_state);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    (Instr::SyncSend { channel: c1, expr }, Instr::Branch { choices: cj, .. }) => {
                        for (cgj, target_j) in cj {
                            if let CG::Receive(c2, t) = cgj {
                                if c1 == c2.name() {
                                    if let Ok(value) = expr.evaluate(p, self) {
                                        let index = match t {
                                            Target::Variable(var) => {
                                                p.variable_index(&var.0).unwrap()
                                            }
                                            Target::Array(arr, idx) => {
                                                self.array_index(arr, idx, p)?
                                            }
                                        };
                                        let mut new_state = self.clone();
                                        new_state.memory[index as usize] = value;
                                        new_state.ptrs[i] = new_state.ptrs[i].bump();
                                        new_state.ptrs[j] = *target_j;
                                        transitions.push(new_state);
                                    }
                                }
                            }
                        }
                    }
                    (
                        Instr::Branch { choices: ci, .. },
                        Instr::SyncReceive {
                            channel: c2,
                            target,
                        },
                    ) => {
                        for (cgi, target_i) in ci {
                            if let CG::Send(c1, expr) = cgi {
                                if c1.name() == c2 {
                                    if let Ok(value) = expr.evaluate(p, self) {
                                        let mut new_state = self.clone();
                                        let index = match target {
                                            Target::Variable(var) => {
                                                p.variable_index(&var.0).unwrap()
                                            }
                                            Target::Array(arr, idx) => {
                                                self.array_index(arr, idx, p)?
                                            }
                                        };
                                        new_state.memory[index as usize] = value;
                                        new_state.ptrs[i] = *target_i;
                                        new_state.ptrs[j] = new_state.ptrs[j].bump();
                                        transitions.push(new_state);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(transitions)
    }

    fn follow_gotos(mut self, p: &Program) -> State {
        for ptr in self.ptrs.iter_mut() {
            loop {
                match &p[*ptr] {
                    Instr::Goto(q) => *ptr = *q,
                    Instr::Nop => *ptr = ptr.bump(),
                    _ => break,
                }
            }
        }
        self
    }
    pub fn spans<'a>(&'a self, p: &'a Program) -> impl Iterator<Item = SourceSpan> + 'a {
        self.ptrs
            .iter()
            .filter_map(|ptr| p.source_map[ptr.0 as usize])
    }
    pub fn variables<'a>(&'a self, p: &'a Program) -> impl Iterator<Item = (&'a Variable, i32)> {
        p.variables().zip(self.memory.iter().copied())
    }
    fn step_exe<'a>(
        &'a self,
        p: &'a Program,
        execution: usize,
    ) -> Result<impl Iterator<Item = State> + 'a, StepError> {
        Ok(self
            .step_at(p, self.ptrs[execution])?
            .map(move |(mem, tuple_spaces, channels, ptr)| {
                let mut ptrs = self.ptrs.clone();
                ptrs[execution] = ptr;
                State {
                    ptrs,
                    memory: mem,
                    tuple_spaces,
                    channels,
                }
            }))
    }
    pub fn is_terminated(&self, p: &Program) -> bool {
        self.step_inner(p)
            .all(|t| matches!(t, Err(StepError::Halt)))
    }
    pub fn is_stuck(&self, p: &Program) -> bool {
        let all_stuck_or_halted = self
            .step_inner(p)
            .all(|t| matches!(t, Err(StepError::Stuck | StepError::Halt)));
        let any_stuck = self
            .step_inner(p)
            .any(|t| matches!(t, Err(StepError::Stuck)));

        all_stuck_or_halted && any_stuck
    }

    fn array_index(&self, arr: &Array, idx: &Box<AExpr>, p: &Program) -> Result<u32, StepError> {
        let (base_index, length) = p.array_meta(arr).unwrap();
        let idx = idx.evaluate(p, self)?;

        if idx < 0 {
            return Err(StepError::ArrayIndexNegative);
        }

        let idx_u32 = idx as u32;

        if idx_u32 >= length {
            return Err(StepError::ArrayIndexOutOfBounds);
        }

        Ok(base_index + idx_u32)
    }

    fn matches(&self, t: &Vec<i32>, fields: &Vec<Field>, p: &Program) -> Result<bool, StepError> {
        if t.len() != fields.len() {
            return Ok(false);
        }

        for (v, f) in t.iter().zip(fields.iter()) {
            match f {
                Field::Expression(expr) => {
                    let expected = expr.evaluate(p, self)?;
                    if *v != expected {
                        return Ok(false);
                    }
                }
                _ => {}
            }
        }

        return Ok(true);
    }
    fn update_mem(
        &self,
        pos: usize,
        fields: &Vec<Field>,
        memory: Vec<i32>,
        tuple_spaces: Vec<Vec<Vec<i32>>>,
        ts_index: &u32,
        p: &Program,
        remove: bool,
    ) -> Result<(Memory, Vec<Vec<Vec<i32>>>), StepError> {
        let mut mem_copy = memory.clone();
        let mut ts_copy = tuple_spaces.clone();

        let tuple = if remove {
            ts_copy[*ts_index as usize].remove(pos)
        } else {
            ts_copy[*ts_index as usize][pos].clone()
        };
        for (v, f) in tuple.iter().zip(fields.iter()) {
            if let Field::Variable(t) = f {
                let index = match t {
                    Target::Variable(var) => p.variable_index(&var.0).unwrap(),
                    Target::Array(arr, idx) => self.array_index(arr, idx, p)?,
                };

                mem_copy[index as usize] = *v;
            }
        }

        Ok((mem_copy, ts_copy))
    }

    fn match_tuple_space_type(
        &self,
        ts_type: &TupleSpaceType,
        ts_index: &u32,
        fields: &Vec<Field>,
        p: &Program,
        ptr: &InstrPtr,
        remove: bool,
    ) -> Result<
        Either<
            std::array::IntoIter<(Memory, Vec<Vec<Vec<Int>>>, Vec<Vec<Int>>, InstrPtr), 1>,
            std::vec::IntoIter<(Memory, Vec<Vec<Vec<Int>>>, Vec<Vec<Int>>, InstrPtr)>,
        >,
        StepError,
    > {
        let tuple_spaces = self.tuple_spaces.clone();
        let memory = self.memory.clone();

        match ts_type {
            TupleSpaceType::Random => {
                let mut results = Vec::new();

                for (pos, t) in tuple_spaces[*ts_index as usize].iter().enumerate() {
                    if self.matches(t, fields, p)? {
                        let (mem_copy, ts_copy) = self.update_mem(
                            pos,
                            fields,
                            memory.clone(),
                            tuple_spaces.clone(),
                            ts_index,
                            p,
                            remove,
                        )?;

                        results.push((mem_copy, ts_copy, self.channels.clone(), ptr.bump()));
                    }
                }

                if results.is_empty() {
                    return Err(StepError::Stuck);
                }

                Ok(Either::Right(results.into_iter()))
            }
            TupleSpaceType::FIFO => {
                let mut found_pos = None;

                for (pos, t) in tuple_spaces[*ts_index as usize].iter().enumerate() {
                    if self.matches(t, fields, p)? {
                        found_pos = Some(pos);
                        break;
                    }
                }

                if let Some(pos) = found_pos {
                    let (new_mem, new_ts) = self.update_mem(
                        pos,
                        fields,
                        memory.clone(),
                        tuple_spaces.clone(),
                        ts_index,
                        p,
                        remove,
                    )?;
                    Ok(Either::Left(
                        [(new_mem, new_ts, self.channels.clone(), ptr.bump())].into_iter(),
                    ))
                } else {
                    return Err(StepError::Stuck);
                }
            }
            TupleSpaceType::LIFO => {
                let mut found_pos = None;

                for (pos, t) in tuple_spaces[*ts_index as usize].iter().enumerate().rev() {
                    if self.matches(t, fields, p)? {
                        found_pos = Some(pos);
                        break;
                    }
                }

                if let Some(pos) = found_pos {
                    let (new_mem, new_ts) = self.update_mem(
                        pos,
                        fields,
                        memory.clone(),
                        tuple_spaces.clone(),
                        ts_index,
                        p,
                        remove,
                    )?;
                    Ok(Either::Left(
                        [(new_mem, new_ts, self.channels.clone(), ptr.bump())].into_iter(),
                    ))
                } else {
                    return Err(StepError::Stuck);
                }
            }
            TupleSpaceType::Queue => {
                if let Some(t) = tuple_spaces[*ts_index as usize].first() {
                    if self.matches(t, fields, p)? {
                        let (new_mem, new_ts) = self.update_mem(
                            0,
                            fields,
                            memory.clone(),
                            tuple_spaces.clone(),
                            ts_index,
                            p,
                            remove,
                        )?;
                        return Ok(Either::Left(
                            [(new_mem, new_ts, self.channels.clone(), ptr.bump())].into_iter(),
                        ));
                    }
                }
                Err(StepError::Stuck)
            }
            TupleSpaceType::Stack => {
                if let Some(t) = tuple_spaces[*ts_index as usize].last() {
                    let pos = tuple_spaces[*ts_index as usize].len() - 1;
                    if self.matches(t, fields, p)? {
                        let (new_mem, new_ts) = self.update_mem(
                            pos,
                            fields,
                            memory.clone(),
                            tuple_spaces.clone(),
                            ts_index,
                            p,
                            remove,
                        )?;
                        return Ok(Either::Left(
                            [(new_mem, new_ts, self.channels.clone(), ptr.bump())].into_iter(),
                        ));
                    }
                }
                Err(StepError::Stuck)
            }
        }
    }

    fn eval_bool_guard(
        &self,
        p: &Program,
        expr: &BExpr,
    ) -> Vec<(Memory, Vec<Vec<Vec<Int>>>, Vec<Vec<Int>>)> {
        match expr {
            BExpr::OP(Operation::Get(t, f)) => {
                let ts_index = p.tuple_space_index(t.name()).unwrap();
                let ts_type = p.tuple_spaces[ts_index as usize].space_type.clone();
                self.match_tuple_space_type(&ts_type, &ts_index, f, p, &InstrPtr(0), true)
                    .map(|either| {
                        either
                            .into_iter()
                            .map(|(m, ts, _, _)| (m, ts, self.channels.clone()))
                            .collect()
                    })
                    .unwrap_or_default()
            }
            BExpr::OP(Operation::Query(t, f)) => {
                let ts_index = p.tuple_space_index(t.name()).unwrap();
                let ts_type = p.tuple_spaces[ts_index as usize].space_type.clone();
                self.match_tuple_space_type(&ts_type, &ts_index, f, p, &InstrPtr(0), false)
                    .map(|either| {
                        either
                            .into_iter()
                            .map(|(m, ts, _, _)| (m, ts, self.channels.clone()))
                            .collect()
                    })
                    .unwrap_or_default()
            }
            BExpr::OP(Operation::Put(t, args)) => {
                let ts_index = p.tuple_space_index(t.name()).unwrap();
                let ts_meta = &p.tuple_spaces[ts_index as usize];
                if let BufferSize::Finite(max) = ts_meta.size {
                    if self.tuple_spaces[ts_index as usize].len() >= max as usize {
                        return vec![];
                    }
                }
                let mut ts = self.tuple_spaces.clone();
                let values = args
                    .iter()
                    .map(|e| e.evaluate(p, self).unwrap_or(0))
                    .collect();
                ts[ts_index as usize].push(values);
                vec![(self.memory.clone(), ts, self.channels.clone())]
            }
            BExpr::Logic(l, LogicOp::And | LogicOp::Land, r) => self
                .eval_bool_guard(p, l)
                .into_iter()
                .flat_map(|(m, ts, _)| {
                    State {
                        ptrs: self.ptrs.clone(),
                        memory: m,
                        tuple_spaces: ts,
                        channels: self.channels.clone(),
                    }
                    .eval_bool_guard(p, r)
                })
                .collect(),
            BExpr::Logic(l, LogicOp::Or | LogicOp::Lor, r) => {
                let left_res = self.eval_bool_guard(p, l);

                if left_res.is_empty() {
                    self.eval_bool_guard(p, r)
                } else {
                    let chained: Vec<_> = left_res
                        .iter()
                        .flat_map(|(m, ts, _)| {
                            State {
                                ptrs: self.ptrs.clone(),
                                memory: m.clone(),
                                tuple_spaces: ts.clone(),
                                channels: self.channels.clone(),
                            }
                            .eval_bool_guard(p, r)
                        })
                        .collect();

                    if !chained.is_empty() {
                        chained
                    } else {
                        left_res
                    }
                }
            }
            _ => {
                if expr.evaluate(p, self).unwrap_or(false) {
                    vec![(
                        self.memory.clone(),
                        self.tuple_spaces.clone(),
                        self.channels.clone(),
                    )]
                } else {
                    vec![]
                }
            }
        }
    }

    fn eval_guard(
        &self,
        p: &Program,
        cg: &CG,
    ) -> Result<Vec<(Memory, Vec<Vec<Vec<Int>>>, Vec<Vec<Int>>)>, StepError> {
        match cg {
            CG::BoolExpression(expr) => Ok(self.eval_bool_guard(p, expr)),
            CG::Send(ch, expr) => {
                if p.channel_index(ch.name()).is_none() {
                    return Ok(vec![]);
                }
                if let Ok(value) = expr.evaluate(p, self) {
                    let mut channels = self.channels.clone();
                    let ch_index = p.channel_index(ch.name()).unwrap();
                    let buffer_size = p.channels[ch_index as usize].size.clone();

                    match buffer_size {
                        BufferSize::Finite(max_size) => {
                            if channels[ch_index as usize].len() < max_size as usize {
                                channels[ch_index as usize].push(value);
                                return Ok(vec![(
                                    self.memory.clone(),
                                    self.tuple_spaces.clone(),
                                    channels,
                                )]);
                            } else {
                                return Ok(vec![]);
                            }
                        }
                        BufferSize::Infinite => {
                            channels[ch_index as usize].push(value);
                            return Ok(vec![(
                                self.memory.clone(),
                                self.tuple_spaces.clone(),
                                channels,
                            )]);
                        }
                    }
                };
                Ok(vec![])
            }
            CG::Receive(ch, target) => {
                if p.channel_index(ch.name()).is_none() {
                    return Ok(vec![]);
                }
                let mut channels = self.channels.clone();
                let mut memory = self.memory.clone();
                let ch_index = p.channel_index(ch.name()).unwrap();
                let index = match target {
                    Target::Variable(var) => p.variable_index(&var.0).unwrap(),
                    Target::Array(arr, idx) => self.array_index(arr, idx, p)?,
                };

                if let Some(v) = channels[ch_index as usize].first() {
                    memory[index as usize] = *v;
                    channels[ch_index as usize].remove(0);

                    return Ok(vec![(memory, self.tuple_spaces.clone(), channels)]);
                }
                Ok(vec![])
            }
        }
    }

    fn step_at(
        &self,
        p: &Program,
        ptr: InstrPtr,
    ) -> Result<
        impl Iterator<Item = (Memory, Vec<Vec<Vec<Int>>>, Vec<Vec<Int>>, InstrPtr)>,
        StepError,
    > {
        match &p[ptr] {
            Instr::Nop => Ok(Either::Left(
                [(
                    self.memory.clone(),
                    self.tuple_spaces.clone(),
                    self.channels.clone(),
                    ptr.bump(),
                )]
                .into_iter(),
            )),
            Instr::Assign(target, e) => {
                let value = e.evaluate(p, self)?;
                let mut memory = self.memory.clone();

                let index = match target {
                    Target::Variable(var) => p.variable_index(&var.0).unwrap(),
                    Target::Array(arr, idx) => self.array_index(arr, idx, p)?,
                };

                memory[index as usize] = value;

                Ok(Either::Left(
                    [(
                        memory,
                        self.tuple_spaces.clone(),
                        self.channels.clone(),
                        ptr.bump(),
                    )]
                    .into_iter(),
                ))
            }
            Instr::Branch { choices, otherwise } => {
                let mut valid = Vec::new();
                for (cg, target) in choices {
                    for (mem, ts, ch) in self.eval_guard(p, cg)? {
                        valid.push((mem, ts, ch, *target));
                    }
                }
                if valid.is_empty() {
                    if let Some(target) = otherwise {
                        Ok(Either::Left(
                            [(
                                self.memory.clone(),
                                self.tuple_spaces.clone(),
                                self.channels.clone(),
                                *target,
                            )]
                            .into_iter(),
                        ))
                    } else {
                        Err(StepError::Stuck)
                    }
                } else {
                    Ok(Either::Right(valid.into_iter()))
                }
            }
            Instr::Goto(target) => Ok(Either::Left(
                [(
                    self.memory.clone(),
                    self.tuple_spaces.clone(),
                    self.channels.clone(),
                    *target,
                )]
                .into_iter(),
            )),
            Instr::Halt => Err(StepError::Halt),
            Instr::Put(ts_max_size, ts_index, args) => {
                let values: Vec<Int> = args
                    .iter()
                    .map(|e| e.evaluate(p, self).map(Int::from))
                    .collect::<Result<_, _>>()?;

                let mut tuple_spaces = self.tuple_spaces.clone();

                match ts_max_size {
                    BufferSize::Finite(max_size) => {
                        if tuple_spaces[*ts_index as usize].len() < *max_size as usize {
                            tuple_spaces[*ts_index as usize].push(values);
                        } else {
                            return Err(StepError::Stuck);
                        }
                    }
                    BufferSize::Infinite => {
                        tuple_spaces[*ts_index as usize].push(values);
                    }
                }

                Ok(Either::Left(
                    [(
                        self.memory.clone(),
                        tuple_spaces,
                        self.channels.clone(),
                        ptr.bump(),
                    )]
                    .into_iter(),
                ))
            }
            Instr::Get(ts_type, ts_index, fields) => {
                self.match_tuple_space_type(ts_type, ts_index, fields, p, &ptr, true)
            }
            Instr::Query(ts_type, ts_index, fields) => {
                self.match_tuple_space_type(ts_type, ts_index, fields, p, &ptr, false)
            }
            Instr::Send(buffer_size, ch_index, e) => {
                let value = e.evaluate(p, self)?;
                let mut channels = self.channels.clone();

                match buffer_size {
                    BufferSize::Finite(max_size) => {
                        if channels[*ch_index as usize].len() < *max_size as usize {
                            channels[*ch_index as usize].push(value);
                        } else {
                            return Err(StepError::Stuck);
                        }
                    }
                    BufferSize::Infinite => {
                        channels[*ch_index as usize].push(value);
                    }
                }

                Ok(Either::Left(
                    [(
                        self.memory.clone(),
                        self.tuple_spaces.clone(),
                        channels,
                        ptr.bump(),
                    )]
                    .into_iter(),
                ))
            }
            Instr::Receive(ch_index, target) => {
                let mut channels = self.channels.clone();
                let mut memory = self.memory.clone();

                let index = match target {
                    Target::Variable(var) => p.variable_index(&var.0).unwrap(),
                    Target::Array(arr, idx) => self.array_index(arr, idx, p)?,
                };

                if let Some(v) = channels[*ch_index as usize].first() {
                    memory[index as usize] = *v;
                    channels[*ch_index as usize].remove(0);

                    return Ok(Either::Left(
                        [(memory, self.tuple_spaces.clone(), channels, ptr.bump())].into_iter(),
                    ));
                }
                Err(StepError::Stuck)
            }
            Instr::SyncSend { .. } => Err(StepError::Stuck),
            Instr::SyncReceive { .. } => Err(StepError::Stuck),
            Instr::Broadcast { .. } => Err(StepError::Stuck),
            Instr::Gather { .. } => Err(StepError::Stuck),
        }
    }

    pub fn raw_id(&self) -> String {
        let vars = self.memory.iter().format(" ");
        format!("{}@{}", vars, self.ptrs.iter().map(|p| p.0).format("X"))
    }
    pub fn format<'a>(&'a self, p: &'a Program) -> StateFormat<'a> {
        StateFormat {
            state: self,
            program: p,
        }
    }
}

pub struct StateFormat<'a> {
    state: &'a State,
    program: &'a Program,
}

impl fmt::Display for StateFormat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut parts = Vec::new();

        let var_count = self.program.variables.len();
        self.state
            .memory
            .iter()
            .take(var_count)
            .zip(&self.program.variables)
            .for_each(|(value, var)| parts.push(format!("{var} = {value}")));

        for ArrayMeta {
            name,
            base_index,
            length,
        } in &self.program.arrays
        {
            let array_values: Vec<String> = self
                .state
                .memory
                .iter()
                .skip(*base_index as usize)
                .take(*length as usize)
                .map(|v| v.to_string())
                .collect();
            parts.push(format!("{name} = [{}]", array_values.join(", ")));
        }

        for (ts_meta, ts_values) in self
            .program
            .tuple_spaces
            .iter()
            .zip(&self.state.tuple_spaces)
        {
            let tuples_str = ts_values
                .iter()
                .map(|tuple| {
                    format!(
                        "({})",
                        tuple
                            .iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("{} = {{{}}}", ts_meta.name, tuples_str))
        }

        for (ch_meta, ch_values) in self.program.channels.iter().zip(&self.state.channels) {
            parts.push(format!(
                "{} = ({})",
                ch_meta.name,
                ch_values
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }

        write!(f, "{}", parts.join(", "))
    }
}

impl AExpr {
    fn evaluate(&self, p: &Program, state: &State) -> Result<i32, StepError> {
        Ok(match self {
            AExpr::Number(n) => *n,
            AExpr::Reference(r) => {
                let index = match r {
                    Target::Variable(var) => p.variable_index(&var.0).unwrap(),
                    Target::Array(arr, idx) => state.array_index(arr, idx, p)?,
                };
                state.memory[index as usize]
            }
            AExpr::Binary(l, op, r) => {
                let l = l.evaluate(p, state)?;
                let r = r.evaluate(p, state)?;
                match op {
                    AOp::Plus => l + r,
                    AOp::Minus => l - r,
                    AOp::Times => l * r,
                    AOp::Divide => l / r,
                }
            }
            AExpr::Minus(e) => -e.evaluate(p, state)?,
            AExpr::Function(f) => match f {
                Function::Division(a, b) => {
                    let a = a.evaluate(p, state)?;
                    let b = b.evaluate(p, state)?;
                    if b == 0 {
                        return Err(StepError::DivisionByZero);
                    }
                    a / b
                }
                Function::Min(a, b) => {
                    let a = a.evaluate(p, state)?;
                    let b = b.evaluate(p, state)?;
                    a.min(b)
                }
                Function::Max(a, b) => {
                    let a = a.evaluate(p, state)?;
                    let b = b.evaluate(p, state)?;
                    a.max(b)
                }
                Function::Fac(x) => {
                    let x = x.evaluate(p, state)?;
                    if x < 0 {
                        return Err(StepError::NegativeFactorial);
                    }
                    (1..=x).product()
                }
                Function::Fib(x) => {
                    let x = x.evaluate(p, state)?;
                    if x < 0 {
                        return Err(StepError::NegativeFibonacci);
                    }
                    let mut a = 0;
                    let mut b = 1;
                    for _ in 0..x {
                        let c = a + b;
                        a = b;
                        b = c;
                    }
                    a
                }
                Function::Exp(a, b) => {
                    let a = a.evaluate(p, state)?;
                    let b = b.evaluate(p, state)?;
                    if b < 0 {
                        return Err(StepError::NegativePower);
                    }
                    a.pow(b as u32)
                }
            },
            AExpr::Old(_) => return Err(StepError::HitOld),
        })
    }
}

impl BExpr {
    pub fn evaluate(&self, p: &Program, state: &State) -> Result<bool, StepError> {
        Ok(match self {
            BExpr::Bool(b) => *b,
            BExpr::Rel(l, op, r) => {
                let l = l.evaluate(p, state)?;
                let r = r.evaluate(p, state)?;
                match op {
                    RelOp::Eq => l == r,
                    RelOp::Ne => l != r,
                    RelOp::Lt => l < r,
                    RelOp::Le => l <= r,
                    RelOp::Gt => l > r,
                    RelOp::Ge => l >= r,
                }
            }
            BExpr::Logic(l, op, r) => {
                let l = l.evaluate(p, state)?;
                let r = r.evaluate(p, state)?;
                match op {
                    LogicOp::And => l && r,
                    LogicOp::Land => l && r,
                    LogicOp::Or => l || r,
                    LogicOp::Lor => l || r,
                    LogicOp::Implies => !l || r,
                }
            }
            BExpr::Not(e) => !e.evaluate(p, state)?,
            BExpr::Quantified(_, _, _) => todo!(),
            BExpr::OP(_) => !state.eval_bool_guard(p, self).is_empty(),
        })
    }
}

impl ChannelFormula {
    pub fn evaluate(&self, p: &Program, state: &State) -> Result<bool, StepError> {
        Ok(match self {
            ChannelFormula::ChannelHead(c, e) => {
                let e = e.evaluate(p, state)?;
                let index = p.channel_index(c.name()).unwrap();
                let v = state.channels[index as usize].first().unwrap();
                e == *v
            }
            ChannelFormula::ChannelContains(c, e) => {
                let e = e.evaluate(p, state)?;
                let index = p.channel_index(c.name()).unwrap();
                state.channels[index as usize].contains(&e)
            }
        })
    }
}

impl LTLFormula {
    pub fn to_mcltl(
        &self,
        rels: &mut Vec<(AExpr, RelOp, AExpr)>,
        operations: &mut Vec<Operation>,
        channels: &mut Vec<ChannelFormula>,
    ) -> mcltl::ltl::expression::LTLExpression {
        use mcltl::ltl::expression::LTLExpression;

        match self {
            LTLFormula::Bool(true) => LTLExpression::True,
            LTLFormula::Bool(false) => LTLExpression::False,
            LTLFormula::Locator(l) => LTLExpression::lit(l.to_lit()),
            LTLFormula::Rel(lhs, op, rhs) => {
                let idx = if let Some(idx) = rels
                    .iter()
                    .position(|(l, o, r)| l == lhs && o == op && r == rhs)
                {
                    idx
                } else {
                    rels.push((lhs.clone(), *op, rhs.clone()));
                    rels.len() - 1
                };
                LTLExpression::Literal(format!("p{idx}").into())
            }
            LTLFormula::Operation(op) => {
                let idx = if let Some(idx) = operations.iter().position(|x| x == op.as_ref()) {
                    idx
                } else {
                    operations.push(op.as_ref().clone());
                    operations.len() - 1
                };
                LTLExpression::Literal(format!("o{idx}").into())
            }
            LTLFormula::ChannelFormula(cf) => {
                let idx = if let Some(idx) = channels.iter().position(|x| x == cf.as_ref()) {
                    idx
                } else {
                    channels.push(cf.as_ref().clone());
                    channels.len() - 1
                };
                LTLExpression::Literal(format!("c{idx}").into())
            }
            LTLFormula::Not(e) => !e.to_mcltl(rels, operations, channels),
            LTLFormula::And(p, q) => {
                p.to_mcltl(rels, operations, channels) & q.to_mcltl(rels, operations, channels)
            }
            LTLFormula::Or(p, q) => {
                p.to_mcltl(rels, operations, channels) | q.to_mcltl(rels, operations, channels)
            }
            LTLFormula::Implies(p, q) => {
                !p.to_mcltl(rels, operations, channels) | q.to_mcltl(rels, operations, channels)
            }
            LTLFormula::Until(p, q) => p
                .to_mcltl(rels, operations, channels)
                .U(q.to_mcltl(rels, operations, channels)),
            LTLFormula::Next(p) => {
                LTLExpression::X(Box::new(p.to_mcltl(rels, operations, channels)))
            }
            LTLFormula::Globally(p) => {
                LTLExpression::G(Box::new(p.to_mcltl(rels, operations, channels)))
            }
            LTLFormula::Finally(p) => {
                LTLExpression::F(Box::new(p.to_mcltl(rels, operations, channels)))
            }
        }
    }
}

impl Locator {
    pub fn to_lit(&self) -> Literal {
        format!("@{self}").into()
    }
}
