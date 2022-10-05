use std::collections::HashMap;

use nom::branch::alt;
use nom::bytes::complete::{is_not, tag};
use nom::character::complete::{
    alpha1, alphanumeric0, alphanumeric1, digit1, newline, space0, space1,
};
use nom::character::is_alphabetic;
use nom::combinator::{map, map_res, opt};
use nom::error::{FromExternalError, ParseError};
use nom::multi::{many0, many1, separated_list0, separated_list1};
use nom::sequence::{pair, preceded, terminated, tuple};
use nom::IResult;
use simplez_common::*;

pub use nom;
use twelve_bit::u12;
use twelve_bit::u12::*;

#[derive(Copy, Clone, Debug)]
pub enum ParamType {
    Direction,
    Number,
}

#[derive(Debug)]
pub enum ErrorKind {
    InvalidParameter { expected_type: ParamType },
    InvalidNumber,
    UndefinedLabel { name: String },
    InvalidInstruction { name: String },
    RedefinedLabel { name: String },
    MissingParameter,
    InvalidLabelName,
    SyntaxError,
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

impl<I> FromExternalError<I, Error<I>> for Error<I> {
    fn from_external_error(_input: I, _kind: nom::error::ErrorKind, e: Error<I>) -> Self {
        e
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

#[derive(Clone, Copy, Debug)]
pub enum Parameter<'s> {
    Direction(Direction<'s>),
    Number(U12),
}

pub enum Directive {
    Org { address: Address },
    Data { value: U12 },
    Reserve { amount: U12 },
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
        tag("/"),
        alt((
            map(parse_label, |label| Direction::Label(label)),
            map(
                pair(opt(tag("-")), digit1),
                |(sign, addr): (Option<&str>, &str)| {
                    if let Some(_sign) = sign {
                        Direction::Address(Address(u12!(4096) - addr.parse::<U12>().unwrap()))
                    } else {
                        Direction::Address(Address(addr.parse::<U12>().unwrap()))
                    }
                },
            ),
        )),
    )(input)
}

pub fn parse_parameter<'s>(input: &'s str) -> IResult<&str, Parameter<'s>, Error<&str>> {
    let dir_parser = map(parse_direction, Parameter::Direction);
    let num_parser = map_res(digit1, |num| {
        Ok(Parameter::Number(str::parse(num).map_err(|_| Error {
            input,
            kind: ErrorKind::InvalidNumber,
        })?))
    });
    alt((dir_parser, num_parser))(input)
}

