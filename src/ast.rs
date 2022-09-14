use std::convert::{From, TryFrom};
use std::fmt;
use std::str::FromStr;
use strum_macros::{Display, EnumString};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegParseError {
    #[error("invalid register `{0}`")]
    RegParseError(String),
}

#[derive(Debug)]
pub enum Target {
    Function(String),
    Label(String),
    Address(u32),
}

impl Target {
    pub fn as_u32(&self) -> u32 {
        match self {
            Target::Function(name) => {
                panic!("{}", name)
            }
            Target::Label(name) => {
                panic!("{}", name)
            }
            Target::Address(addr) => *addr,
        }
    }
}

#[derive(Debug)]
pub enum Immediate {
    Int(u16),
    Label(String),
}

impl Immediate {
    pub fn as_u32(&self) -> u32 {
        match self {
            Immediate::Int(i) => *i as u32,
            Immediate::Label(lbl) => panic!("{}", lbl),
        }
    }
}

struct Signed(u16);

impl fmt::LowerHex for Signed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = self.0 as i16;
        let p = if f.alternate() { "0x" } else { "" };
        let x = format!("{}{:x}", p, val.abs());
        f.pad_integral(val >= 0, "", &x)
    }
}

#[derive(Debug)]
pub enum Instruction {
    Immediate {
        op: ITypeOp,
        rs: Register,
        rt: Register,
        imm: Immediate,
    },
    Jump {
        op: JTypeOp,
        target: Target,
    },
    Register {
        op: RTypeOp,
        rs: Register,
        rt: Register,
        rd: Register,
        sa: u16,
    },
}

