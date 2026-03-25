use crate::{ast::*, parse::SourceSpan};
use indexmap::IndexMap;
use lalrpop_util::ParseError;

type LTLParseError<'a> =
    ParseError<usize, lalrpop_util::lexer::Token<'a>, crate::parse::CustomError>;

pub fn validate_ltl_program<'a>(
    assignments: (
        IndexMap<Variable, i32>,
        IndexMap<Array, Vec<i32>>,
        IndexMap<Variable, TupleSpace>,
        IndexMap<Variable, Channel>,
    ),
    commands: Vec<Commands<(), ()>>,
    properties: Vec<LTLProperty>,
) -> Result<LTLProgram, LTLParseError<'a>> {
    let (init_variables, init_arrays, init_tuple_spaces, init_channels) = assignments;

    for commands in &commands {
        for command in &commands.0 {
            validate_command(command, &init_tuple_spaces, &init_channels)?;
        }
    }

    for (span, property) in &properties {
        validate_property(property, span, &init_tuple_spaces, &init_channels)?;
    }

    Ok(LTLProgram {
        init_variables,
        init_arrays,
        init_tuple_spaces,
        init_channels,
        commands,
        properties,
    })
}

fn validate_command<'a>(
    command: &Command<(), ()>,
    tuple_spaces: &IndexMap<Variable, TupleSpace>,
    _channels: &IndexMap<Variable, Channel>,
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
        CommandKind::Loop(_, guards) => {
            for guard in guards {
                validate_guard_operations(&guard.guard, tuple_spaces, &command.span)?;
            }
        }
        CommandKind::IfCG(cgs) | CommandKind::LoopCG(_, cgs) => {
            for cg in cgs {
                validate_communication_guard(cg, tuple_spaces, &command.span)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_guard_operations<'a>(
    guard: &BExpr,
    tuple_spaces: &IndexMap<Variable, TupleSpace>,
    span: &SourceSpan,
) -> Result<(), LTLParseError<'a>> {
    match guard {
        BExpr::OP(op) => {
            let name = extract_operation_name(op);
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
        _ => {}
    }
    Ok(())
}

fn validate_communication_guard<'a>(
    cg: &CommunicationGuard<(), ()>,
    tuple_spaces: &IndexMap<Variable, TupleSpace>,
    span: &SourceSpan,
) -> Result<(), LTLParseError<'a>> {
    match &cg.guard {
        CG::BoolExpression(expr) => {
            validate_guard_operations(expr, tuple_spaces, span)?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_property<'a>(
    property: &LTLFormula,
    span: &SourceSpan,
    tuple_spaces: &IndexMap<Variable, TupleSpace>,
    channels: &IndexMap<Variable, Channel>,
) -> Result<(), LTLParseError<'a>> {
    match property {
        LTLFormula::Operation(op) => {
            let name = extract_operation_name(op);
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
                ChannelFormula::ChannelHead(name, _)
                | ChannelFormula::ChannelContains(name, _) => name,
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
        _ => {}
    }
    Ok(())
}

fn extract_operation_name<'a>(operation: &'a Operation) -> &'a Variable {
    match operation {
        Operation::Put(name, _) | Operation::Get(name, _) | Operation::Query(name, _) => {
            name
        }
    }
}
