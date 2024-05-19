use std::fs::File;
use std::io::Write;

use crate::{Instruction, Program};
use anyhow::{Context, Result};

pub enum Operand {
    Register(u8),
    Immediate(usize),
    Immediate8(u8),
    MemoryByRegister(u8),
    MemoryByRegisterAndOffset(u8, u8),
}

#[derive(Default)]
pub struct X86Assembler {
    code: Vec<u8>,
}

impl X86Assembler {
    const RAX: u8 = 0x00;
    const RDI: u8 = 0x07;
    const RSI: u8 = 0x06;
    const RDX: u8 = 0x02;

    fn clear(&mut self) {
        self.code.clear();
    }

    fn position(&self) -> usize {
        self.code.len()
    }

    fn emit(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
    }
    fn emit_movzx(&mut self, dst: Operand, src: Operand) {
        match (dst, src) {
            (
                Operand::Register(dst_reg),
                Operand::MemoryByRegisterAndOffset(src_reg, offset_reg),
            ) => {
                // movzx dst, byte [src_reg + offset_reg*1]
                self.emit(&[
                    0x0F,
                    0xB6,
                    0x04,
                    0x00 | (dst_reg << 3) | src_reg | offset_reg,
                ]);
            }
            _ => todo!("not implemented"),
        }
    }

    fn emit_mov(&mut self, dst: Operand, src: Operand) {
        match (dst, src) {
            (Operand::Register(dst_reg), Operand::MemoryByRegister(src_reg)) => {
                // mov dst, [src]
                self.emit(&[0x48, 0x8B, 0x00 | (dst_reg << 3) | src_reg]);
            }
            (Operand::Register(dst), Operand::Immediate(src)) => {
                // mov dst, src
                self.emit(&[
                    0x48,
                    0xC7,
                    0xC0 | dst,
                    (src & 0xFF) as u8,
                    ((src >> 8) & 0xFF) as u8,
                    ((src >> 16) & 0xFF) as u8,
                    ((src >> 24) & 0xFF) as u8,
                ]);
            }
            (Operand::MemoryByRegister(dst), Operand::Register(src)) => {
                // mov [dst], src
                self.emit(&[0x48, 0x89, 0x00 | (src << 3) | dst]);
            }
            (
                Operand::Register(dst_reg),
                Operand::MemoryByRegisterAndOffset(src_reg, offset_reg),
            ) => {
                // mov dst, [src_reg + offset_reg*1]
                self.emit(&[
                    0x48,
                    0x8B,
                    0x04 | (dst_reg << 3),
                    0x00 | (offset_reg << 3) | src_reg,
                ]);
            }
            (Operand::Register(dst), Operand::Register(src)) => {
                // mov dst, src
                self.emit(&[0x48, 0x89, 0xC0 | (src << 3) | dst]);
            }
            _ => todo!("not implemented"),
        }
    }

    fn emit_add(&mut self, dst: Operand, src: Operand) {
        match (dst, src) {
            (Operand::Register(dst), Operand::Immediate8(value)) => {
                // add dst, value
                self.emit(&[0x48, 0x83, 0xC0 | dst, value]);
            }
            (Operand::Register(dst), Operand::Immediate(value)) => {
                // add dst, value
                self.emit(&[
                    0x48,
                    0x81,
                    0xC0 | dst,
                    (value & 0xFF) as u8,
                    ((value >> 8) & 0xFF) as u8,
                    ((value >> 16) & 0xFF) as u8,
                    ((value >> 24) & 0xFF) as u8,
                ]);
            }
            (
                Operand::MemoryByRegisterAndOffset(dst_reg, offset_reg),
                Operand::Immediate8(value),
            ) => {
                // add [dst_reg + offset_reg*1] byte value
                self.emit(&[0x80, 0x04, 0x00 | (dst_reg << 3) | offset_reg, value]);
            }
            (Operand::Register(dst), Operand::Register(src)) => {
                // add dst, src
                self.emit(&[0x48, 0x01, 0xC0 | (src << 3) | dst]);
            }
            _ => todo!("not implemented"),
        }
    }