type I = ITypeOp;
type R = RTypeOp;

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Instruction::Immediate {
                op,
                rs,
                rt,
                imm: Immediate::Int(imm),
            } => match op {
                I::Cache
                | I::Lb
                | I::Lbu
                | I::Ld
                | I::Ldl
                | I::Ldr
                | I::Lh
                | I::Lhu
                | I::Ll
                | I::Lld
                | I::Lw
                | I::Lwl
                | I::Lwr
                | I::Lwu
                | I::Sb
                | I::Sc
                | I::Scd
                | I::Sd
                | I::Sdl
                | I::Sdr
                | I::Sh
                | I::Sw
                | I::Swl
                | I::Swr => {
                    write!(f, "{}\t    {}, {:#x}({})", op, rt, Signed(*imm), rs)
                }
                I::Addi | I::Addiu | I::Daddi | I::Daddiu | I::Slti | I::Sltiu => {
                    write!(f, "{}\t    {}, {}, {:#x}", op, rt, rs, Signed(*imm))
                }
                I::Andi | I::Ori | I::Xori => write!(f, "{}\t    {}, {}, {:#x}", op, rt, rs, imm),
                I::Lui => write!(f, "{}\t    {}, {:#x}", op, rt, imm),
                I::Beqz | I::Bgtz | I::Bgtzl | I::Blez | I::Blezl | I::Bnez => {
                    write!(f, "{}\t    {}, {:#x}", op, rs, Signed(*imm))
                }
                I::Beq | I::Beql | I::Bne | I::Bnel => {
                    write!(f, "{}\t    {}, {}, {:#x}", op, rs, rt, Signed(*imm))
                }
                I::Bgez
                | I::Bgezal
                | I::Bgezall
                | I::Bgezl
                | I::Bltz
                | I::Bltzal
                | I::Bltzall
                | I::Bltzl
                | I::Teqi
                | I::Tgei
                | I::Tgeiu
                | I::Tlti
                | I::Tltiu
                | I::Tnei => {
                    write!(f, "{}\t    {}, {:#x}", op, rs, Signed(*imm))
                }
                I::Bc0f
                | I::Bc1f
                | I::Bc0fl
                | I::Bc1fl
                | I::Bc0t
                | I::Bc1t
                | I::Bc0tl
                | I::Bc1tl => {
                    write!(f, "{}\t    {:#x}", op, Signed(*imm))
                }
                I::Ldc1 | I::Lwc1 | I::Sdc1 | I::Swc1 => {
                    write!(
                        f,
                        "{}\t    {}, {:#x}({})",
                        op,
                        FloatRegister::from(*rt),
                        Signed(*imm),
                        rs
                    )
                }
            },
            Instruction::Jump {
                op,
                target: Target::Address(target),
            } => {
                write!(f, "{}\t    {:#X?}", op, target)
            }
            Instruction::Register { op, rs, rt, rd, sa } => match op {
                R::Sync => write!(f, "{}", op),
                R::Add
                | R::Addu
                | R::And
                | R::Dadd
                | R::Daddu
                | R::Dsub
                | R::Dsubu
                | R::Nor
                | R::Or
                | R::Slt
                | R::Sltu
                | R::Sub
                | R::Subu
                | R::Xor => {
                    write!(f, "{}\t    {}, {}, {}", op, rd, rs, rt)
                }
                R::Dsll
                | R::Dsll32
                | R::Dsra
                | R::Dsra32
                | R::Dsrl
                | R::Dsrl32
                | R::Sll
                | R::Sra
                | R::Srl => {
                    write!(f, "{}\t    {}, {}, {:#x?}", op, rd, rt, sa)
                }
                R::Dsllv | R::Dsrav | R::Dsrlv | R::Sllv | R::Srav | R::Srlv => {
                    write!(f, "{}\t    {}, {}, {}", op, rd, rt, rs)
                }
                R::Break | R::Syscall => {
                    write!(f, "{}", op)
                }
                R::Ddiv
                | R::Ddivu
                | R::Div
                | R::Divu
                | R::Dmult
                | R::Dmultu
                | R::Mult
                | R::Multu
                | R::Teq
                | R::Tge
                | R::Tgeu
                | R::Tlt
                | R::Tltu
                | R::Tne => {
                    write!(f, "{}\t    {}, {}", op, rs, rt)
                }
                R::Jalr => {
                    if let &Register::Ra = rd {
                        write!(f, "{}\t    {}", op, rs)
                    } else {
                        write!(f, "{}\t    {}, {}", op, rd, rs)
                    }
                }
                R::Jr | R::Mthi | R::Mtlo => {
                    write!(f, "{}\t    {}", op, rs)
                }
                R::Mfhi | R::Mflo => {
                    write!(f, "{}\t    {}", op, rd)
                }
                R::Dmfc0 | R::Dmtc0 | R::Mfc0 | R::Mtc0 => {
                    write!(f, "{}\t    {}, {}", op, rt, rd)
                }
                R::Cfc1 | R::Ctc1 | R::Dmfc1 | R::Dmtc1 | R::Mfc1 | R::Mtc1 => {
                    write!(f, "{}\t    {}, {}", op, rt, FloatRegister::from(*rd))
                }
                R::Eret | R::Tlbp | R::Tlbr | R::Tlbwi | R::Tlbwr => {
                    write!(f, "{}", op)
                }
                R::AddS | R::AddD | R::SubS | R::SubD | R::MulS | R::MulD | R::DivS | R::DivD => {
                    let x = op.to_string().replace('_', ".");
                    write!(
                        f,
                        "{}\t    {}, {}, {}",
                        x,
                        FloatRegister::from(*rd),
                        FloatRegister::from(*rs),
                        FloatRegister::from(*rt)
                    )
                }
                R::AbsS
                | R::AbsD
                | R::CvtDS
                | R::CvtDW
                | R::CvtDL
                | R::CvtLS
                | R::CvtLD
                | R::CvtSD
                | R::CvtSW
                | R::CvtSL
                | R::CvtWD
                | R::CvtWS
                | R::MovS
                | R::MovD
                | R::NegS
                | R::NegD
                | R::SqrtS
                | R::SqrtD => {
                    let x = op.to_string().replace('_', ".");
                    write!(
                        f,
                        "{}\t    {}, {}",
                        x,
                        FloatRegister::from(*rd),
                        FloatRegister::from(*rs)
                    )
                }
                R::CeilLS | R::CeilLD | R::CeilWS | R::CeilWD => {
                    let x = op.to_string().replace('_', ".");
                    write!(
                        f,
                        "{}    {}, {}",
                        x,
                        FloatRegister::from(*rd),
                        FloatRegister::from(*rs)
                    )
                }
                R::FloorLS
                | R::FloorLD
                | R::FloorWS
                | R::FloorWD
                | R::RoundLS
                | R::RoundLD
                | R::RoundWS
                | R::RoundWD
                | R::TruncLS
                | R::TruncLD
                | R::TruncWS
                | R::TruncWD => {
                    let x = op.to_string().replace('_', ".");
                    write!(
                        f,
                        "{}   {}, {}",
                        x,
                        FloatRegister::from(*rd),
                        FloatRegister::from(*rs)
                    )
                }
                R::Cs => {
                    if *sa == 9 {
                        write!(
                            f,
                            "c.{}.s    {}, {}",
                            FloatCond::try_from(*sa).unwrap(),
                            FloatRegister::from(*rs),
                            FloatRegister::from(*rt)
                        )
                    } else {
                        write!(
                            f,
                            "c.{}.s\t    {}, {}",
                            FloatCond::try_from(*sa).unwrap(),
                            FloatRegister::from(*rs),
                            FloatRegister::from(*rt)
                        )
                    }
                }
                R::Cd => {
                    if *sa == 9 {
                        write!(
                            f,
                            "c.{}.d    {}, {}",
                            FloatCond::try_from(*sa).unwrap(),
                            FloatRegister::from(*rs),
                            FloatRegister::from(*rt)
                        )
                    } else {
                        write!(
                            f,
                            "c.{}.d\t    {}, {}",
                            FloatCond::try_from(*sa).unwrap(),
                            FloatRegister::from(*rs),
                            FloatRegister::from(*rt)
                        )
                    }
                }
            },
            e => panic!("Invalid instruction: {:?}", e),
        }
    }
}

