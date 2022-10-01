use std::collections::HashMap;

use nom::branch::alt;
use nom::bytes::complete::{is_a, is_not, tag};
use nom::character::complete::{
    alpha1, alphanumeric0, alphanumeric1, digit1, newline, space0, space1,
};
use nom::character::{is_alphabetic, is_newline};
use nom::combinator::{map, map_res, opt};
use nom::error::{FromExternalError, ParseError};
use nom::multi::{many0, many1, separated_list0, separated_list1};
use nom::sequence::{pair, preceded, terminated, tuple};
use nom::{Finish, IResult};
use simplez_common::*;
use thiserror::Error;

pub use nom;

#[derive(Debug)]
pub enum ErrorKind {
    InvalidParameter,
    UndefinedLabel { name: String },
    InvalidInstruction { name: String },
    RedefinedLabel { name: String },
    MissingParameter,
    InvalidLabelName,
    ParseError(nom::error::ErrorKind),
}

#[derive(Debug)]
pub struct Error<I> {
    pub input: I,
    pub kind: ErrorKind,
}

impl<I> ParseError<I> for Error<I> {
    fn from_error_kind(input: I, kind: nom::error::ErrorKind) -> Self {
        Self {
            input,
            kind: ErrorKind::ParseError(kind),
        }
    }

    fn append(_: I, _: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

pub struct AssemblyLine<'s> {
    label: Option<&'s str>,
    command: Option<Command<'s>>,
}

#[derive(Clone, Copy, Debug)]
pub enum Direction<'s> {
    Label(&'s str),
    Address(Address),
}

pub enum Directive {
    Org { address: Address },
    Data { value: u16 },
    Reserve { amount: u16 },
    End,
}

pub enum Command<'s> {
    Directive(Directive),
    Instruction(Instruction<Direction<'s>>),
}

pub fn parse_label<'s>(input: &'s str) -> IResult<&str, &str, Error<&str>> {
    if let Some(first_char) = input.bytes().nth(0) {
        if is_alphabetic(first_char) {
            alphanumeric1(input)
        } else {
            Err(nom::Err::Error(Error {
                input,
                kind: ErrorKind::InvalidLabelName,
            }))
        }
    } else {
        Err(nom::Err::Error(Error {
            input,
            kind: ErrorKind::InvalidLabelName,
        }))
    }
}

pub fn parse_direction<'s>(input: &'s str) -> IResult<&str, Direction<'s>, Error<&str>> {
    preceded(
        opt(tag("/")),
        alt((
            map(parse_label, |label| Direction::Label(label)),
            map(digit1, |addr: &str| {
                Direction::Address(Address(addr.parse::<u16>().unwrap()))
            }),
        )),
    )(input)
}

pub fn parse_assembly_line<'s>(input: &'s str) -> IResult<&str, AssemblyLine<'s>, Error<&str>> {
    let (input, (label, _, instruction, _)) = terminated(
        tuple((
            alphanumeric0,
            space1,
            opt(tuple((
                alpha1,
                opt(preceded(space1, separated_list1(space1, parse_direction))),
            ))),
            opt(tuple((space0, tag(";"), many0(is_not("\n"))))),
        )),
        space0,
    )(input)?;

    let label = if label.is_empty() { None } else { Some(label) };

    let instruction = instruction.map(|(instruction, params)| -> Result<Command, Error<&'s str>> {
        let get_param = || {
            params.map(|p| p[0]).ok_or(Error {
                input,
                kind: ErrorKind::MissingParameter,
            })
        };
        Ok(match instruction.to_owned().to_lowercase().as_str() {
            "st" => Command::Instruction(Instruction::Store {
                address: get_param()?,
            }),
            "ld" => Command::Instruction(Instruction::Load {
                address: get_param()?,
            }),
            "add" => Command::Instruction(Instruction::Add {
                address: get_param()?,
            }),
            "br" => Command::Instruction(Instruction::Branch {
                address: get_param()?,
            }),
            "bz" => Command::Instruction(Instruction::BranchIfZero {
                address: get_param()?,
            }),
            "clr" => Command::Instruction(Instruction::Clear),
            "dec" => Command::Instruction(Instruction::Decrease),
            "halt" => Command::Instruction(Instruction::Halt),

            "org" => Command::Directive(Directive::Org {
                address: get_param().and_then(|direction| match direction {
                    Direction::Address(addr) => Ok(addr),
                    _ => Err(Error {
                        input,
                        kind: ErrorKind::InvalidParameter,
                    }),
                })?,
            }),

            // TODO: Support for character data directives
            "data" => Command::Directive(Directive::Data {
                value: get_param().and_then(|direction| match direction {
                    Direction::Address(addr) => Ok(addr.0),
                    _ => Err(Error {
                        input,
                        kind: ErrorKind::InvalidParameter,
                    }),
                })?,
            }),

            "res" => Command::Directive(Directive::Reserve {
                amount: get_param().and_then(|direction| match direction {
                    Direction::Address(addr) => Ok(addr.0),
                    _ => Err(Error {
                        input,
                        kind: ErrorKind::InvalidParameter,
                    }),
                })?,
            }),

            "end" => Command::Directive(Directive::End),

            other => {
                return Err(Error {
                    input,
                    kind: ErrorKind::InvalidInstruction {
                        name: other.to_string(),
                    },
                })
            }
        })
    });
    let instruction = match instruction {
        Some(x) => Some(x.map_err(|err| nom::Err::Failure(err))?),
        None => None,
    };

    Ok((
        input,
        AssemblyLine {
            command: instruction,
            label,
        },
    ))
}

