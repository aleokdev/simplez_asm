use std::collections::HashMap;

use nom::branch::alt;
use nom::bytes::complete::{is_a, tag};
use nom::character::complete::{alpha1, alphanumeric0, alphanumeric1, digit1, newline, space1};
use nom::character::is_alphabetic;
use nom::combinator::{map, map_res, opt};
use nom::error::{FromExternalError, ParseError};
use nom::multi::{many1, separated_list0, separated_list1};
use nom::sequence::{pair, preceded, tuple};
use nom::IResult;
use simplez_common::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error<I> {
    UndefinedLabel { name: String },
    InvalidInstruction { name: String },
    RedefinedLabel { name: String },
    MissingParameter,
    InvalidLabelName,
    ParseError(#[from] nom::error::Error<I>),
}

impl<I> ParseError<I> for Error<I> {
    fn from_error_kind(input: I, kind: nom::error::ErrorKind) -> Self {
        Self::ParseError(nom::error::Error::from_error_kind(input, kind))
    }

    fn append(_: I, _: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

impl<I> FromExternalError<I, Error<I>> for Error<I> {
    fn from_external_error(_: I, _: nom::error::ErrorKind, e: Error<I>) -> Self {
        e
    }
}

pub struct AssemblyLine<'s> {
    label: Option<&'s str>,
    instruction: Option<Instruction<Direction<'s>>>,
}

#[derive(Clone, Copy, Debug)]
pub enum Direction<'s> {
    Label(&'s str),
    Address(Address),
}

pub fn parse_label<'s>(input: &'s str) -> IResult<&str, &str, Error<&str>> {
    if let Some(first_char) = input.bytes().nth(0) {
        if is_alphabetic(first_char) {
            alphanumeric1(input)
        } else {
            Err(nom::Err::Error(Error::InvalidLabelName))
        }
    } else {
        Err(nom::Err::Error(Error::InvalidLabelName))
    }
}

pub fn parse_direction<'s>(input: &'s str) -> IResult<&str, Direction<'s>, Error<&str>> {
    alt((
        map(parse_label, |label| Direction::Label(label)),
        map(preceded(opt(tag("/")), digit1), |addr: &str| {
            Direction::Address(Address(addr.parse::<u16>().unwrap()))
        }),
    ))(input)
}

pub fn parse_assembly_line<'s>(input: &'s str) -> IResult<&str, AssemblyLine<'s>, Error<&str>> {
    map_res(
        tuple((
            alphanumeric0,
            space1,
            opt(tuple((
                alpha1,
                opt(preceded(space1, separated_list1(space1, parse_direction))),
            ))),
        )),
        |(label, _, instruction): (&str, &str, Option<(&str, Option<Vec<Direction<'s>>>)>)| {
            let label = if label.is_empty() { None } else { Some(label) };

            let instruction = instruction.map(
                |(instruction, params)| -> Result<Instruction<Direction>, Error<&'s str>> {
                    let get_param = || params.map(|p| p[0]).ok_or(Error::MissingParameter);
                    Ok(match instruction.to_owned().to_lowercase().as_str() {
                        "st" => Instruction::Store {
                            address: get_param()?,
                        },
                        "ld" => Instruction::Load {
                            address: get_param()?,
                        },
                        "add" => Instruction::Add {
                            address: get_param()?,
                        },
                        "br" => Instruction::Branch {
                            address: get_param()?,
                        },
                        "bz" => Instruction::BranchIfZero {
                            address: get_param()?,
                        },
                        "clr" => Instruction::Clear,
                        "dec" => Instruction::Decrease,
                        "halt" => Instruction::Halt,

                        other => {
                            return Err(Error::InvalidInstruction {
                                name: other.to_string(),
                            })
                        }
                    })
                },
            );
            let instruction = match instruction {
                Some(x) => Some(x?),
                None => None,
            };

            Ok(AssemblyLine { instruction, label })
        },
    )(input)
}

pub fn assemble<'s>(input: &'s str) -> IResult<&str, Vec<u16>, Error<&str>> {
    let mut labels = HashMap::new();
    let lines = separated_list0(many1(newline), parse_assembly_line)(input)?.1;
    {
        let mut current_addr = Address::ZERO;
        for line in lines.iter() {
            if let Some(label) = line.label {
                if labels.contains_key(&label) {
                    return Err(nom::Err::Failure(Error::RedefinedLabel {
                        name: label.to_string(),
                    }));
                }
                labels.insert(label, current_addr);
            }

            current_addr.0 += 1;
        }
    }

    let convert_direction = |dir: Direction| -> Result<Address, Error<&str>> {
        match dir {
            Direction::Address(addr) => Ok(addr),
            Direction::Label(label) => {
                labels
                    .get(&label)
                    .copied()
                    .ok_or_else(|| Error::UndefinedLabel {
                        name: label.to_string(),
                    })
            }
        }
    };

    Ok((
        input,
        lines
            .into_iter()
            .filter_map(|line| line.instruction)
            .map(|instruction| -> Result<_, _> {
                Ok(match instruction {
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
                })
            })
            .collect::<Result<Vec<u16>, Error<&str>>>()
            .map_err(|err| nom::Err::Failure(err))?,
    ))
}

#[cfg(test)]
#[test]
fn test() {
    let asm = include_str!("../../test.sz").to_owned();
    let result = assemble(&asm).unwrap().1;
    let lines = asm.lines();
    let mut words = result.iter();

    for line in lines {
        let asm_instruction = parse_assembly_line(line)
            .ok()
            .map(|line| line.1.instruction);
        if asm_instruction.is_some() {
            println!("{:04o} {}", words.next().unwrap(), line);
        } else {
            println!("     {}", line);
        }
    }

    // All lines with instructions should map 1:1 to words
    assert!(words.next() == None);
}