#[derive(Clone, Copy, Debug, Display)]
#[strum(serialize_all = "snake_case")]
pub enum Register {
    Zero,
    At,
    V0,
    V1,
    A0,
    A1,
    A2,
    A3,
    T0,
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7,
    S0,
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    T8,
    T9,
    K0,
    K1,
    Gp,
    Sp,
    Fp,
    Ra,
}

impl Register {
    pub fn null() -> Self {
        Register::Zero
    }

    pub fn as_num(&self) -> u32 {
        *self as u32
    }
}

impl TryFrom<u32> for Register {
    type Error = RegParseError;

    fn try_from(reg: u32) -> Result<Self, Self::Error> {
        match reg {
            0 => Ok(Register::Zero),
            1 => Ok(Register::At),
            2 => Ok(Register::V0),
            3 => Ok(Register::V1),
            4 => Ok(Register::A0),
            5 => Ok(Register::A1),
            6 => Ok(Register::A2),
            7 => Ok(Register::A3),
            8 => Ok(Register::T0),
            9 => Ok(Register::T1),
            10 => Ok(Register::T2),
            11 => Ok(Register::T3),
            12 => Ok(Register::T4),
            13 => Ok(Register::T5),
            14 => Ok(Register::T6),
            15 => Ok(Register::T7),
            16 => Ok(Register::S0),
            17 => Ok(Register::S1),
            18 => Ok(Register::S2),
            19 => Ok(Register::S3),
            20 => Ok(Register::S4),
            21 => Ok(Register::S5),
            22 => Ok(Register::S6),
            23 => Ok(Register::S7),
            24 => Ok(Register::T8),
            25 => Ok(Register::T9),
            26 => Ok(Register::K0),
            27 => Ok(Register::K1),
            28 => Ok(Register::Gp),
            29 => Ok(Register::Sp),
            30 => Ok(Register::Fp),
            31 => Ok(Register::Ra),
            e => Err(RegParseError::RegParseError(e.to_string())),
        }
    }
}

impl FromStr for Register {
    type Err = RegParseError;

    fn from_str(reg: &str) -> Result<Self, Self::Err> {
        match reg.trim_start_matches('$').to_lowercase().as_str() {
            "zero" | "r0" => Ok(Register::Zero),
            "at" => Ok(Register::At),
            "v0" => Ok(Register::V0),
            "v1" => Ok(Register::V1),
            "a0" => Ok(Register::A0),
            "a1" => Ok(Register::A1),
            "a2" => Ok(Register::A2),
            "a3" => Ok(Register::A3),
            "t0" => Ok(Register::T0),
            "t1" => Ok(Register::T1),
            "t2" => Ok(Register::T2),
            "t3" => Ok(Register::T3),
            "t4" => Ok(Register::T4),
            "t5" => Ok(Register::T5),
            "t6" => Ok(Register::T6),
            "t7" => Ok(Register::T7),
            "s0" => Ok(Register::S0),
            "s1" => Ok(Register::S1),
            "s2" => Ok(Register::S2),
            "s3" => Ok(Register::S3),
            "s4" => Ok(Register::S4),
            "s5" => Ok(Register::S5),
            "s6" => Ok(Register::S6),
            "s7" => Ok(Register::S7),
            "t8" => Ok(Register::T8),
            "t9" => Ok(Register::T9),
            "k0" => Ok(Register::K0),
            "k1" => Ok(Register::K1),
            "gp" => Ok(Register::Gp),
            "sp" => Ok(Register::Sp),
            "fp" => Ok(Register::Fp),
            "ra" => Ok(Register::Ra),
            e => Err(RegParseError::RegParseError(e.to_string())),
        }
    }
}