pub fn assemble<'s>(input: &'s str) -> IResult<&str, [u16; 512], Error<&str>> {
    let mut labels = HashMap::new();
    let lines = preceded(
        many0(newline),
        separated_list0(many1(newline), parse_assembly_line),
    )(input)?
    .1;
    {
        let mut current_addr = Address::ZERO;
        for line in lines.iter() {
            if let Some(label) = line.label {
                if labels.contains_key(&label) {
                    return Err(nom::Err::Failure(Error {
                        input,
                        kind: ErrorKind::RedefinedLabel {
                            name: label.to_string(),
                        },
                    }));
                }
                labels.insert(label, current_addr);
            }
            match line.command {
                Some(Command::Directive(Directive::Org { address })) => current_addr = address,
                Some(Command::Directive(Directive::Reserve { amount })) => current_addr.0 += amount,
                Some(_) => current_addr.0 += 1,
                None => (),
            }
        }
    }

    let convert_direction = |dir: Direction| -> Result<Address, nom::Err<Error<&str>>> {
        match dir {
            Direction::Address(addr) => Ok(addr),
            Direction::Label(label) => labels.get(&label).copied().ok_or_else(|| {
                nom::Err::Failure(Error {
                    input,
                    kind: ErrorKind::UndefinedLabel {
                        name: label.to_string(),
                    },
                })
            }),
        }
    };

    dbg!(&labels);

    let mut memory = [0; 512];
    let mut current_addr = Address(0);

    for command in lines.into_iter().filter_map(|line| line.command) {
        match command {
            Command::Instruction(instruction) => {
                memory[current_addr.0 as usize] = match instruction {
                    Instruction::Store { address } => convert_direction(address)?.0 & 0o777,
                    Instruction::Load { address } => {
                        (1 << 9) | convert_direction(address)?.0 & 0o777
                    }
                    Instruction::Add { address } => {
                        (2 << 9) | convert_direction(address)?.0 & 0o777
                    }
                    Instruction::Branch { address } => {
                        (3 << 9) | convert_direction(address)?.0 & 0o777
                    }
                    Instruction::BranchIfZero { address } => {
                        (4 << 9) | convert_direction(address)?.0 & 0o777
                    }
                    Instruction::Clear => 5 << 9,
                    Instruction::Decrease => 6 << 9,
                    Instruction::Halt => 7 << 9,
                };
                current_addr.0 += 1;
            }
            Command::Directive(directive) => match directive {
                Directive::Org { address } => current_addr = address,
                Directive::Data { value } => {
                    memory[current_addr.0 as usize] = value;
                    current_addr.0 += 1;
                }
                Directive::Reserve { amount } => current_addr.0 += amount,
                Directive::End => return Ok((input, memory)),
            },
        }
    }

    Ok((input, memory))
}

#[cfg(test)]
#[test]
fn test() {
    let asm = include_str!("../../test.sz").to_owned();
    let result = assemble(&asm).unwrap().1;
    let lines = asm.lines();
    let mut words = result.iter();

    for line in lines {
        let asm_instruction = parse_assembly_line(line).ok().map(|line| line.1.command);
        if asm_instruction.is_some() {
            println!("{:04o} {}", words.next().unwrap(), line);
        } else {
            println!("     {}", line);
        }
    }

    // All lines with instructions should map 1:1 to words
    assert!(words.next() == None);
}
