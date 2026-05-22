use crate::{ast::*, parse::SourceSpan};
use indexmap::IndexMap;
use lalrpop_util::ParseError;

type LTLParseError<'a> =
    ParseError<usize, lalrpop_util::lexer::Token<'a>, crate::parse::CustomError>;

pub fn validate_ltl_program<'a>(
    assignments: Initial,
    commands: Vec<Commands<(), ()>>,
    properties: Vec<LTLProperty>,
) -> Result<LTLProgram, LTLParseError<'a>> {
    let initial = assignments;

    for commands in &commands {
        for command in &commands.0 {
            validate_command(
                command,
                &initial.tuple_spaces,
                &initial.async_channels,
                &initial.sync_channels,
            )?;
        }
    }

    for (span, property) in &properties {
        validate_property(
            property,
            span,
            &initial.tuple_spaces,
            &initial.async_channels,
        )?;
    }

    Ok(LTLProgram {
        initial,
        commands,
        properties,
    })
}

fn validate_command<'a>(
    command: &Command<(), ()>,
    tuple_spaces: &IndexMap<TupleSpaceName, TupleSpace>,
    async_channels: &IndexMap<ChannelName, Channel>,
    sync_channels: &Vec<ChannelName>,
) -> Result<(), LTLParseError<'a>> {
    match &command.kind {
        CommandKind::O(operation) => {
            let name = extract_operation_name(operation);
            if !tuple_spaces.contains_key(name) {
                return Err(ParseError::User {
                    error: crate::parse::CustomError::Undefined {
                        name: name.0.clone(),
                        from: command.span.offset(),
                        to: command.span.offset() + name.0.len(),
                    },
                });
            }
        }
        CommandKind::Send(c, _) | CommandKind::Receive(c, _) => {
            if !async_channels.contains_key(c) && !sync_channels.contains(c) {
                return Err(ParseError::User {
                    error: crate::parse::CustomError::Undefined {
                        name: c.0.clone(),
                        from: command.span.offset(),
                        to: command.span.offset() + c.0.len(),
                    },
                });
            }
        }
        CommandKind::Broadcast(c, _, _) | CommandKind::Gather(c, _, _, _) => {
            if !sync_channels.contains(c) {
                return Err(ParseError::User {
                    error: crate::parse::CustomError::Undefined {
                        name: c.0.clone(),
                        from: command.span.offset(),
                        to: command.span.offset() + c.0.len(),
                    },
                });
            }
        }
        CommandKind::Loop(_, guards) => {
            for guard in guards {
                validate_guard_operations(&guard.guard, tuple_spaces, &command.span)?;
                for cmd in &guard.cmds.0 {
                    validate_command(cmd, tuple_spaces, async_channels, sync_channels)?;
                }
            }
        }
        CommandKind::IfCG(cgs) | CommandKind::LoopCG(_, cgs) => {
            for cg in cgs {
                validate_communication_guard(
                    cg,
                    tuple_spaces,
                    async_channels,
                    sync_channels,
                    &command.span,
                )?;
                for cmd in &cg.cmds.0 {
                    validate_command(cmd, tuple_spaces, async_channels, sync_channels)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_guard_operations<'a>(
    guard: &BExpr,
    tuple_spaces: &IndexMap<TupleSpaceName, TupleSpace>,
    span: &SourceSpan,
) -> Result<(), LTLParseError<'a>> {
    match guard {
        BExpr::OP(op) => {
            let name = extract_operation_p_name(op);
            if !tuple_spaces.contains_key(name) {
                return Err(ParseError::User {
                    error: crate::parse::CustomError::Undefined {
                        name: name.0.clone(),
                        from: span.offset(),
                        to: span.offset() + name.0.len(),
                    },
                });
            }
        }
        BExpr::Logic(b1, _, b2) => {
            validate_guard_operations(b1, tuple_spaces, span)?;
            validate_guard_operations(b2, tuple_spaces, span)?;
        }
        BExpr::Not(b) => {
            validate_guard_operations(b, tuple_spaces, span)?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_communication_guard<'a>(
    cg: &CommunicationGuard<(), ()>,
    tuple_spaces: &IndexMap<TupleSpaceName, TupleSpace>,
    async_channels: &IndexMap<ChannelName, Channel>,
    sync_channels: &Vec<ChannelName>,
    span: &SourceSpan,
) -> Result<(), LTLParseError<'a>> {
    match &cg.guard {
        CG::BoolExpression(expr) => {
            validate_guard_operations(expr, tuple_spaces, span)?;
        }
        CG::Send(c, _) | CG::Receive(c, _) => {
            if !async_channels.contains_key(c) && !sync_channels.contains(c) {
                return Err(ParseError::User {
                    error: crate::parse::CustomError::Undefined {
                        name: c.0.clone(),
                        from: span.offset(),
                        to: span.offset() + c.0.len(),
                    },
                });
            }
        }
    }
    Ok(())
}

fn validate_property<'a>(
    property: &LTLFormula,
    span: &SourceSpan,
    tuple_spaces: &IndexMap<TupleSpaceName, TupleSpace>,
    channels: &IndexMap<ChannelName, Channel>,
) -> Result<(), LTLParseError<'a>> {
    match property {
        LTLFormula::OperationP(op) => {
            let name = extract_operation_p_name(op);
            if !tuple_spaces.contains_key(name) {
                return Err(ParseError::User {
                    error: crate::parse::CustomError::Undefined {
                        name: name.0.clone(),
                        from: span.offset(),
                        to: span.offset() + name.0.len(),
                    },
                });
            }
        }
        LTLFormula::ChannelFormula(cf) => {
            let name = match cf {
                ChannelFormula::ChannelHead(name, _) | ChannelFormula::ChannelContains(name, _) => {
                    name
                }
            };
            if !channels.contains_key(name) {
                return Err(ParseError::User {
                    error: crate::parse::CustomError::Undefined {
                        name: name.0.clone(),
                        from: span.offset(),
                        to: span.offset() + name.0.len(),
                    },
                });
            }
        }
        LTLFormula::Not(f)
        | LTLFormula::Next(f)
        | LTLFormula::Globally(f)
        | LTLFormula::Finally(f) => {
            validate_property(f, span, tuple_spaces, channels)?;
        }
        LTLFormula::And(f1, f2)
        | LTLFormula::Or(f1, f2)
        | LTLFormula::Implies(f1, f2)
        | LTLFormula::Until(f1, f2) => {
            validate_property(f1, span, tuple_spaces, channels)?;
            validate_property(f2, span, tuple_spaces, channels)?;
        }
        _ => {}
    }
    Ok(())
}

fn extract_operation_name<'a>(operation: &'a Operation) -> &'a TupleSpaceName {
    match operation {
        Operation::Put(name, _) | Operation::Get(name, _) | Operation::Query(name, _) => name,
    }
}

fn extract_operation_p_name<'a>(operation_p: &'a OperationP) -> &'a TupleSpaceName {
    match operation_p {
        OperationP::PutP(name, _) | OperationP::GetP(name, _) | OperationP::QueryP(name, _) => name,
    }
}