impl From<FloatRegister> for Register {
    fn from(reg: FloatRegister) -> Self {
        Register::try_from(reg as u32).unwrap()
    }
}

#[derive(Clone, Copy, Debug, Display)]
#[strum(serialize_all = "snake_case")]
pub enum FloatRegister {
    Fv0,
    Fv0f,
    Fv1,
    Fv1f,
    Ft0,
    Ft0f,
    Ft1,
    Ft1f,
    Ft2,
    Ft2f,
    Ft3,
    Ft3f,
    Fa0,
    Fa0f,
    Fa1,
    Fa1f,
    Ft4,
    Ft4f,
    Ft5,
    Ft5f,
    Fs0,
    Fs0f,
    Fs1,
    Fs1f,
    Fs2,
    Fs2f,
    Fs3,
    Fs3f,
    Fs4,
    Fs4f,
    Fs5,
    Fs5f,
}

impl TryFrom<u32> for FloatRegister {
    type Error = RegParseError;

    fn try_from(reg: u32) -> Result<Self, Self::Error> {
        match reg {
            0 => Ok(FloatRegister::Fv0),
            1 => Ok(FloatRegister::Fv0f),
            2 => Ok(FloatRegister::Fv1),
            3 => Ok(FloatRegister::Fv1f),
            4 => Ok(FloatRegister::Ft0),
            5 => Ok(FloatRegister::Ft0f),
            6 => Ok(FloatRegister::Ft1),
            7 => Ok(FloatRegister::Ft1f),
            8 => Ok(FloatRegister::Ft2),
            9 => Ok(FloatRegister::Ft2f),
            10 => Ok(FloatRegister::Ft3),
            11 => Ok(FloatRegister::Ft3f),
            12 => Ok(FloatRegister::Fa0),
            13 => Ok(FloatRegister::Fa0f),
            14 => Ok(FloatRegister::Fa1),
            15 => Ok(FloatRegister::Fa1f),
            16 => Ok(FloatRegister::Ft4),
            17 => Ok(FloatRegister::Ft4f),
            18 => Ok(FloatRegister::Ft5),
            19 => Ok(FloatRegister::Ft5f),
            20 => Ok(FloatRegister::Fs0),
            21 => Ok(FloatRegister::Fs0f),
            22 => Ok(FloatRegister::Fs1),
            23 => Ok(FloatRegister::Fs1f),
            24 => Ok(FloatRegister::Fs2),
            25 => Ok(FloatRegister::Fs2f),
            26 => Ok(FloatRegister::Fs3),
            27 => Ok(FloatRegister::Fs3f),
            28 => Ok(FloatRegister::Fs4),
            29 => Ok(FloatRegister::Fs4f),
            30 => Ok(FloatRegister::Fs5),
            31 => Ok(FloatRegister::Fs5f),
            e => Err(RegParseError::RegParseError(e.to_string())),
        }
    }
}

impl FromStr for FloatRegister {
    type Err = RegParseError;