    fn emit_sub(&mut self, dst: Operand, src: Operand) {
        match (dst, src) {
            (Operand::Register(dst), Operand::Immediate8(value)) => {
                // sub [dst] byte value
                self.emit(&[0x48, 0x83, 0xE8 | dst, value]);
            }
            (Operand::Register(dst), Operand::Immediate(value)) => {
                // sub dst, value
                self.emit(&[
                    0x48,
                    0x81,
                    0xE8 | dst,
                    (value & 0xFF) as u8,
                    ((value >> 8) & 0xFF) as u8,
                    ((value >> 16) & 0xFF) as u8,
                    ((value >> 24) & 0xFF) as u8,
                ]);
            }
            (
                Operand::MemoryByRegisterAndOffset(dst_reg, offset_reg),
                Operand::Immediate8(value),
            ) => {
                // sub byte [dst_reg + offset_reg], value
                self.emit(&[0x80, 0x2C, 0x00 | (dst_reg << 3) | offset_reg, value]);
            }
            _ => todo!("not implemented"),
        }
    }

    fn emit_push(&mut self, src: Operand) {
        match src {
            Operand::Register(src) => {
                // push src
                self.emit(&[0x50 | src]);
            }
            _ => todo!("not implemented"),
        }
    }

    fn emit_pop(&mut self, dst: Operand) {
        match dst {
            Operand::Register(dst) => {
                // pop dst
                self.emit(&[0x58 | dst]);
            }
            _ => todo!("not implemented"),
        }
    }

    fn emit_compare(&mut self, dst: Operand, src: Operand) {
        match (dst, src) {
            (Operand::Register(dst), Operand::Immediate8(value)) => {
                // cmp dst, byte value
                self.emit(&[0x48, 0x83, 0xF8 | dst, value]);
            }
            _ => todo!("not implemented"),
        }
    }

    fn emit_jump_if_zero(&mut self, target: usize) {
        // The source will be the point AFTER this instruction as it is based on
        // the RIP after the instruction has been read.
        let src_pos = (self.position() + 6) as i32;
        let relative_target = target as i32 - src_pos;

        // je relative_target
        self.emit(&[0x0F, 0x84]);
        self.emit(&relative_target.to_le_bytes());
    }

    fn emit_jump_if_non_zero(&mut self, target: usize) {
        // The source will be the point AFTER this instruction as it is based on
        // the RIP after the instruction has been read.
        let src_pos = (self.position() + 6) as i32;
        let relative_target = target as i32 - src_pos;

        // jne relative_target
        self.emit(&[0x0F, 0x85]);
        self.emit(&relative_target.to_le_bytes());
    }

    fn patch_jump_target(&mut self, patch_target_pos: usize, new_target: usize) {
        let relative_target = new_target as i32 - patch_target_pos as i32;
        self.code[patch_target_pos - 4..patch_target_pos]
            .copy_from_slice(&relative_target.to_le_bytes());
    }

    fn emit_syscall(&mut self) {
        self.emit(&[0x0F, 0x05]);
    }

    fn emit_return(&mut self) {
        self.emit(&[0xC3])
    }
}

pub struct JitCompiler {
    assembler: X86Assembler,
    program: Program,
    memory: Vec<u8>,
    addr: usize,
}

impl JitCompiler {
    pub fn new(program: Program, assembler: X86Assembler) -> Self {
        Self {
            assembler,
            program,
            memory: vec![0; 640000],
            addr: 0,
        }
    }