pub fn parse_assembly_line<'s>(input: &'s str) -> IResult<&str, AssemblyLine<'s>, Error<&str>> {
    let (input, (label, _, instruction, _)) = terminated(
        tuple((
            alphanumeric0,
            space1,
            opt(tuple((
                alpha1,
                opt(preceded(space1, separated_list1(space1, parse_parameter))),
            ))),
            opt(tuple((space0, tag(";"), many0(is_not("\n"))))),
        )),
        space0,
    )(input)?;

    let label = if label.is_empty() { None } else { Some(label) };

    let instruction = instruction.map(|(instruction, params)| -> Result<Command, Error<&'s str>> {
        let get_number = || match params.as_ref().map(|p| p[0]).ok_or(Error {
            input,
            kind: ErrorKind::MissingParameter,
        })? {
            Parameter::Number(num) => Ok(num),
            Parameter::Direction(_) => Err(Error {
                input,
                kind: ErrorKind::InvalidParameter {
                    expected_type: ParamType::Number,
                },
            }),
        };
        let get_dir = || match params.as_ref().map(|p| p[0]).ok_or(Error {
            input,
            kind: ErrorKind::MissingParameter,
        })? {
            Parameter::Direction(dir) => Ok(dir),
            Parameter::Number(_) => Err(Error {
                input,
                kind: ErrorKind::InvalidParameter {
                    expected_type: ParamType::Direction,
                },
            }),
        };
        Ok(match instruction.to_owned().to_lowercase().as_str() {
            "st" => Command::Instruction(Instruction::Store {
                address: get_dir()?,
            }),
            "ld" => Command::Instruction(Instruction::Load {
                address: get_dir()?,
            }),
            "add" => Command::Instruction(Instruction::Add {
                address: get_dir()?,
            }),
            "br" => Command::Instruction(Instruction::Branch {
                address: get_dir()?,
            }),
            "bz" => Command::Instruction(Instruction::BranchIfZero {
                address: get_dir()?,
            }),
            "clr" => Command::Instruction(Instruction::Clear),
            "dec" => Command::Instruction(Instruction::Decrease),
            "halt" => Command::Instruction(Instruction::Halt),

            "org" => Command::Directive(Directive::Org {
                address: get_number().map(Address)?,
            }),

            // TODO: Support for character data directives
            "data" => Command::Directive(Directive::Data {
                value: get_number()?,
            }),

            "res" => Command::Directive(Directive::Reserve {
                amount: get_number()?,
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

pub fn assemble<'s>(input: &'s str) -> Result<Memory, Error<&str>> {
    let mut labels = HashMap::new();
    let lines = preceded(
        many0(newline),
        separated_list0(many1(newline), parse_assembly_line),
    )(input)
    .map_err(|err| match err {
        nom::Err::Error(x) => x,
        nom::Err::Failure(x) => x,
        nom::Err::Incomplete(_) => unreachable!(),
    })?;
    let lines = if !lines.0.trim().is_empty() {
        return Err(Error {
            input: lines.0,
            kind: ErrorKind::SyntaxError,
        });
    } else {
        lines.1
    };
    {
        let mut current_addr = Address::default();
        for line in lines.iter() {
            if let Some(label) = line.label {
                if labels.contains_key(&label) {
                    return Err(Error {
                        input,
                        kind: ErrorKind::RedefinedLabel {
                            name: label.to_string(),
                        },
                    });
                }
                labels.insert(label, current_addr);
            }
            match line.command {
                Some(Command::Directive(Directive::Org { address })) => current_addr = address,
                Some(Command::Directive(Directive::Reserve { amount })) => current_addr.0 += amount,
                Some(_) => current_addr.0 += u12!(1),
                None => (),
            }
        }
    }

    let convert_direction = |dir: Direction| -> Result<Address, Error<&str>> {
        match dir {
            Direction::Address(addr) => Ok(addr),
            Direction::Label(label) => labels.get(&label).copied().ok_or_else(|| Error {
                input,
                kind: ErrorKind::UndefinedLabel {
                    name: label.to_string(),
                },
            }),
        }
    };

    let mut memory = Memory::default();
    let mut current_addr = Address::default();

    for command in lines.into_iter().filter_map(|line| line.command) {
        match command {
            Command::Instruction(instruction) => {
                memory[current_addr] = match instruction {
                    Instruction::Store { address } => convert_direction(address)?.0 & u12!(0o777),
                    Instruction::Load { address } => {
                        u12!(1 << 9) | convert_direction(address)?.0 & u12!(0o777)
                    }
                    Instruction::Add { address } => {
                        u12!(2 << 9) | convert_direction(address)?.0 & u12!(0o777)
                    }
                    Instruction::Branch { address } => {
                        u12!(3 << 9) | convert_direction(address)?.0 & u12!(0o777)
                    }
                    Instruction::BranchIfZero { address } => {
                        u12!(4 << 9) | convert_direction(address)?.0 & u12!(0o777)
                    }
                    Instruction::Clear => u12!(5 << 9),
                    Instruction::Decrease => u12!(6 << 9),
                    Instruction::Halt => u12!(7 << 9),
                };
                current_addr.0 += u12!(1);
            }
            Command::Directive(directive) => match directive {
                Directive::Org { address } => current_addr = address,
                Directive::Data { value } => {
                    memory[current_addr] = value;
                    current_addr.0 += u12!(1);
                }
                Directive::Reserve { amount } => current_addr.0 += amount,
                Directive::End => return Ok(memory),
            },
        }
    }

    Ok(memory)
}

#[cfg(test)]
#[test]
fn test() {
    let asm = include_str!("../../test.sz").to_owned();
    let result = assemble(&asm).unwrap();
    let lines = asm.lines();
    let mut words = result.iter();

    for line in lines {
        let asm_instruction = parse_assembly_line(line).ok().map(|line| line.1.command);
        if asm_instruction.is_some() {
            println!("{:04o} {}", u16::from(*words.next().unwrap()), line);
        } else {
            println!("     {}", line);
        }
    }

    // All lines with instructions should map 1:1 to words
    assert!(words.next() == None);
}