    fn from_str(reg: &str) -> Result<Self, Self::Err> {
        match reg.trim_start_matches('$').to_lowercase().as_str() {
            "f0" | "fv0" => Ok(FloatRegister::Fv0),
            "f1" | "fv0f" => Ok(FloatRegister::Fv0f),
            "f2" | "fv1" => Ok(FloatRegister::Fv1),
            "f3" | "fv1f" => Ok(FloatRegister::Fv1f),
            "f4" | "ft0" => Ok(FloatRegister::Ft0),
            "f5" | "ft0f" => Ok(FloatRegister::Ft0f),
            "f6" | "ft1" => Ok(FloatRegister::Ft1),
            "f7" | "ft1f" => Ok(FloatRegister::Ft1f),
            "f8" | "ft2" => Ok(FloatRegister::Ft2),
            "f9" | "ft2f" => Ok(FloatRegister::Ft2f),
            "f10" | "ft3" => Ok(FloatRegister::Ft3),
            "f11" | "ft3f" => Ok(FloatRegister::Ft3f),
            "f12" | "fa0" => Ok(FloatRegister::Fa0),
            "f13" | "fa0f" => Ok(FloatRegister::Fa0f),
            "f14" | "fa1" => Ok(FloatRegister::Fa1),
            "f15" | "fa1f" => Ok(FloatRegister::Fa1f),
            "f16" | "ft4" => Ok(FloatRegister::Ft4),
            "f17" | "ft4f" => Ok(FloatRegister::Ft4f),
            "f18" | "ft5" => Ok(FloatRegister::Ft5),
            "f19" | "ft5f" => Ok(FloatRegister::Ft5f),
            "f20" | "fs0" => Ok(FloatRegister::Fs0),
            "f21" | "fs0f" => Ok(FloatRegister::Fs0f),
            "f22" | "fs1" => Ok(FloatRegister::Fs1),
            "f23" | "fs1f" => Ok(FloatRegister::Fs1f),
            "f24" | "fs2" => Ok(FloatRegister::Fs2),
            "f25" | "fs2f" => Ok(FloatRegister::Fs2f),
            "f26" | "fs3" => Ok(FloatRegister::Fs3),
            "f27" | "fs3f" => Ok(FloatRegister::Fs3f),
            "f28" | "fs4" => Ok(FloatRegister::Fs4),
            "f29" | "fs4f" => Ok(FloatRegister::Fs4f),
            "f30" | "fs5" => Ok(FloatRegister::Fs5),
            "f31" | "fs5f" => Ok(FloatRegister::Fs5f),
            e => Err(RegParseError::RegParseError(e.to_string())),
        }
    }
}

impl From<Register> for FloatRegister {
    fn from(reg: Register) -> Self {
        FloatRegister::try_from(reg as u32).unwrap()
    }
}

#[derive(Clone, Copy, Debug, Display, EnumString)]
#[strum(ascii_case_insensitive)]
#[strum(serialize_all = "snake_case")]
pub enum ITypeOp {
    Addi,
    Addiu,
    Andi,
    Bc0f,
    Bc0fl,
    Bc0t,
    Bc0tl,
    Bc1f,
    Bc1fl,
    Bc1t,
    Bc1tl,
    Beq,
    Beql,
    Beqz,
    Bgez,
    Bgezal,
    Bgezall,
    Bgezl,
    Bgtz,
    Bgtzl,
    Blez,
    Blezl,
    Bltz,
    Bltzal,
    Bltzall,
    Bltzl,
    Bne,
    Bnez,
    Bnel,
    Cache,
    Daddi,
    Daddiu,
    Lb,
    Lbu,
    Ld,
    Ldc1,
    Ldl,
    Ldr,
    Lh,
    Lhu,
    Ll,
    Lld,
    Lui,
    Lw,
    Lwc1,
    Lwl,
    Lwr,
    Lwu,
    Ori,
    Sb,
    Sc,
    Scd,
    Sd,
    Sdc1,
    Sdl,
    Sdr,
    Sh,
    Slti,
    Sltiu,
    Sw,
    Swc1,
    Swl,
    Swr,
    Teqi,
    Tgei,
    Tgeiu,
    Tlti,
    Tltiu,
    Tnei,
    Xori,
}

#[derive(Clone, Copy, Debug, Display, EnumString)]
#[strum(ascii_case_insensitive)]
#[strum(serialize_all = "snake_case")]
pub enum JTypeOp {
    J,
    Jal,
}

