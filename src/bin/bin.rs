use clap::{Parser, ValueEnum};
use std::collections::HashMap;
use std::error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Assemble or disassemble the input file
    #[clap(arg_enum, value_parser)]
    mode: Mode,
    /// Write output to this file
    #[clap(short, value_parser, value_name = "output")]
    output_file: Option<PathBuf>,
    /// Import symbols from this file
    #[clap(short, value_parser, value_name = "syms")]
    syms: Option<PathBuf>,
    /// Use this file as input
    #[clap(value_parser)]
    input_file: PathBuf,
    /// Use this address as the base address of the program
    #[clap(default_value_t = String::from("0x80000000"), short, value_parser, value_name = "base addr")]
    base_addr: String,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Mode {
    Asm,
    Disasm,
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let cli = Cli::parse();

    let syms: String = match cli.syms.as_deref() {
        Some(syms) => fs::read_to_string(syms)?.parse()?,
        None => String::new(),
    };

    let symbols: HashMap<String, u32> = HashMap::from_iter(syms.lines().map(|s| {
        let mut parts = s.split('=');
        let name = parts.next().unwrap().trim();
        let value = parts.next().unwrap();
        let value = u32::from_str_radix(value.replace("0x", "").trim(), 16).unwrap();
        (name.to_string(), value)
    }));

    let addr = cli.base_addr.replace("0x", "");
    let addr = u32::from_str_radix(&addr, 16).unwrap_or_else(|_| {
        eprintln!("Error: Invalid base address `{}`", cli.base_addr);
        std::process::exit(1);
    });

    match cli.mode {
        Mode::Asm => {
            let data: String = fs::read_to_string(cli.input_file)?.parse()?;
            let output = match mipsasm::parser::scan(&data, addr, symbols) {
                Ok(output) => output,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };

            let output = mipsasm::assembler::assemble(output);

            if let Some(output_file) = cli.output_file {
                let mut bytes = vec![];
                for word in output {
                    bytes.append(&mut word.to_be_bytes().to_vec());
                }
                File::create(output_file)?.write_all(&bytes)?;
            } else {
                println!("{:08X?}", output);
            }
        }
        Mode::Disasm => {
            let mut words = vec![];
            let mut bytes = fs::read(cli.input_file)?;
            loop {
                let mut word = [0; 4];
                word.copy_from_slice(&bytes[0..4]);
                words.push(u32::from_be_bytes(word));
                bytes.drain(0..4);
                if bytes.is_empty() {
                    break;
                }
            }
            let output = mipsasm::disassembler::disassemble(words);

            if let Some(output_file) = cli.output_file {
                let mut f = File::create(output_file)?;
                for inst in output {
                    write!(f, "{}", inst)?;
                }
            } else {
                for inst in output {
                    println!("{}", inst);
                }
            }
        }
    }
    Ok(())
}
