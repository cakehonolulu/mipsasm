use crate::ast;
use regex::Regex;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("label `{0}` defined multiple times")]
    MultipleLabelDefinition(String),
    #[error("invalid instruction `{0}`")]
    InvalidInstruction(String),
    #[error("invalid number of operands `{line}`\n Expected {expected} operands, found {found}")]
    InvalidOperandCount {
        line: String,
        expected: usize,
        found: usize,
    },
    #[error("invalid opcode `{0}`")]
    InvalidOpcode(String),
    #[error("invalid register `{0}`")]
    InvalidRegister(String),
    #[error("invalid target address `{0}`")]
    InvalidTargetAddress(String),
    #[error("invalid immediate `{0}`")]
    InvalidImmediate(String),
    #[error("invalid coprocessor `{0}`")]
    InvalidCopNumber(String),
    #[error("invalid coprocessor sub-opcode `{0}`")]
    InvalidCopSubOpcode(String),
    #[error("invalid float compare condition `{0}`")]
    InvalidFloatCond(String),
}

pub fn scan(
    input: &str,
    base_addr: u32,
    syms: Option<HashMap<String, u32>>,
) -> Result<Vec<ast::Instruction>, ParserError> {
    let mut parser = Parser::new(input, base_addr, syms.unwrap_or_default());
    parser.scan()?;
    parser.adjust_labels();
    Ok(parser.insts)
}

struct Parser<'a> {
    input: &'a str,
    insts: Vec<ast::Instruction>,
    labels: HashMap<&'a str, isize>,
    base_addr: u32,
    syms: HashMap<String, u32>,
}

impl<'a> Parser<'a> {
    fn new(input: &str, base_addr: u32, syms: HashMap<String, u32>) -> Parser {
        Parser {
            input,
            insts: vec![],
            labels: HashMap::new(),
            base_addr,
            syms,
        }
    }

    fn scan(&mut self) -> Result<(), ParserError> {
        for line in self.input.lines() {
            self.scan_line(line)?;
        }
        Ok(())
    }

    fn scan_line(&mut self, line: &'a str) -> Result<(), ParserError> {
        if line.ends_with(':') {
            self.labels
                .insert(self.parse_label(line)?, self.insts.len() as isize);
        } else if !line.is_empty() {
            self.insts.push(self.parse_inst(line)?);
        }

        Ok(())
    }

