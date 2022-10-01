use std::collections::HashMap;

use nom::branch::alt;
use nom::bytes::complete::is_a;
use nom::character::complete::{alpha1, alphanumeric0, alphanumeric1, digit1, newline, space1};
use nom::character::is_alphabetic;
use nom::combinator::{map, opt};
use nom::multi::{many1, separated_list0, separated_list1};
use nom::sequence::{pair, preceded, tuple};
use nom::IResult;
use simplez_common::*;

pub struct AssemblyLine<'s> {
    label: Option<&'s str>,
    instruction: Instruction<Direction<'s>>,
}

#[derive(Clone, Copy, Debug)]
pub enum Direction<'s> {
    Label(&'s str),
    Address(Address),
}

pub fn parse_label<'s>(input: &'s str) -> IResult<&str, &str> {
    if let Some(first_char) = input.bytes().nth(0) {
        if is_alphabetic(first_char) {
            alphanumeric1(input)
        } else {
            Err(nom::Err::Error(nom::error::Error {
                input,
                code: nom::error::ErrorKind::Alpha,
            }))
        }
    } else {
        Err(nom::Err::Error(nom::error::Error {
            input,
            code: nom::error::ErrorKind::AlphaNumeric,
        }))
    }
}

pub fn parse_direction<'s>(input: &'s str) -> IResult<&str, Direction<'s>> {
    alt((
        map(parse_label, |label| Direction::Label(label)),
        map(digit1, |addr: &str| {
            Direction::Address(Address(addr.parse::<u16>().unwrap()))
        }),
    ))(input)
}

pub fn parse_assembly_line<'s>(input: &'s str) -> IResult<&str, AssemblyLine<'s>> {
    map(
        tuple((
            alphanumeric0::<&str, nom::error::Error<&str>>,
            space1,
            alpha1,
            opt(preceded(space1, separated_list1(space1, parse_direction))),
        )),
        |(label, _, instruction, params): (&str, &str, &str, Option<Vec<Direction<'s>>>)| {
            let label = if label.is_empty() { None } else { Some(label) };

            let instruction = match instruction.to_owned().to_lowercase().as_str() {
                "st" => Instruction::Store {
                    address: params.unwrap()[0],
                },
                "ld" => Instruction::Load {
                    address: params.unwrap()[0],
                },
                "add" => Instruction::Add {
                    address: params.unwrap()[0],
                },
                "br" => Instruction::Branch {
                    address: params.unwrap()[0],
                },
                "bz" => Instruction::BranchIfZero {
                    address: params.unwrap()[0],
                },
                "clr" => Instruction::Clear,
                "dec" => Instruction::Decrease,
                "halt" => Instruction::Halt,

                _ => panic!(),
            };

            AssemblyLine { instruction, label }
        },
    )(input)
}

pub fn assemble<'s>(input: &'s str) -> IResult<&str, Vec<u16>> {
    let mut labels = HashMap::new();
    let lines = separated_list0(many1(newline), parse_assembly_line)(input)?.1;
    {
        let mut current_addr = Address::ZERO;
        for line in lines.iter() {
            if let Some(label) = line.label {
                if labels.contains_key(&label) {
                    panic!()
                }
                labels.insert(label, current_addr);
            }

            current_addr.0 += 1;
        }
    }

    let convert_direction = |dir: Direction| -> Address {
        dbg!(dir);
        match dir {
            Direction::Address(addr) => addr,
            Direction::Label(label) => labels[label],
        }
    };

    dbg!(&labels);

    Ok((
        input,
        lines
            .into_iter()
            .map(|line| match line.instruction {
                Instruction::Store { address } => convert_direction(address).0 & 0o777,
                Instruction::Load { address } => (1 << 9) | convert_direction(address).0 & 0o777,
                Instruction::Add { address } => (2 << 9) | convert_direction(address).0 & 0o777,
                Instruction::Branch { address } => (3 << 9) | convert_direction(address).0 & 0o777,
                Instruction::BranchIfZero { address } => {
                    (4 << 9) | convert_direction(address).0 & 0o777
                }
                Instruction::Clear => 5 << 9,
                Instruction::Decrease => 6 << 9,
                Instruction::Halt => 7 << 9,
            })
            .collect(),
    ))
}

#[cfg(test)]
#[test]
fn test() {
    let asm = include_str!("../../test.sz").to_owned();
    let result = assemble(&asm).unwrap().1;
    let lines = asm.lines();
    for (word, line) in result.iter().zip(lines) {
        println!("{:04o} {}", word, line);
    }
}
