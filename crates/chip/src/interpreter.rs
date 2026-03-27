use std::fmt;

use indexmap::IndexMap;
use itertools::{Either, Itertools};
use mcltl::ltl::expression::Literal;

use crate::{
    ast::{
        AExpr, AOp, Array, BExpr, BufferSize, CG, Channel, ChannelFormula, ChannelName, Command,
        CommandKind, Commands, Field, Function, Int, LTLFormula, Locator, LogicOp, Operation,
        OperationP, RelOp, Target, TupleSpace, TupleSpaceName, TupleSpaceType, Variable,
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
        channel: ChannelName,
        expr: AExpr,
    },
    SyncReceive {
        channel: ChannelName,
        target: Target<Box<AExpr>>,
    },
    Broadcast {
        channel: ChannelName,
        k: Int,
        expr: AExpr,
    },
    Gather {
        channel: ChannelName,
        k: Int,
        array: Array,
        target: Target<Box<AExpr>>,
    },
}

#[derive(Debug)]
pub struct Program {
    targets: Vec<TargetMeta>,
    tuple_spaces: Vec<TupleSpaceMeta>,
    channels: Vec<ChannelMeta>,
    instrs: Vec<Instr>,
    entry_points: Vec<InstrPtr>,
    source_map: Vec<Option<SourceSpan>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetMeta {
    pub target: Target,
    pub base_index: u32,
    pub length: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleSpaceMeta {
    pub name: TupleSpaceName,
    pub space_type: TupleSpaceType,
    pub size: BufferSize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelMeta {
    pub name: ChannelName,
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
        additional_targets: impl IntoIterator<Item = Target>,
        tuple_spaces: IndexMap<TupleSpaceName, TupleSpace>,
        channels: IndexMap<ChannelName, Channel>,
        array_sizes: IndexMap<Array, u32>,
    ) -> Program {
        let targets: Vec<Target> = cmdss
            .iter()
            .flat_map(|cmds| cmds.fv())
            .chain(additional_targets)
            .sorted()
            .dedup()
            .collect();

        let mut metas = Vec::new();
        let mut current_index = 0u32;

        for target in targets {
            match target {
                Target::Variable(v) => {
                    metas.push(TargetMeta {
                        target: Target::Variable(v),
                        base_index: current_index,
                        length: 1,
                    });
                    current_index += 1;
                }

                Target::Array(arr, _) => {
                    let length = array_sizes.get(&arr).copied().unwrap_or(1);
                    metas.push(TargetMeta {
                        target: Target::Array(arr, ()),
                        base_index: current_index,
                        length,
                    });
                    current_index += length;
                }
            }
        }

        let mut p = Program {
            targets: metas,
            instrs: Vec::new(),
            entry_points: Vec::new(),
            source_map: Vec::new(),
            tuple_spaces: tuple_spaces
                .into_iter()
                .sorted()
                .map(|(var, ts)| TupleSpaceMeta {
                    name: var,
                    space_type: ts.space_type,
                    size: ts.size,
                })
                .collect(),
            channels: channels
                .into_iter()
                .sorted()
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
        let mut data = Vec::new();

        for meta in &self.targets {
            match &meta.target {
                Target::Variable(v) => {
                    data.push(var_init(v));
                }

                Target::Array(arr, _) => {
                    let mut values = arr_init(arr);
                    values.resize(meta.length as usize, 0);
                    data.extend_from_slice(&values);
                }
            }
        }

        State {
            ptrs: self.entry_points.clone(),
            memory: Memory {
                data,
                tuple_spaces: tuple_space_memory,
                channels: channel_memory,
            },
        }
    }

    pub fn variables(&self) -> impl Iterator<Item = &Variable> + '_ {
        self.targets.iter().filter_map(|meta| match &meta.target {
            Target::Variable(v) => Some(v),
            _ => None,
        })
    }

    pub fn arrays(&self) -> impl Iterator<Item = (&Array, u32, u32)> + '_ {
        self.targets.iter().filter_map(|meta| match &meta.target {
            Target::Array(arr, _) => Some((arr, meta.base_index, meta.length)),
            _ => None,
        })
    }

    fn variable_index(&self, name: &Variable) -> Option<u32> {
        self.targets
            .iter()
            .position(|meta| match &meta.target {
                Target::Variable(v) => v == name,
                _ => false,
            })
            .map(|idx| idx as u32)
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
            CommandKind::O(Operation::Put(ts, args)) => {
                let index = self.tuple_space_index(&ts.0).unwrap();
                let tuple_max_size = self.tuple_spaces[index as usize].size.clone();
                self.push(
                    Instr::Put(tuple_max_size, index, args.clone()),
                    Some(cmd.span),
                );
            }
            CommandKind::O(Operation::Get(ts, args)) => {
                let index = self.tuple_space_index(&ts.0).unwrap();
                let tuple_type = self.tuple_spaces[index as usize].space_type.clone();
                self.push(Instr::Get(tuple_type, index, args.clone()), Some(cmd.span));
            }
            CommandKind::O(Operation::Query(ts, args)) => {
                let index = self.tuple_space_index(&ts.0).unwrap();
                let tuple_type = self.tuple_spaces[index as usize].space_type.clone();
                self.push(
                    Instr::Query(tuple_type, index, args.clone()),
                    Some(cmd.span),
                );
            }
            CommandKind::Send(ch, e) => {
                if let Some(index) = self.channel_index(&ch.0) {
                    let size = self.channels[index as usize].size.clone();
                    self.push(Instr::Send(size, index, e.clone()), Some(cmd.span));
                } else {
                    self.push(
                        Instr::SyncSend {
                            channel: ch.clone(),
                            expr: e.clone(),
                        },
                        Some(cmd.span),
                    );
                }
            }
            CommandKind::Receive(ch, target) => {
                if let Some(ch_index) = self.channel_index(&ch.0) {
                    self.push(Instr::Receive(ch_index, target.clone()), Some(cmd.span));
                } else {
                    self.push(
                        Instr::SyncReceive {
                            channel: ch.clone(),
                            target: target.clone(),
                        },
                        Some(cmd.span),
                    );
                }
            }
            CommandKind::Broadcast(ch, k, e) => {
                self.push(
                    Instr::Broadcast {
                        channel: ch.clone(),
                        k: k.clone(),
                        expr: e.clone(),
                    },
                    Some(cmd.span),
                );
            }
            CommandKind::Gather(ch, k, arr, x) => {
                self.push(
                    Instr::Gather {
                        channel: ch.clone(),
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Memory {
    data: Vec<i32>,
    tuple_spaces: Vec<Vec<Vec<Int>>>,
    channels: Vec<Vec<Int>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    ptrs: Vec<InstrPtr>,
    memory: Memory,
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
    SameArrayGather,
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
                            }
                            Instr::Branch { choices, .. } => {
                                for (cg, t) in choices {
                                    if let CG::Receive(c2, target) = cg {
                                        if c1 == c2 {
                                            receivers.push((j, target.clone(), Some(*t)));
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    if *k as usize <= receivers.len() {
                        let mut new_state = self.clone();
                        for (j, target, branch_ptr) in receivers {
                            let index = match target {
                                Target::Variable(var) => p.variable_index(&var).unwrap(),
                                Target::Array(arr, idx) => self.array_index(&arr, &idx, p)?,
                            };
                            new_state.memory.data[index as usize] = value;

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
                Instr::Gather {
                    channel: c1,
                    k,
                    array,
                    target,
                } => {
                    if let Target::Array(arr, _) = target
                        && arr == array
                    {
                        return Err(StepError::SameArrayGather);
                    }

                    let (array_base, array_len) = p
                        .arrays()
                        .find(|(a, _, _)| a == &array)
                        .map(|(_, base, len)| (base, len))
                        .unwrap();

                    let mut senders = Vec::new();

                    for j in 0..self.ptrs.len() {
                        if i == j {
                            continue;
                        }
                        match &p[self.ptrs[j]] {
                            Instr::SyncSend { channel: c2, expr } if c1 == c2 => {
                                let value = expr.evaluate(p, self)?;
                                senders.push((j, value, None));
                            }
                            Instr::Branch { choices, .. } => {
                                for (cg, t) in choices {
                                    if let CG::Send(c2, expr) = cg {
                                        if c1 == c2 {
                                            let value = expr.evaluate(p, self)?;
                                            senders.push((j, value, Some(*t)));
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    if senders.len() >= *k as usize {
                        for perm in senders.iter().permutations(senders.len()) {
                            let mut new_state = self.clone();

                            for (idx, (j, value, branch_ptr)) in
                                perm.clone().into_iter().enumerate()
                            {
                                if idx < array_len as usize {
                                    new_state.memory.data[array_base as usize + idx] = *value;
                                }

                                if let Some(ptr) = branch_ptr {
                                    new_state.ptrs[*j] = *ptr;
                                } else {
                                    new_state.ptrs[*j] = new_state.ptrs[*j].bump();
                                }
                            }

                            let index = match target {
                                Target::Variable(var) => p.variable_index(&var).unwrap(),
                                Target::Array(arr, idx) => self.array_index(&arr, &idx, p)?,
                            };

                            new_state.memory.data[index as usize] =
                                perm.len().min(array_len as usize) as i32;

                            new_state.ptrs[i] = self.ptrs[i].bump();

                            transitions.push(new_state);
                        }
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
                                let index = match target {
                                    Target::Variable(var) => p.variable_index(&var).unwrap(),
                                    Target::Array(arr, idx) => self.array_index(arr, idx, p)?,
                                };
                                let mut new_state = self.clone();
                                new_state.memory.data[index as usize] = value;
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
                                                    p.variable_index(&var).unwrap()
                                                }
                                                Target::Array(arr, idx) => {
                                                    self.array_index(arr, idx, p)?
                                                }
                                            };
                                            let mut new_state = self.clone();
                                            new_state.memory.data[index as usize] = value;
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
                                if c1 == c2 {
                                    if let Ok(value) = expr.evaluate(p, self) {
                                        let index = match t {
                                            Target::Variable(var) => {
                                                p.variable_index(&var).unwrap()
                                            }
                                            Target::Array(arr, idx) => {
                                                self.array_index(arr, idx, p)?
                                            }
                                        };
                                        let mut new_state = self.clone();
                                        new_state.memory.data[index as usize] = value;
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
                                if c1 == c2 {
                                    if let Ok(value) = expr.evaluate(p, self) {
                                        let index = match target {
                                            Target::Variable(var) => {
                                                p.variable_index(&var).unwrap()
                                            }
                                            Target::Array(arr, idx) => {
                                                self.array_index(arr, idx, p)?
                                            }
                                        };
                                        let mut new_state = self.clone();
                                        new_state.memory.data[index as usize] = value;
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
        p.variables().zip(self.memory.data.iter().copied())
    }
    fn step_exe<'a>(
        &'a self,
        p: &'a Program,
        execution: usize,
    ) -> Result<impl Iterator<Item = State> + 'a, StepError> {
        Ok(self
            .step_at(p, self.ptrs[execution])?
            .map(move |(mem, ptr)| {
                let mut ptrs = self.ptrs.clone();
                ptrs[execution] = ptr;
                State { ptrs, memory: mem }
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

    fn array_index(
        &self,
        arr: &Array,
        idx_expr: &Box<AExpr>,
        p: &Program,
    ) -> Result<u32, StepError> {
        let idx = idx_expr.evaluate(p, self)?;
        if idx < 0 {
            return Err(StepError::ArrayIndexNegative);
        }

        let (base_index, length) = p
            .arrays()
            .find(|(a, _, _)| *a == arr)
            .map(|(_, base, len)| (base, len))
            .unwrap();

        if idx as u32 >= length {
            return Err(StepError::ArrayIndexOutOfBounds);
        }

        Ok(base_index + idx as u32)
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
        data: Vec<i32>,
        tuple_spaces: Vec<Vec<Vec<i32>>>,
        ts_index: &u32,
        p: &Program,
        remove: bool,
    ) -> Result<(Vec<i32>, Vec<Vec<Vec<i32>>>), StepError> {
        let mut data_copy = data.clone();
        let mut ts_copy = tuple_spaces.clone();

        let tuple = if remove {
            ts_copy[*ts_index as usize].remove(pos)
        } else {
            ts_copy[*ts_index as usize][pos].clone()
        };
        for (v, f) in tuple.iter().zip(fields.iter()) {
            if let Field::Target(t) = f {
                let index = match t {
                    Target::Variable(var) => p.variable_index(&var).unwrap(),
                    Target::Array(arr, idx) => self.array_index(arr, idx, p)?,
                };

                data_copy[index as usize] = *v;
            }
        }

        Ok((data_copy, ts_copy))
    }

    fn get_updated_mem(
        &self,
        pos: usize,
        fields: &Vec<Field>,
        data: Vec<i32>,
        tuple_spaces: Vec<Vec<Vec<i32>>>,
        ts_index: &u32,
        p: &Program,
        remove: bool,
        ptr: &InstrPtr,
    ) -> Result<(Memory, InstrPtr), StepError> {
        let (data_copy, ts_copy) = self.update_mem(
            pos,
            fields,
            data.clone(),
            tuple_spaces.clone(),
            ts_index,
            p,
            remove,
        )?;

        Ok((
            Memory {
                data: data_copy,
                tuple_spaces: ts_copy,
                channels: self.memory.channels.clone(),
            },
            ptr.bump(),
        ))
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
        Either<std::array::IntoIter<(Memory, InstrPtr), 1>, std::vec::IntoIter<(Memory, InstrPtr)>>,
        StepError,
    > {
        let tuple_spaces = self.memory.tuple_spaces.clone();
        let data = self.memory.data.clone();

        match ts_type {
            TupleSpaceType::Random => {
                let mut results = Vec::new();

                for (pos, t) in tuple_spaces[*ts_index as usize].iter().enumerate() {
                    if self.matches(t, fields, p)? {
                        results.push(self.get_updated_mem(
                            pos,
                            fields,
                            data.clone(),
                            tuple_spaces.clone(),
                            ts_index,
                            p,
                            remove,
                            ptr,
                        )?);
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
                    Ok(Either::Left(
                        [self.get_updated_mem(
                            pos,
                            fields,
                            data,
                            tuple_spaces,
                            ts_index,
                            p,
                            remove,
                            ptr,
                        )?]
                        .into_iter(),
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
                    Ok(Either::Left(
                        [self.get_updated_mem(
                            pos,
                            fields,
                            data,
                            tuple_spaces,
                            ts_index,
                            p,
                            remove,
                            ptr,
                        )?]
                        .into_iter(),
                    ))
                } else {
                    return Err(StepError::Stuck);
                }
            }
            TupleSpaceType::Queue => {
                if let Some(t) = tuple_spaces[*ts_index as usize].first() {
                    if self.matches(t, fields, p)? {
                        return Ok(Either::Left(
                            [self.get_updated_mem(
                                0,
                                fields,
                                data,
                                tuple_spaces,
                                ts_index,
                                p,
                                remove,
                                ptr,
                            )?]
                            .into_iter(),
                        ));
                    }
                }
                Err(StepError::Stuck)
            }
            TupleSpaceType::Stack => {
                if let Some(t) = tuple_spaces[*ts_index as usize].last() {
                    let pos = tuple_spaces[*ts_index as usize].len() - 1;
                    if self.matches(t, fields, p)? {
                        return Ok(Either::Left(
                            [self.get_updated_mem(
                                pos,
                                fields,
                                data,
                                tuple_spaces,
                                ts_index,
                                p,
                                remove,
                                ptr,
                            )?]
                            .into_iter(),
                        ));
                    }
                }
                Err(StepError::Stuck)
            }
        }
    }

    fn find_tuple_p(
        &self,
        p: &Program,
        ts_name: &TupleSpaceName,
        field: &Vec<Field>,
        remove: bool,
    ) -> Vec<Memory> {
        let ts_index = p.tuple_space_index(&ts_name.0).unwrap();
        let ts_type = p.tuple_spaces[ts_index as usize].space_type.clone();
        self.match_tuple_space_type(&ts_type, &ts_index, field, p, &InstrPtr(0), remove)
            .map(|either| {
                either
                    .into_iter()
                    .map(|(m, _)| Memory {
                        data: m.data,
                        tuple_spaces: m.tuple_spaces,
                        channels: m.channels,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn eval_bool_guard(&self, p: &Program, expr: &BExpr) -> Vec<Memory> {
        match expr {
            BExpr::OP(OperationP::GetP(t, f)) => self.find_tuple_p(p, t, f, true),
            BExpr::OP(OperationP::QueryP(t, f)) => self.find_tuple_p(p, t, f, false),
            BExpr::OP(OperationP::PutP(t, args)) => {
                let ts_index = p.tuple_space_index(&t.0).unwrap();
                let ts_meta = &p.tuple_spaces[ts_index as usize];
                if let BufferSize::Finite(max) = ts_meta.size {
                    if self.memory.tuple_spaces[ts_index as usize].len() >= max as usize {
                        return vec![];
                    }
                }
                let mut ts = self.memory.tuple_spaces.clone();
                let values = args
                    .iter()
                    .map(|e| e.evaluate(p, self).unwrap_or(0))
                    .collect();
                ts[ts_index as usize].push(values);
                vec![Memory {
                    data: self.memory.data.clone(),
                    tuple_spaces: ts,
                    channels: self.memory.channels.clone(),
                }]
            }
            BExpr::Logic(l, LogicOp::And | LogicOp::Land, r) => self
                .eval_bool_guard(p, l)
                .into_iter()
                .flat_map(|m| {
                    State {
                        memory: Memory {
                            data: m.data,
                            tuple_spaces: m.tuple_spaces,
                            channels: m.channels,
                        },
                        ptrs: self.ptrs.clone(),
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
                        .flat_map(|m| {
                            State {
                                memory: Memory {
                                    data: m.data.clone(),
                                    tuple_spaces: m.tuple_spaces.clone(),
                                    channels: m.channels.clone(),
                                },
                                ptrs: self.ptrs.clone(),
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
                    vec![Memory {
                        data: self.memory.data.clone(),
                        tuple_spaces: self.memory.tuple_spaces.clone(),
                        channels: self.memory.channels.clone(),
                    }]
                } else {
                    vec![]
                }
            }
        }
    }

    fn eval_guard(&self, p: &Program, cg: &CG) -> Result<Vec<Memory>, StepError> {
        match cg {
            CG::BoolExpression(expr) => Ok(self.eval_bool_guard(p, expr)),
            CG::Send(ch, expr) => {
                if p.channel_index(&ch.0).is_none() {
                    return Ok(vec![]);
                }
                if let Ok(value) = expr.evaluate(p, self) {
                    let mut channels = self.memory.channels.clone();
                    let ch_index = p.channel_index(&ch.0).unwrap();
                    let buffer_size = p.channels[ch_index as usize].size.clone();

                    match buffer_size {
                        BufferSize::Finite(max_size) => {
                            if channels[ch_index as usize].len() < max_size as usize {
                                channels[ch_index as usize].push(value);
                                return Ok(vec![Memory {
                                    data: self.memory.data.clone(),
                                    tuple_spaces: self.memory.tuple_spaces.clone(),
                                    channels,
                                }]);
                            } else {
                                return Ok(vec![]);
                            }
                        }
                        BufferSize::Infinite => {
                            channels[ch_index as usize].push(value);
                            return Ok(vec![Memory {
                                data: self.memory.data.clone(),
                                tuple_spaces: self.memory.tuple_spaces.clone(),
                                channels,
                            }]);
                        }
                    }
                };
                Ok(vec![])
            }
            CG::Receive(ch, target) => {
                if p.channel_index(&ch.0).is_none() {
                    return Ok(vec![]);
                }
                let mut channels = self.memory.channels.clone();
                let mut data = self.memory.data.clone();
                let ch_index = p.channel_index(&ch.0).unwrap();
                let index = match target {
                    Target::Variable(var) => p.variable_index(&var).unwrap(),
                    Target::Array(arr, idx) => self.array_index(arr, idx, p)?,
                };

                if let Some(v) = channels[ch_index as usize].first() {
                    data[index as usize] = *v;
                    channels[ch_index as usize].remove(0);

                    return Ok(vec![Memory {
                        data,
                        tuple_spaces: self.memory.tuple_spaces.clone(),
                        channels,
                    }]);
                }
                Ok(vec![])
            }
        }
    }

    fn step_at(
        &self,
        p: &Program,
        ptr: InstrPtr,
    ) -> Result<impl Iterator<Item = (Memory, InstrPtr)>, StepError> {
        match &p[ptr] {
            Instr::Nop => Ok(Either::Left(
                [(
                    Memory {
                        data: self.memory.data.clone(),
                        tuple_spaces: self.memory.tuple_spaces.clone(),
                        channels: self.memory.channels.clone(),
                    },
                    ptr.bump(),
                )]
                .into_iter(),
            )),
            Instr::Assign(target, e) => {
                let value = e.evaluate(p, self)?;
                let mut data = self.memory.data.clone();

                let index = match target {
                    Target::Variable(var) => p.variable_index(&var).unwrap(),
                    Target::Array(arr, idx) => self.array_index(arr, idx, p)?,
                };

                data[index as usize] = value;

                Ok(Either::Left(
                    [(
                        Memory {
                            data,
                            tuple_spaces: self.memory.tuple_spaces.clone(),
                            channels: self.memory.channels.clone(),
                        },
                        ptr.bump(),
                    )]
                    .into_iter(),
                ))
            }
            Instr::Branch { choices, otherwise } => {
                let mut valid = Vec::new();
                for (cg, target) in choices {
                    for mem in self.eval_guard(p, cg)? {
                        valid.push((mem, *target));
                    }
                }
                if valid.is_empty() {
                    if let Some(target) = otherwise {
                        Ok(Either::Left(
                            [(
                                Memory {
                                    data: self.memory.data.clone(),
                                    tuple_spaces: self.memory.tuple_spaces.clone(),
                                    channels: self.memory.channels.clone(),
                                },
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
                    Memory {
                        data: self.memory.data.clone(),
                        tuple_spaces: self.memory.tuple_spaces.clone(),
                        channels: self.memory.channels.clone(),
                    },
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

                let mut tuple_spaces = self.memory.tuple_spaces.clone();

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
                        Memory {
                            data: self.memory.data.clone(),
                            tuple_spaces,
                            channels: self.memory.channels.clone(),
                        },
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
                let mut channels = self.memory.channels.clone();

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
                        Memory {
                            data: self.memory.data.clone(),
                            tuple_spaces: self.memory.tuple_spaces.clone(),
                            channels,
                        },
                        ptr.bump(),
                    )]
                    .into_iter(),
                ))
            }
            Instr::Receive(ch_index, target) => {
                let mut channels = self.memory.channels.clone();
                let mut data = self.memory.data.clone();

                let index = match target {
                    Target::Variable(var) => p.variable_index(&var).unwrap(),
                    Target::Array(arr, idx) => self.array_index(arr, idx, p)?,
                };

                if let Some(v) = channels[*ch_index as usize].first() {
                    data[index as usize] = *v;
                    channels[*ch_index as usize].remove(0);

                    return Ok(Either::Left(
                        [(
                            Memory {
                                data,
                                tuple_spaces: self.memory.tuple_spaces.clone(),
                                channels,
                            },
                            ptr.bump(),
                        )]
                        .into_iter(),
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
        let vars = self.memory.data.iter().format(" ");
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

        for (idx, target_meta) in self.program.targets.iter().enumerate() {
            if let Target::Variable(var) = &target_meta.target {
                let value = self.state.memory.data[idx];
                parts.push(format!("{var} = {value}"));
            }
        }

        for (arr, base, len) in self.program.arrays() {
            let array_values: Vec<String> = self.state.memory.data
                [base as usize..(base + len) as usize]
                .iter()
                .map(|v| v.to_string())
                .collect();
            parts.push(format!("{arr} = [{}]", array_values.join(", ")));
        }

        for (ts_meta, ts_values) in self
            .program
            .tuple_spaces
            .iter()
            .zip(&self.state.memory.tuple_spaces)
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

        for (ch_meta, ch_values) in self
            .program
            .channels
            .iter()
            .zip(&self.state.memory.channels)
        {
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
                    Target::Variable(var) => p.variable_index(&var).unwrap(),
                    Target::Array(arr, idx) => state.array_index(arr, idx, p)?,
                };
                state.memory.data[index as usize]
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
                let index = p.channel_index(&c.0).unwrap();
                state.memory.channels[index as usize]
                    .first()
                    .map_or(false, |v| e == *v)
            }
            ChannelFormula::ChannelContains(c, e) => {
                let e = e.evaluate(p, state)?;
                let index = p.channel_index(&c.0).unwrap();
                state.memory.channels[index as usize].contains(&e)
            }
        })
    }
}

impl LTLFormula {
    pub fn to_mcltl(
        &self,
        rels: &mut Vec<(AExpr, RelOp, AExpr)>,
        operations: &mut Vec<OperationP>,
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
            LTLFormula::OperationP(op) => {
                let idx = if let Some(idx) = operations.iter().position(|x| x == op) {
                    idx
                } else {
                    operations.push(op.clone());
                    operations.len() - 1
                };
                LTLExpression::Literal(format!("o{idx}").into())
            }
            LTLFormula::ChannelFormula(cf) => {
                let idx = if let Some(idx) = channels.iter().position(|x| x == cf) {
                    idx
                } else {
                    channels.push(cf.clone());
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