#[derive(Clone, Copy, Debug, Display, EnumString)]
#[strum(ascii_case_insensitive)]
#[strum(serialize_all = "snake_case")]
pub enum RTypeOp {
    AbsS,
    AbsD,
    Add,
    Addu,
    AddS,
    AddD,
    And,
    Break,
    Cs,
    Cd,
    #[strum(to_string = "ceil.l.s")]
    CeilLS,
    #[strum(to_string = "ceil.l.d")]
    CeilLD,
    #[strum(to_string = "ceil.w.s")]
    CeilWS,
    #[strum(to_string = "ceil.w.d")]
    CeilWD,
    Cfc1,
    Ctc1,
    #[strum(to_string = "cvt.d.s")]
    CvtDS,
    #[strum(to_string = "cvt.d.w")]
    CvtDW,
    #[strum(to_string = "cvt.d.l")]
    CvtDL,
    #[strum(to_string = "cvt.l.s")]
    CvtLS,
    #[strum(to_string = "cvt.l.d")]
    CvtLD,
    #[strum(to_string = "cvt.s.d")]
    CvtSD,
    #[strum(to_string = "cvt.s.w")]
    CvtSW,
    #[strum(to_string = "cvt.s.l")]
    CvtSL,
    #[strum(to_string = "cvt.w.s")]
    CvtWS,
    #[strum(to_string = "cvt.w.d")]
    CvtWD,
    Dadd,
    Daddu,
    Ddiv,
    Ddivu,
    Div,
    Divu,
    DivS,
    DivD,
    Dmfc0,
    Dmfc1,
    Dmtc0,
    Dmtc1,
    Dmult,
    Dmultu,
    Dsll,
    Dsll32,
    Dsllv,
    Dsra,
    Dsra32,
    Dsrav,
    Dsrl,
    Dsrl32,
    Dsrlv,
    Dsub,
    Dsubu,
    Eret,
    #[strum(to_string = "floor.l.s")]
    FloorLS,
    #[strum(to_string = "floor.l.d")]
    FloorLD,
    #[strum(to_string = "floor.w.s")]
    FloorWS,
    #[strum(to_string = "floor.w.d")]
    FloorWD,
    Jalr,
    Jr,
    Mfc0,
    Mfc1,
    Mfhi,
    Mflo,
    MovS,
    MovD,
    Mtc0,
    Mtc1,
    Mthi,
    Mtlo,
    MulS,
    MulD,
    Mult,
    Multu,
    NegS,
    NegD,
    Nor,
    Or,
    #[strum(to_string = "round.l.s")]
    RoundLS,
    #[strum(to_string = "round.l.d")]
    RoundLD,
    #[strum(to_string = "round.w.s")]
    RoundWS,
    #[strum(to_string = "round.w.d")]
    RoundWD,
    Sll,
    Sllv,
    Slt,
    Sltu,
    SqrtS,
    SqrtD,
    Sra,
    Srav,
    Srl,
    Srlv,
    Sub,
    Subu,
    SubS,
    SubD,
    Sync,
    Syscall,
    Teq,
    Tge,
    Tgeu,
    Tlbp,
    Tlbr,
    Tlbwi,
    Tlbwr,
    Tlt,
    Tltu,
    Tne,
    #[strum(to_string = "trunc.l.s")]
    TruncLS,
    #[strum(to_string = "trunc.l.d")]
    TruncLD,
    #[strum(to_string = "trunc.w.s")]
    TruncWS,
    #[strum(to_string = "trunc.w.d")]
    TruncWD,
    Xor,
}

#[derive(Clone, Copy, Debug, Display, EnumString)]
#[strum(ascii_case_insensitive)]
#[strum(serialize_all = "snake_case")]
pub enum FloatCond {
    F,
    Un,
    Eq,
    Ueq,
    Olt,
    Ult,
    Ole,
    Ule,
    Sf,
    Ngle,
    Seq,
    Ngl,
    Lt,
    Nge,
    Le,
    Ngt,
}

impl TryFrom<u16> for FloatCond {
    type Error = RegParseError;

    fn try_from(cond: u16) -> Result<Self, Self::Error> {
        match cond {
            0 => Ok(FloatCond::F),
            1 => Ok(FloatCond::Un),
            2 => Ok(FloatCond::Eq),
            3 => Ok(FloatCond::Ueq),
            4 => Ok(FloatCond::Olt),
            5 => Ok(FloatCond::Ult),
            6 => Ok(FloatCond::Ole),
            7 => Ok(FloatCond::Ule),
            8 => Ok(FloatCond::Sf),
            9 => Ok(FloatCond::Ngle),
            10 => Ok(FloatCond::Seq),
            11 => Ok(FloatCond::Ngl),
            12 => Ok(FloatCond::Lt),
            13 => Ok(FloatCond::Nge),
            14 => Ok(FloatCond::Le),
            15 => Ok(FloatCond::Ngt),
            e => Err(RegParseError::RegParseError(e.to_string())),
        }
    }
}
