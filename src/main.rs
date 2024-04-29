use anyhow::{anyhow, Context, Result};
use std::env;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::process;

struct Lexer<R: Read> {
    source: R,
    location: Location,
    peeked_token: Option<Token>,
}

#[derive(Debug, Copy, Clone)]
struct Location {
    line: usize,
    column: usize,
}

impl Default for Location {
    fn default() -> Self {
        Self { line: 1, column: 1 }
    }
}

#[derive(Debug, Clone, Copy)]
struct Token {
    char: char,
    location: Location,
}

#[derive(Debug, Clone)]
enum Instruction {
    AddrRight(usize),
    AddrLeft(usize),
    Inc(u8),
    Dec(u8),
    Output(usize),
    Input(usize),
    JmpForward(usize),
    JmpBack(usize),
}

impl<R> Lexer<R>
where
    R: Read,
{
    fn new(source: R) -> Self {
        Self {
            source,
            location: Location::default(),
            peeked_token: None,
        }
    }

    fn is_char_in_language(candidate: char) -> bool {
        let lang_chars = "<>+-.,[]";
        for char in lang_chars.chars() {
            if char == candidate {
                return true;
            }
        }

        false
    }

    fn chop(&mut self) -> Result<Option<Token>> {
        if self.peeked_token.is_some() {
            let token = self
                .peeked_token
                .take()
                .expect("peeked token to be available");
            return Ok(Some(token));
        }

        let mut buf: [u8; 1] = [0; 1];
        let mut location = self.location;
        while !Self::is_char_in_language(buf[0].into()) {
            location = self.location;
            let read_bytes = self
                .source
                .read(&mut buf)
                .context("read next byte from source")?;
            if read_bytes != 1 {
                return Ok(None);
            }
            self.location.column += 1;
            if buf[0] == b'\n' {
                self.location.column = 1;
                self.location.line += 1;
            }
        }

        Ok(Some(Token {
            char: buf[0].into(),
            location,
        }))
    }

    fn peek(&mut self) -> Result<Option<Token>> {
        if let Some(token) = self.peeked_token {
            return Ok(Some(token));
        }

        self.peeked_token = self.chop().context("reading next token to peek at it")?;
        Ok(self.peeked_token)
    }

    fn chop_while(&mut self, token: &Token) -> Result<usize> {
        let mut count: usize = 0;
        while let Some(candidate) = self.peek()? {
            if candidate.char == token.char {
                self.chop()?;
                count += 1;
            } else {
                break;
            }
        }

        Ok(count)
    }
}

type Program = Vec<Instruction>;

#[derive(Default)]
struct Parser {
    forward_jumps: Vec<usize>,
    program: Program,
}

impl Parser {
    fn parse_instruction<R: Read>(
        &mut self,
        lexer: &mut Lexer<R>,
        token: &Token,
    ) -> Result<Instruction> {
        match token {
            Token { char: '<', .. } => Ok(Instruction::AddrLeft(1 + lexer.chop_while(token)?)),
            Token { char: '>', .. } => Ok(Instruction::AddrRight(1 + lexer.chop_while(token)?)),
            Token { char: '+', .. } => Ok(Instruction::Inc(
                ((1 + lexer.chop_while(token)?) % 255) as u8,
            )),
            Token { char: '-', .. } => Ok(Instruction::Dec(
                ((1 + lexer.chop_while(token)?) % 255) as u8,
            )),
            Token { char: '.', .. } => Ok(Instruction::Output(1 + lexer.chop_while(token)?)),
            Token { char: ',', .. } => Ok(Instruction::Input(1 + lexer.chop_while(token)?)),
            Token { char: '[', .. } => {
                self.forward_jumps.push(self.program.len());
                // Position will be backpatched once encountering corresponding
                // JmpBack
                Ok(Instruction::JmpForward(0))
            }
            Token {
                char: ']',
                location: Location { line, column },
            } => {
                if let Some(target) = self.forward_jumps.pop() {
                    self.program[target] = Instruction::JmpForward(self.program.len() + 1);
                    Ok(Instruction::JmpBack(target + 1))
                } else {
                    Err(anyhow!(
                        "Could not find corresponding forward jump for ] at {line}:{column}"
                    ))
                }
            }
            _ => unreachable!("No other token than the defined set is expected."),
        }
    }

    fn parse_program<R: Read>(&mut self, lexer: &mut Lexer<R>) -> Result<Program> {
        self.program = vec![];
        self.forward_jumps = vec![];
        while let Some(token) = lexer.chop()? {
            let instruction = self.parse_instruction(lexer, &token)?;
            self.program.push(instruction);
        }
        Ok(self.program.clone())
    }
}
struct Intepreter {
    program: Program,
    memory: Vec<u8>,
    addr: usize,
    instruction_ptr: usize,
}

impl Intepreter {
    fn new(program: Program) -> Self {
        Self {
            program,
            // @TODO: allocate dynamically
            memory: vec![0; 640000],
            addr: 0,
            instruction_ptr: 0,
        }
    }

    fn run(&mut self) -> Result<()> {
        while self.instruction_ptr < self.program.len() {
            match self.program[self.instruction_ptr] {
                Instruction::AddrRight(count) => {
                    self.addr += count;
                    self.instruction_ptr += 1;
                }
                Instruction::AddrLeft(count) => {
                    self.addr -= count;
                    self.instruction_ptr += 1;
                }
                Instruction::Inc(count) => {
                    self.memory[self.addr] = self.memory[self.addr].wrapping_add(count);
                    self.instruction_ptr += 1;
                }
                Instruction::Dec(count) => {
                    self.memory[self.addr] = self.memory[self.addr].wrapping_sub(count);
                    self.instruction_ptr += 1;
                }
                Instruction::Output(count) => {
                    let mut stdout = std::io::stdout();
                    for _ in 0..count {
                        stdout
                            .write(&self.memory[self.addr..self.addr + 1])
                            .context("writing data to stdout")?;
                    }
                    stdout.flush().context("flush stdout")?;
                    self.instruction_ptr += 1;
                }
                Instruction::Input(_) => todo!(),
                Instruction::JmpForward(target) => {
                    if self.memory[self.addr] == 0 {
                        self.instruction_ptr = target;
                    } else {
                        self.instruction_ptr += 1;
                    }
                }
                Instruction::JmpBack(target) => {
                    if self.memory[self.addr] != 0 {
                        self.instruction_ptr = target;
                    } else {
                        self.instruction_ptr += 1;
                    }
                }
            }
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<String>>();
    let (command, args) = args
        .split_first()
        .expect("expected to have at least the command in the args array");
    if args.is_empty() {
        eprintln!("Usage:");
        eprintln!("  {command} <brainfuck_file>");
        process::exit(1);
    }

    let input = &args[0];
    println!("Opening brainfuck file {input} for execution");
    let reader = BufReader::new(
        File::open(input).with_context(|| format!("open file {input} for reading"))?,
    );
    let mut lexer = Lexer::new(reader);
    let mut parser = Parser::default();
    let program = parser.parse_program(&mut lexer)?;
    let mut intepreter = Intepreter::new(program);
    intepreter.run()?;

    Ok(())
}