    fn parse_label(&self, label: &'a str) -> Result<&'a str, ParserError> {
        if self.labels.contains_key(&label) {
            return Err(ParserError::MultipleLabelDefinition(label.to_string()));
        }
        Ok(label.trim_end_matches(':'))
    }

    fn parse_inst(&self, inst: &'a str) -> Result<ast::Instruction, ParserError> {
        let mut line = inst.split_whitespace();
        let op = match line.next() {
            Some(x) => x,
            None => return Err(ParserError::InvalidInstruction(inst.to_string())),
        };
        let args = line.collect::<String>();
        let args = args.split(',').collect::<Vec<&str>>();

        let offset_regex = Regex::new(r".+\s*\(").unwrap();
        let base_regex = Regex::new(r"\(.*?\)").unwrap();

        match op.to_lowercase().trim() {
            // -----------------------------------------------------------------
            // |    op     |  base   |   rt    |             offset            |
            // ------6----------5---------5-------------------16----------------
            //  Format:  op rt, offset(base)
            "cache" | "lb" | "lbu" | "ld" | "ldl" | "ldr" | "lh" | "lhu" | "ll" | "lld" | "lw"
            | "lwl" | "lwr" | "lwu" | "sb" | "sc" | "scd" | "sd" | "sdl" | "sdr" | "sh" | "sw"
            | "swl" | "swr" => {
                if args.len() != 2 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 2,
                        found: args.len(),
                    });
                }
                let rt = if op.to_lowercase().trim() == "cache" {
                    ast::Register::try_from(
                        self.parse_immediate::<u16>(
                            args.first()
                                .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?,
                        )?
                        .as_u32(),
                    )
                    .unwrap()
                } else {
                    args.first()
                        .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                        .parse()
                        .unwrap()
                };
                let x = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?;
                let base = base_regex
                    .find_iter(x)
                    .last()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .as_str()
                    .replace(&['(', ')'][..], "")
                    .trim()
                    .parse()
                    .unwrap();
                if let Some(x) = offset_regex.find(x) {
                    Ok(ast::Instruction::Immediate {
                        op: op
                            .parse()
                            .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                        rs: base,
                        rt,
                        imm: self.parse_immediate::<i16>(&x.as_str()[..x.as_str().len() - 1])?,
                    })
                } else {
                    Ok(ast::Instruction::Immediate {
                        op: op
                            .parse()
                            .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                        rs: base,
                        rt,
                        imm: self.parse_immediate::<i16>("0")?,
                    })
                }
            }
            // -----------------------------------------------------------------
            // |    op     |   rs    |   rt    |          immediate            |
            // ------6----------5---------5-------------------16----------------
            //  Format:  op rt, rs, immediate
            "addi" | "addiu" | "andi" | "daddi" | "daddiu" | "ori" | "slti" | "sltiu" | "xori"
            | "dsubi" | "dsubiu" | "subi" | "subiu" => {
                if args.len() != 3 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 3,
                        found: args.len(),
                    });
                }
                let rt = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let rs = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let imm = args
                    .get(2)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?;
                if op == "andi" || op == "ori" || op == "xori" {
                    Ok(ast::Instruction::Immediate {
                        op: op
                            .parse()
                            .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                        rt,
                        rs,
                        imm: self.parse_immediate::<u16>(imm)?,
                    })
                } else {
                    Ok(ast::Instruction::Immediate {
                        op: op
                            .parse()
                            .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                        rt,
                        rs,
                        imm: self.parse_immediate::<i16>(imm)?,
                    })
                }
            }
            // -----------------------------------------------------------------
            // |    op     |  00000  |   rt    |           immediate           |
            // ------6----------5---------5-------------------16----------------
            //  Format:  op rt, immediate
            "lui" | "lli" => {
                if args.len() != 2 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 2,
                        found: args.len(),
                    });
                }
                let rt = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let imm = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?;
                Ok(ast::Instruction::Immediate {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rt,
                    rs: ast::Register::null(),
                    imm: self.parse_immediate::<u16>(imm)?,
                })
            }
            // -----------------------------------------------------------------
            // |    op     |   rs    |  00000  |            offset             |
            // ------6----------5---------5-------------------16----------------
            //  Format:  op rs, offset
            "bgez" | "bgezal" | "bgezall" | "bgezl" | "bltz" | "bltzal" | "bltzall" | "bltzl"
            | "teqi" | "tgei" | "tgeiu" | "tlti" | "tltiu" | "tnei" | "beqz" | "bnez" | "beqzl"
            | "bnezl" | "bgtz" | "bgtzl" | "blez" | "blezl" => {
                if args.len() != 2 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 2,
                        found: args.len(),
                    });
                }
                let rs = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let imm = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?;
                Ok(ast::Instruction::Immediate {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rt: ast::Register::null(),
                    rs,
                    imm: self.parse_immediate::<i16>(imm)?,
                })
            }
            // -----------------------------------------------------------------
            // |    op     |   rs    |   rt    |            offset             |
            // ------6----------5---------5-------------------16----------------
            //  Format:  op rs, rt, offset
            "beq" | "beql" | "bne" | "bnel" | "bge" | "bgt" | "ble" | "blt" | "bgeu" | "bgtu"
            | "bleu" | "bltu" | "bgel" | "bgtl" | "blel" | "bltl" | "bgeul" | "bgtul" | "bleul"
            | "bltul" => {
                if args.len() != 3 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 3,
                        found: args.len(),
                    });
                }
                let rs = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let rt = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let imm = args
                    .get(2)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?;
                Ok(ast::Instruction::Immediate {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rt,
                    rs,
                    imm: self.parse_immediate::<i16>(imm)?,
                })
            }
            // -----------------------------------------------------------------
            // |    op     |                       target                      |
            // ------6-------------------------------26-------------------------
            //  Format:  op target
            "j" | "jal" => {
                if args.len() != 1 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                let target = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .trim();
                Ok(ast::Instruction::Jump {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    target: self.parse_target(target)?,
                })
            }
            // -----------------------------------------------------------------
            // |  SPECIAL  |      0000 0000 0000 000     |  stype  |    op     |
            // ------6-------------------15-------------------5---------6-------
            //  Format:  op          (stype = 0 implied)
            "nop" | "sync" => Ok(ast::Instruction::Register {
                op: op
                    .parse()
                    .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                rd: ast::Register::null(),
                rs: ast::Register::null(),
                rt: ast::Register::null(),
                sa: 0,
            }),
            // -----------------------------------------------------------------
            // |  SPECIAL  |   rs    |   rt    |   rd    |  00000  |    op     |
            // ------6----------5---------5---------5---------5----------6------
            //  Format:  op rd, rs, rt
            "add" | "addu" | "and" | "dadd" | "daddu" | "dsub" | "dsubu" | "nor" | "or" | "slt"
            | "sltu" | "sub" | "subu" | "xor" | "dmul" | "dmulu" | "dmulo" | "dmulou" | "drem"
            | "dremu" | "drol" | "dror" | "mul" | "mulu" | "mulo" | "mulou" | "rem" | "remu"
            | "seq" | "sge" | "sgeu" | "sgt" | "sgtu" | "sle" | "sleu" | "sne" => {
                if args.len() != 3 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 3,
                        found: args.len(),
                    });
                }
                let rd = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let rs = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let rt = args
                    .get(2)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                Ok(ast::Instruction::Register {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rd,
                    rs,
                    rt,
                    sa: 0,
                })
            }
            // -----------------------------------------------------------------
            // |  SPECIAL  |  00000  |   rt    |    rd   |   sa    |    op     |
            // ------6----------5---------5---------5---------5----------6------
            //  Format:  op rd, rt, sa
            "dsll" | "dsll32" | "dsra" | "dsra32" | "dsrl" | "dsrl32" | "sll" | "sra" | "srl" => {
                if args.len() != 3 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 3,
                        found: args.len(),
                    });
                }
                let rd = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let rt = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let sa = args
                    .get(2)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .trim();
                Ok(ast::Instruction::Register {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rd,
                    rs: ast::Register::null(),
                    rt,
                    sa: if sa.ends_with('`') || !sa.contains("0x") {
                        sa.trim_end_matches('`').parse::<i32>().unwrap() as u32
                    } else {
                        let sa = sa.replace("0x", "");
                        i32::from_str_radix(&sa, 16).unwrap() as u32
                    },
                })
            }
            // -----------------------------------------------------------------
            // |  SPECIAL  |   rs    |   rt    |    rd   |  00000  |    op     |
            // ------6----------5---------5---------5---------5----------6------
            //  Format:  op rd, rt, rs
            "dsllv" | "dsrav" | "dsrlv" | "sllv" | "srav" | "srlv" => {
                if args.len() != 3 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 3,
                        found: args.len(),
                    });
                }
                let rd = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let rt = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let rs = args
                    .get(2)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                Ok(ast::Instruction::Register {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rd,
                    rs,
                    rt,
                    sa: 0,
                })
            }
            // -----------------------------------------------------------------
            // |  SPECIAL  |                   code                |    op     |
            // ------6--------------------------20-----------------------6------
            //  Format:  op offset
            "break" | "syscall" => {
                if args.len() != 1 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                let code = if args.first().unwrap().is_empty() {
                    ast::Immediate::Short(0)
                } else if !args.first().unwrap().is_empty() {
                    self.parse_immediate::<u16>(
                        args.first()
                            .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                            .trim(),
                    )?
                } else {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                };

                Ok(ast::Instruction::Register {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rd: ast::Register::null(),
                    rs: ast::Register::null(),
                    rt: ast::Register::null(),
                    sa: code.as_u32(),
                })
            }
            // -----------------------------------------------------------------
            // |  SPECIAL  |   rs    |   rt    |   0000 0000 00    |    op     |
            // ------6----------5---------5--------------10--------------6------
            //  Format:  op rs, rt
            "dmult" | "dmultu" | "mult" | "multu" | "teq" | "tge" | "tgeu" | "tlt" | "tltu"
            | "tne" => {
                if args.len() != 2 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 2,
                        found: args.len(),
                    });
                }
                let rs = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let rt = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                Ok(ast::Instruction::Register {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rd: ast::Register::null(),
                    rs,
                    rt,
                    sa: 0,
                })
            }
            // -----------------------------------------------------------------
            // |  SPECIAL  |   rs    |  00000  |   rd    |  00000  |    op     |
            // ------6----------5---------5---------5---------5----------6------
            //  Format:  op rd, rs
            "jalr" => {
                if args.len() != 2 && args.len() != 1 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 2,
                        found: args.len(),
                    });
                }
                let rs = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                if args.len() == 1 {
                    Ok(ast::Instruction::Register {
                        op: op
                            .parse()
                            .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                        rd: ast::Register::Ra,
                        rs,
                        rt: ast::Register::null(),
                        sa: 0,
                    })
                } else {
                    let rd = args
                        .get(1)
                        .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                        .parse()
                        .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                    Ok(ast::Instruction::Register {
                        op: op
                            .parse()
                            .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                        rd: rs,
                        rs: rd,
                        rt: ast::Register::null(),
                        sa: 0,
                    })
                }
            }
            "abs" | "dabs" | "dmove" | "dneg" | "dnegu" | "move" | "neg" | "negu" | "not" => {
                if args.len() != 2 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 2,
                        found: args.len(),
                    });
                }
                let rd = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let rs = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                Ok(ast::Instruction::Register {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rd,
                    rs,
                    rt: ast::Register::null(),
                    sa: 0,
                })
            }
            "b" | "bal" => {
                if args.len() != 1 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }

                let imm = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?;
                Ok(ast::Instruction::Immediate {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rs: ast::Register::null(),
                    rt: ast::Register::null(),
                    imm: self.parse_immediate::<i16>(imm)?,
                })
            }
            "li" => {
                if args.len() != 2 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 2,
                        found: args.len(),
                    });
                }
                let rt = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let imm = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?;
                Ok(ast::Instruction::Immediate {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rs: ast::Register::null(),
                    rt,
                    imm: self.parse_immediate::<i32>(imm)?,
                })
            }
            "ddiv" | "ddivu" | "div" | "divu" => {
                if args.len() != 3 && args.len() != 2 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 3,
                        found: args.len(),
                    });
                }
                let rd = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let rs = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                if args.len() == 2 {
                    Ok(ast::Instruction::Register {
                        op: op
                            .parse()
                            .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                        rd,
                        rs,
                        rt: ast::Register::null(),
                        sa: 0,
                    })
                } else {
                    let rt = args
                        .get(2)
                        .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                        .parse()
                        .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                    Ok(ast::Instruction::Register {
                        op: op
                            .parse()
                            .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                        rd,
                        rs,
                        rt,
                        sa: 0,
                    })
                }
            }
            // -----------------------------------------------------------------
            // |  SPECIAL  |   rs    |     0000 0000 0000 000      |    op     |
            // ------6----------5------------------15--------------------6------
            //  Format:  op rs
            "jr" | "mthi" | "mtlo" => {
                if args.len() != 1 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                let rs = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                Ok(ast::Instruction::Register {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rd: ast::Register::null(),
                    rs,
                    rt: ast::Register::null(),
                    sa: 0,
                })
            }
            // -----------------------------------------------------------------
            // |  SPECIAL  |   0000 0000 00    |   rd    |  00000  |    op     |
            // ------6---------------10-------------5---------5----------6------
            //  Format:  op rd
            "clear" | "mfhi" | "mflo" => {
                if args.len() != 1 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                let rd = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                Ok(ast::Instruction::Register {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rd,
                    rs: ast::Register::null(),
                    rt: ast::Register::null(),
                    sa: 0,
                })
            }
            "dli" => {
                if args.len() != 2 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 2,
                        found: args.len(),
                    });
                }
                let rt = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let imm = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?;
                Ok(ast::Instruction::Immediate {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rs: ast::Register::null(),
                    rt,
                    imm: self.parse_immediate::<i64>(imm)?,
                })
            }
            // -----------------------------------------------------------------
            // |   COPz    |   op    |    bc    |           offset             |
            // ------6----------5----------5------------------16----------------
            //  Format:  op offset
            "bc0f" | "bc1f" | "bc0fl" | "bc1fl" | "bc0t" | "bc1t" | "bc0tl" | "bc1tl" => {
                if args.len() != 1 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                let offset = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?;
                Ok(ast::Instruction::Immediate {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rs: ast::Register::null(),
                    rt: ast::Register::null(),
                    imm: self.parse_immediate::<i16>(offset)?,
                })
            }
            // -----------------------------------------------------------------
            // |   COPz    |   op    |   rt    |   rd    |    0000 0000 000    |
            // ------6----------5---------5---------5--------------11-----------
            //  Format:  op rt, rd
            "cfc0" | "ctc0" | "dmfc0" | "dmtc0" | "mfc0" | "mtc0" => {
                if args.len() != 2 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 2,
                        found: args.len(),
                    });
                }
                let rt = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let rd = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse::<ast::Cop0Register>()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                Ok(ast::Instruction::Register {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rs: ast::Register::null(),
                    rt,
                    rd: rd.into(),
                    sa: 0,
                })
            }
            // -----------------------------------------------------------------
            // |   COPz    |   op    |   rt    |   fs    |    0000 0000 000    |
            // ------6----------5---------5---------5--------------11-----------
            //  Format:  op rt, fs
            "cfc1" | "ctc1" | "dmfc1" | "dmtc1" | "mfc1" | "mtc1" => {
                if args.len() != 2 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 2,
                        found: args.len(),
                    });
                }
                let rt = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let rd = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse::<ast::FloatRegister>()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                Ok(ast::Instruction::Register {
                    op: op
                        .parse()
                        .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                    rs: ast::Register::null(),
                    rt,
                    rd: rd.into(),
                    sa: 0,
                })
            }
            // -----------------------------------------------------------------
            // |   COPz    |CO|      0000 0000 0000 0000 000       |    op     |
            // ------6------1-------------------19-----------------------6------
            //  Format:  op
            "eret" | "tlbp" | "tlbr" | "tlbwi" | "tlbwr" => Ok(ast::Instruction::Register {
                op: op
                    .parse()
                    .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                rs: ast::Register::null(),
                rt: ast::Register::null(),
                rd: ast::Register::null(),
                sa: 0,
            }),
            // -----------------------------------------------------------------
            // |    op     |   base  |   ft    |            offset             |
            // ------6----------5---------5-------------------16----------------
            //  Format:  op ft, offset(base)
            "ldc1" | "lwc1" | "sdc1" | "swc1" => {
                if args.len() != 2 {
                    return Err(ParserError::InvalidOperandCount {
                        line: inst.to_string(),
                        expected: 2,
                        found: args.len(),
                    });
                }
                let ft = args
                    .first()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .parse::<ast::FloatRegister>()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                let x = args
                    .get(1)
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?;
                let base = base_regex
                    .find_iter(x)
                    .last()
                    .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                    .as_str()
                    .replace(&['(', ')'][..], "")
                    .trim()
                    .parse()
                    .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                if let Some(x) = offset_regex.find(x) {
                    Ok(ast::Instruction::Immediate {
                        op: op
                            .parse()
                            .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                        rs: base,
                        rt: ast::Register::from(ft),
                        imm: self.parse_immediate::<i16>(&x.as_str().replace('(', ""))?,
                    })
                } else {
                    Ok(ast::Instruction::Immediate {
                        op: op
                            .parse()
                            .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                        rs: base,
                        rt: ast::Register::from(ft),
                        imm: self.parse_immediate::<i16>("0")?,
                    })
                }
            }
            _ => match &op.to_lowercase()[..op.len() - 2] {
                // -----------------------------------------------------------------
                // |   COP1    |   fmt   |   ft    |   fs    |   fd    |    op     |
                // ------6----------5---------5---------5---------5----------6------
                //  Format:  op.fmt fd, fs, ft
                "add" | "sub" | "mul" | "div" => {
                    if args.len() != 3 {
                        return Err(ParserError::InvalidOperandCount {
                            line: inst.to_string(),
                            expected: 3,
                            found: args.len(),
                        });
                    }
                    let fd = args
                        .first()
                        .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                        .parse::<ast::FloatRegister>()
                        .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                    let fs = args
                        .get(1)
                        .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                        .parse::<ast::FloatRegister>()
                        .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                    let ft = args
                        .get(2)
                        .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                        .parse::<ast::FloatRegister>()
                        .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                    Ok(ast::Instruction::Register {
                        op: op
                            .parse()
                            .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                        rs: ast::Register::from(fs),
                        rt: ast::Register::from(ft),
                        rd: ast::Register::from(fd),
                        sa: 0,
                    })
                }
                // -----------------------------------------------------------------
                // |   COP1    |   fmt   |  00000  |   fs    |   fd    |    op     |
                // ------6----------5---------5---------5---------5----------6------
                //  Format:  op.fmt fd, fs
                "abs" | "ceil.l" | "ceil.w" | "cvt.d" | "cvt.l" | "cvt.s" | "cvt.w" | "floor.l"
                | "floor.w" | "mov" | "neg" | "round.l" | "round.w" | "sqrt" | "trunc.l"
                | "trunc.w" => {
                    if args.len() != 2 {
                        return Err(ParserError::InvalidOperandCount {
                            line: inst.to_string(),
                            expected: 2,
                            found: args.len(),
                        });
                    }
                    let fd = args
                        .first()
                        .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                        .parse::<ast::FloatRegister>()
                        .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                    let fs = args
                        .get(1)
                        .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                        .parse::<ast::FloatRegister>()
                        .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                    Ok(ast::Instruction::Register {
                        op: op
                            .parse()
                            .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                        rs: ast::Register::from(fs),
                        rt: ast::Register::null(),
                        rd: ast::Register::from(fd),
                        sa: 0,
                    })
                }
                e => {
                    // -----------------------------------------------------------------
                    // |   COP1    |   fmt   |   ft    |   fs    | 000 |00 |11 | cond  |
                    // ------6----------5---------5---------5-------3----2---2-----4----
                    //  Format:  C.cond.fmt fs, ft
                    if e.starts_with("c.") {
                        if args.len() != 2 {
                            return Err(ParserError::InvalidOperandCount {
                                line: inst.to_string(),
                                expected: 2,
                                found: args.len(),
                            });
                        }
                        let fs = args
                            .first()
                            .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                            .parse::<ast::FloatRegister>()
                            .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                        let ft = args
                            .get(1)
                            .ok_or_else(|| ParserError::InvalidInstruction(inst.to_string()))?
                            .parse::<ast::FloatRegister>()
                            .map_err(|_| ParserError::InvalidRegister(inst.to_string()))?;
                        return Ok(ast::Instruction::Register {
                            op: format!("C.{}", op.chars().last().unwrap())
                                .parse()
                                .map_err(|_| ParserError::InvalidOpcode(inst.to_string()))?,
                            rs: ast::Register::from(fs),
                            rt: ast::Register::from(ft),
                            rd: ast::Register::null(),
                            sa: parse_float_cond(
                                op.split('.').collect::<Vec<&str>>().get(1).unwrap(),
                            )?,
                        });
                    }
                    Err(ParserError::InvalidInstruction(inst.to_string()))
                }
            },
        }
    }

    fn adjust_labels(&mut self) {
        for i in 0..self.insts.len() {
            if let ast::Instruction::Immediate {
                op,
                rs,
                rt,
                imm: ast::Immediate::Label(lbl),
            } = &self.insts[i]
            {
                let lbl_addr = self.labels.get(lbl.as_str()).unwrap();
                self.insts[i] = ast::Instruction::Immediate {
                    op: *op,
                    rs: *rs,
                    rt: *rt,
                    imm: ast::Immediate::Short((*lbl_addr - (i + 1) as isize) as u16),
                };
            } else if let ast::Instruction::Jump {
                op,
                target: ast::Target::Label(lbl),
            } = &self.insts[i]
            {
                let lbl_addr = self.labels.get(lbl.as_str()).unwrap();
                self.insts[i] = ast::Instruction::Jump {
                    op: *op,
                    target: ast::Target::Address(self.base_addr + *lbl_addr as u32 * 4),
                };
            }
        }
    }

    fn parse_immediate<T>(&self, imm: &str) -> Result<ast::Immediate, ParserError>
    where
        T: num::PrimInt + std::str::FromStr,
    {
        let imm = imm.trim();

        if self.labels.contains_key(imm) {
            return Ok(ast::Immediate::Label(imm.to_string()));
        }

        let imm_regex = Regex::new(r"\(.*\)").unwrap();
        if let Some(x) = imm_regex.find(imm) {
            let x = self.parse_target(&x.as_str().replace(&['(', ')'][..], ""))?;
            match &imm[..3] {
                "%hi" => {
                    return Ok(ast::Immediate::new(
                        ((x.as_u32() + (x.as_u32() & 0x8000) * 2) >> 16) as u16,
                    ))
                }
                "%lo" => return Ok(ast::Immediate::new((x.as_u32() & 0xffff) as u16)),
                _ => todo!(),
            }
        }

        if imm.contains("0x") {
            let imm = imm.replace("0x", "");
            Ok(ast::Immediate::new::<T>(
                T::from_str_radix(&imm, 16)
                    .map_err(|_| ParserError::InvalidImmediate(imm.to_string()))?,
            ))
        } else {
            Ok(ast::Immediate::new(imm.parse::<T>().map_err(|_| {
                ParserError::InvalidImmediate(imm.to_string())
            })?))
        }
    }

    fn parse_target(&self, target: &str) -> Result<ast::Target, ParserError> {
        if let Some(x) = self.syms.get(target) {
            return Ok(ast::Target::Address(*x));
        }
        if target.starts_with("~Func:") {
            Ok(ast::Target::Function(target.replace("~Func:", "")))
        } else if target.starts_with('.') {
            Ok(ast::Target::Label(target.to_string()))
        } else if target.ends_with('`') {
            match target.trim_end_matches('`').parse::<u32>() {
                Ok(addr) => Ok(ast::Target::Address(addr)),
                Err(_) => Err(ParserError::InvalidTargetAddress(target.to_string())),
            }
        } else {
            let addr = target.replace("0x", "");
            match u32::from_str_radix(&addr, 16) {
                Ok(addr) => Ok(ast::Target::Address(addr)),
                Err(_) => Err(ParserError::InvalidTargetAddress(target.to_string())),
            }
        }
    }
}

fn parse_float_cond(cond: &str) -> Result<u32, ParserError> {
    match cond.to_lowercase().as_str() {
        "f" => Ok(0),
        "un" => Ok(1),
        "eq" => Ok(2),
        "ueq" => Ok(3),
        "olt" => Ok(4),
        "ult" => Ok(5),
        "ole" => Ok(6),
        "ule" => Ok(7),
        "sf" => Ok(8),
        "ngle" => Ok(9),
        "seq" => Ok(10),
        "ngl" => Ok(11),
        "lt" => Ok(12),
        "nge" => Ok(13),
        "le" => Ok(14),
        "ngt" => Ok(15),
        _ => Err(ParserError::InvalidFloatCond(cond.to_string())),
    }
}