    pub fn compile(&mut self) -> Result<()> {
        let mut forward_jumps = vec![];

        self.assembler.clear();
        // RDI will be the pointer to the memory array
        // RSI will be the offset into the memory array
        for i in 0..self.program.len() {
            use Operand::*;
            match self.program[i] {
                Instruction::AddrRight(value) => {
                    self.assembler.emit_mov(
                        Register(X86Assembler::RAX),
                        MemoryByRegister(X86Assembler::RSI),
                    );
                    self.assembler
                        .emit_add(Register(X86Assembler::RAX), Immediate(value));
                    self.assembler.emit_mov(
                        MemoryByRegister(X86Assembler::RSI),
                        Register(X86Assembler::RAX),
                    );
                }
                Instruction::AddrLeft(value) => {
                    self.assembler.emit_mov(
                        Register(X86Assembler::RAX),
                        MemoryByRegister(X86Assembler::RSI),
                    );
                    self.assembler
                        .emit_sub(Register(X86Assembler::RAX), Immediate(value));
                    self.assembler.emit_mov(
                        MemoryByRegister(X86Assembler::RSI),
                        Register(X86Assembler::RAX),
                    );
                }
                Instruction::Inc(value) => {
                    self.assembler.emit_mov(
                        Register(X86Assembler::RAX),
                        MemoryByRegister(X86Assembler::RSI),
                    );
                    self.assembler.emit_add(
                        MemoryByRegisterAndOffset(X86Assembler::RDI, X86Assembler::RAX),
                        Immediate8(value),
                    );
                }
                Instruction::Dec(value) => {
                    self.assembler.emit_mov(
                        Register(X86Assembler::RAX),
                        MemoryByRegister(X86Assembler::RSI),
                    );
                    self.assembler.emit_sub(
                        MemoryByRegisterAndOffset(X86Assembler::RDI, X86Assembler::RAX),
                        Immediate8(value),
                    );
                }
                Instruction::Output(value) => {
                    for _ in 0..value {
                        self.assembler.emit_push(Register(X86Assembler::RDI));
                        self.assembler.emit_push(Register(X86Assembler::RSI));

                        // Load offset into "memory" into RAX
                        self.assembler.emit_mov(
                            Register(X86Assembler::RAX),
                            MemoryByRegister(X86Assembler::RSI),
                        );

                        // Add RDI to RAX to get the memory location
                        self.assembler
                            .emit_add(Register(X86Assembler::RAX), Register(X86Assembler::RDI));

                        // Put the memory location into RSI
                        self.assembler
                            .emit_mov(Register(X86Assembler::RSI), Register(X86Assembler::RAX));

                        // Load syscall number into RAX (write)
                        self.assembler
                            .emit_mov(Register(X86Assembler::RAX), Immediate(1));
                        // Load stdout file descriptor into RDI
                        self.assembler
                            .emit_mov(Register(X86Assembler::RDI), Immediate(1));
                        // Load size into RDX
                        self.assembler
                            .emit_mov(Register(X86Assembler::RDX), Immediate(1));
                        // Perform syscall
                        self.assembler.emit_syscall();

                        self.assembler.emit_pop(Register(X86Assembler::RSI));
                        self.assembler.emit_pop(Register(X86Assembler::RDI));
                    }
                }
                Instruction::Input(_) => todo!(),
                Instruction::JmpForward(_) => {
                    self.assembler.emit_mov(
                        Register(X86Assembler::RAX),
                        MemoryByRegister(X86Assembler::RSI),
                    );
                    self.assembler.emit_movzx(
                        Register(X86Assembler::RAX),
                        MemoryByRegisterAndOffset(X86Assembler::RDI, X86Assembler::RAX),
                    );
                    self.assembler
                        .emit_compare(Register(X86Assembler::RAX), Immediate8(0));
                    //                     // Backpatch the jump target once we know it.
                    self.assembler.emit_jump_if_zero(0x00c0ffee);
                    forward_jumps.push(self.assembler.position());
                }
                Instruction::JmpBack(_) => {
                    self.assembler.emit_mov(
                        Register(X86Assembler::RAX),
                        MemoryByRegister(X86Assembler::RSI),
                    );
                    self.assembler.emit_movzx(
                        Register(X86Assembler::RAX),
                        MemoryByRegisterAndOffset(X86Assembler::RDI, X86Assembler::RAX),
                    );
                    self.assembler
                        .emit_compare(Register(X86Assembler::RAX), Immediate8(0));
                    let target = forward_jumps.pop().expect("expected forward jump target");
                    self.assembler.emit_jump_if_non_zero(target);

                    // Backpatch the forward jump target
                    let patch_target = self.assembler.position();
                    self.assembler.patch_jump_target(target, patch_target);
                }
            }
        }
        self.assembler.emit_return();

        // write the code to a file
        let mut file = File::create("output.bin").context("creating output file")?;
        file.write_all(&self.assembler.code)
            .context("writing output file")?;
        drop(file);

        Ok(())
    }

    pub fn run(&mut self) {
        let jit_fn = memory_map_executable_code(&self.assembler.code).unwrap();
        jit_fn(self.memory.as_ptr(), &mut self.addr as *mut usize);
    }
}

fn memory_map_executable_code(code: &Vec<u8>) -> Result<extern "C" fn(*const u8, *mut usize)> {
    let func: extern "C" fn(*const u8, *mut usize) = unsafe {
        // 1. mmap to map read/write anonymous memory of size code
        let ptr = libc::mmap(
            0 as *mut libc::c_void,
            code.len(),
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_ANON | libc::MAP_PRIVATE,
            -1,
            0,
        );
        if ptr == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error())
                .context("memory mapping region for executable code");
        }
        // 2. copy code to memory
        std::ptr::copy_nonoverlapping(code.as_ptr(), ptr as *mut u8, code.len());
        // 3. mprotect the memory to read/exec
        let result = libc::mprotect(ptr, code.len(), libc::PROT_EXEC | libc::PROT_READ);
        if result == -1 {
            return Err(std::io::Error::last_os_error())
                .context("making memory mapped region executable");
        }
        // 4. reinterpret_cast memory pointer to function signature
        std::mem::transmute(ptr)
    };

    Ok(func)
}
