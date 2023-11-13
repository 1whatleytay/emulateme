use std::error::Error;
use std::fmt::{Display, Formatter};
use crate::controller::Controller;
use crate::cpu::{Cpu, StatusRegister};
use crate::decoder::Decoder;
use crate::interpreter::CpuError::{Break, InvalidOp, Memory, Stop};
use crate::memory::MemoryError;

#[derive(Debug)]
pub enum CpuError {
    InvalidOp(u8),
    Memory(MemoryError),
    Break,
    Stop,
}

const STACK_START: u16 = 0x100;

impl<'a, C1: Controller, C2: Controller> Cpu<'a, C1, C2> {
    fn get_ptr(&mut self, offset: u8) -> Result<u16, MemoryError> {
        let low = self.memory.get(offset as u16)? as u16;
        let high = self.memory.get(offset.wrapping_add(1) as u16)? as u16;

        Ok((high << 8) | low)
    }

    fn get_ptr_a(&mut self, address: u16) -> Result<u16, MemoryError> {
        let second = (address & 0xFF00) | (((address & 0xFF) as u8).wrapping_add(1) as u16);

        let low = self.memory.get(address)? as u16;
        let high = self.memory.get(second)? as u16;

        Ok((high << 8) | low)
    }

    fn get_zp(&mut self, offset: u8) -> Result<u8, MemoryError> {
        self.memory.get(offset as u16)
    }

    fn get_zpo(&mut self, offset: u8, register: u8) -> Result<u8, MemoryError> {
        self.memory.cycle();

        self.memory.get(offset.wrapping_add(register) as u16)
    }

    fn get_a(&mut self, address: u16) -> Result<u8, MemoryError> {
        self.memory.get(address)
    }

    fn get_ao(&mut self, address: u16, offset: u8) -> Result<u8, MemoryError> {
        if (address as u8).checked_add(offset).is_none() {
            self.memory.cycle();
        }

        self.memory.get(address.wrapping_add(offset as u16))
    }

    fn get_di(&mut self, offset: u8, register: u8) -> Result<u8, MemoryError> {
        let pointer = self.get_ptr(offset.wrapping_add(register))?;

        self.memory.cycle();

        self.memory.get(pointer)
    }

    fn get_do(&mut self, offset: u8, register: u8) -> Result<u8, MemoryError> {
        let pointer = self.get_ptr(offset)?;

        if (pointer as u8).checked_add(register).is_none() {
            self.memory.cycle();
        }

        let pointer = pointer.wrapping_add(register as u16);

        self.memory.get(pointer)
    }

    fn set_zp(&mut self, offset: u8, value: u8) -> Result<(), MemoryError> {
        self.memory.set(offset as u16, value)
    }

    fn set_zpo(&mut self, offset: u8, register: u8, value: u8) -> Result<(), MemoryError> {
        self.memory.set(offset.wrapping_add(register) as u16, value)
    }

    fn set_a(&mut self, address: u16, value: u8) -> Result<(), MemoryError> {
        self.memory.set(address, value)
    }

    fn set_ao(&mut self, address: u16, offset: u8, value: u8) -> Result<(), MemoryError> {
        self.memory.set(address.wrapping_add(offset as u16), value)
    }

    fn set_di(&mut self, offset: u8, register: u8, value: u8) -> Result<(), MemoryError> {
        let pointer = self.get_ptr(offset.wrapping_add(register))?;

        self.memory.cycle();

        self.memory.set(pointer, value)
    }

    fn set_do(&mut self, offset: u8, register: u8, value: u8) -> Result<(), MemoryError> {
        let pointer = self.get_ptr(offset)?;

        if (pointer as u8).checked_add(register).is_none() {
            self.memory.cycle();
        }

        let pointer = pointer + (register as u16);

        self.memory.set(pointer, value)
    }

    fn push(&mut self, value: u8) -> Result<(), MemoryError> {
        self.memory.set(STACK_START + self.registers.sp as u16, value)?;

        self.registers.sp = self.registers.sp.wrapping_sub(1);

        Ok(())
    }

    fn push_address(&mut self, address: u16) -> Result<(), MemoryError> {
        self.push((address >> 8) as u8)?;
        self.push((address & 0xFF) as u8)
    }

    fn pop(&mut self) -> Result<u8, MemoryError> {
        self.registers.sp = self.registers.sp.wrapping_add(1);

        self.memory.get(STACK_START + self.registers.sp as u16)
    }

    fn pop_address(&mut self) -> Result<u16, MemoryError> {
        let low = self.pop()? as u16;
        let high = self.pop()? as u16;

        Ok((high << 8) | low)
    }

    pub fn interrupt(&mut self, pc: u16) -> Result<(), MemoryError> {
        self.push_address(self.registers.pc)?;

        let status = self.registers.p.clone()
            | StatusRegister::ENABLED
            | StatusRegister::BREAK;

        self.push(status.bits())?;

        self.registers.pc = pc;

        Ok(())
    }

    fn set_flags(&mut self, value: u8) {
        self.registers.p.set(StatusRegister::ZERO, value == 0);
        self.registers.p.set(StatusRegister::NEGATIVE, value & 0b10000000 != 0);
    }

    fn add(&mut self, a: u8, b: u8) -> u8 {
        let a_signed = a as i8;
        let b_signed = b as i8;

        let mut result = a.wrapping_add(b);

        let mut has_carry = a.checked_add(b).is_none();
        let mut has_overflow = a_signed.checked_add(b_signed).is_none();

        if self.registers.p.contains(StatusRegister::CARRY) {
            let result_signed = result as i8;

            has_carry = has_carry || result.checked_add(1).is_none();
            has_overflow = has_overflow || result_signed.checked_add(1).is_none();

            result = result.wrapping_add(1);
        }

        self.set_flags(result);
        self.registers.p.set(StatusRegister::CARRY, has_carry);
        self.registers.p.set(StatusRegister::OVERFLOW, has_overflow);

        result
    }

    fn sub(&mut self, a: u8, b: u8) -> u8 {
        self.add(a, !b)
    }

    fn cmp(&mut self, a: u8, b: u8) {
        let result = a.wrapping_sub(b);

        self.set_flags(result);
        self.registers.p.set(StatusRegister::CARRY, a >= b);
    }

    fn asl(&mut self, value: u8) -> u8 {
        let result = value.wrapping_shl(1);

        self.set_flags(result);
        self.registers.p.set(StatusRegister::CARRY, value & 0b10000000 != 0);

        self.memory.cycle();

        result
    }

    fn lsr(&mut self, value: u8) -> u8 {
        let result = value.wrapping_shr(1);

        self.set_flags(result);
        self.registers.p.set(StatusRegister::CARRY, value & 0b00000001 != 0);

        self.memory.cycle();

        result
    }

    fn rol(&mut self, value: u8) -> u8 {
        let carry = if self.registers.p.contains(StatusRegister::CARRY) {
            0b00000001u8
        } else {
            0b00000000u8
        };
        let result = value.wrapping_shl(1) | carry;

        self.set_flags(result);
        self.registers.p.set(StatusRegister::CARRY, value & 0b10000000 != 0);

        self.memory.cycle();

        result
    }

    fn ror(&mut self, value: u8) -> u8 {
        let carry = if self.registers.p.contains(StatusRegister::CARRY) {
            0b10000000u8
        } else {
            0b00000000u8
        };
        let result = value.wrapping_shr(1) | carry;

        self.set_flags(result);
        self.registers.p.set(StatusRegister::CARRY, value & 0b00000001 != 0);

        self.memory.cycle();

        result
    }

    fn branch(&mut self, rel: u8) {
        self.memory.cycle();

        let start = self.registers.pc;

        self.registers.pc = (self.registers.pc as i16).wrapping_add((rel as i8) as i16) as u16;

        if start & 0xFF00 != self.registers.pc & 0xFF00 {
            self.memory.cycle();
        }
    }
}

impl<'a, C1: Controller, C2: Controller> Decoder<Result<(), CpuError>> for Cpu<'a, C1, C2> {
    fn brk(&mut self) -> Result<(), CpuError> {
        Err(Break)
    }

    fn stp(&mut self) -> Result<(), CpuError> {
        Err(Stop)
    }

    fn nop_g(&mut self) -> Result<(), CpuError> {
        /* Do nothing. */

        self.memory.cycle();

        Ok(())
    }

    fn nop_i(&mut self, _: u8) -> Result<(), CpuError> {
        /* Do nothing. */

        Ok(())
    }

    fn nop_z(&mut self, _: u8) -> Result<(), CpuError> {
        /* Do nothing. */

        self.memory.cycle();

        Ok(())
    }

    fn nop_zx(&mut self, _: u8) -> Result<(), CpuError> {
        /* Do nothing. */

        self.memory.cycle_many(2);

        Ok(())
    }

    fn nop_a(&mut self, _: u16) -> Result<(), CpuError> {
        /* Do nothing. */

        self.memory.cycle();

        Ok(())
    }

    fn nop_ax(&mut self, address: u16) -> Result<(), CpuError> {
        /* Do nothing. */

        self.memory.cycle();

        if (address as u8).checked_add(self.registers.x).is_none() {
            self.memory.cycle();
        }

        Ok(())
    }

    fn dex(&mut self) -> Result<(), CpuError> {
        self.registers.x = self.registers.x.wrapping_sub(1);

        self.set_flags(self.registers.x);

        self.memory.cycle();

        Ok(())
    }

    fn dey(&mut self) -> Result<(), CpuError> {
        self.registers.y = self.registers.y.wrapping_sub(1);

        self.set_flags(self.registers.y);

        self.memory.cycle();

        Ok(())
    }

    fn iny(&mut self) -> Result<(), CpuError> {
        self.registers.y = self.registers.y.wrapping_add(1);

        self.set_flags(self.registers.y);

        self.memory.cycle();

        Ok(())
    }

    fn inx(&mut self) -> Result<(), CpuError> {
        self.registers.x = self.registers.x.wrapping_add(1);

        self.set_flags(self.registers.x);

        self.memory.cycle();

        Ok(())
    }

    fn inc_z(&mut self, offset: u8) -> Result<(), CpuError> {
        let address = offset as u16;
        
        let value = self.memory.get(address)?.wrapping_add(1);

        self.set_flags(value);
        self.memory.cycle();

        self.memory.set(address, value)?;

        Ok(())
    }

    fn inc_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        let address = (offset + self.registers.x) as u16;

        let value = self.memory.get(address)?.wrapping_add(1);

        self.set_flags(value);
        self.memory.cycle_many(2);

        self.memory.set(address, value)?;

        Ok(())
    }

    fn inc_a(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.memory.get(address)?.wrapping_add(1);

        self.set_flags(value);
        self.memory.cycle();

        self.memory.set(address, value)?;

        Ok(())
    }

    fn inc_ax(&mut self, address: u16) -> Result<(), CpuError> {
        let address = address.wrapping_add(self.registers.x as u16);
        
        let value = self.memory.get(address)?.wrapping_add(1);

        self.set_flags(value);
        self.memory.cycle_many(2);

        self.memory.set(address, value)?;

        Ok(())
    }

    fn dec_z(&mut self, offset: u8) -> Result<(), CpuError> {
        let address = offset as u16;

        let value = self.memory.get(address)?.wrapping_sub(1);

        self.set_flags(value);
        self.memory.cycle();

        self.memory.set(address, value)?;

        Ok(())
    }

    fn dec_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        let address = (offset + self.registers.x) as u16;

        let value = self.memory.get(address)?.wrapping_sub(1);

        self.set_flags(value);
        self.memory.cycle_many(2);

        self.memory.set(address, value)?;

        Ok(())
    }

    fn dec_a(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.memory.get(address)?.wrapping_sub(1);

        self.set_flags(value);
        self.memory.cycle();

        self.memory.set(address, value)?;

        Ok(())
    }

    fn dec_ax(&mut self, address: u16) -> Result<(), CpuError> {
        let address = address.wrapping_add(self.registers.x as u16);

        let value = self.memory.get(address)?.wrapping_sub(1);

        self.set_flags(value);
        self.memory.cycle_many(2);

        self.memory.set(address, value)?;

        Ok(())
    }

    fn php(&mut self) -> Result<(), CpuError> {
        let status = self.registers.p.clone()
            | StatusRegister::ENABLED
            | StatusRegister::BREAK;

        self.push(status.bits())?;

        self.memory.cycle();

        Ok(())
    }

    fn plp(&mut self) -> Result<(), CpuError> {
        self.registers.p = StatusRegister::from_bits_retain(self.pop()?)
            .union(StatusRegister::ENABLED)
            .difference(StatusRegister::BREAK);

        self.memory.cycle_many(2);

        Ok(())
    }

    fn pha(&mut self) -> Result<(), CpuError> {
        self.push(self.registers.a)?;

        self.memory.cycle();

        Ok(())
    }

    fn pla(&mut self) -> Result<(), CpuError> {
        self.registers.a = self.pop()?;

        self.set_flags(self.registers.a);

        self.memory.cycle_many(2);

        Ok(())
    }

    fn bit_z(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_zp(offset)?;
        let result = self.registers.a & value;

        self.registers.p.set(StatusRegister::ZERO, result == 0);
        self.registers.p.set(StatusRegister::NEGATIVE, value & 0b10000000 != 0);
        self.registers.p.set(StatusRegister::OVERFLOW, value & 0b01000000 != 0);

        Ok(())
    }

    fn bit_a(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.get_a(address)?;
        let result = self.registers.a & value;

        self.registers.p.set(StatusRegister::ZERO, result == 0);
        self.registers.p.set(StatusRegister::OVERFLOW, value & 0b01000000 != 0);
        self.registers.p.set(StatusRegister::NEGATIVE, value & 0b10000000 != 0);

        Ok(())
    }

    fn tay(&mut self) -> Result<(), CpuError> {
        self.registers.y = self.registers.a;

        self.set_flags(self.registers.y);

        self.memory.cycle();

        Ok(())
    }

    fn tya(&mut self) -> Result<(), CpuError> {
        self.registers.a = self.registers.y;

        self.set_flags(self.registers.a);

        self.memory.cycle();

        Ok(())
    }

    fn txa(&mut self) -> Result<(), CpuError> {
        self.registers.a = self.registers.x;

        self.set_flags(self.registers.a);

        self.memory.cycle();

        Ok(())
    }

    fn txs(&mut self) -> Result<(), CpuError> {
        self.registers.sp = self.registers.x;

        self.memory.cycle();

        Ok(())
    }

    fn tax(&mut self) -> Result<(), CpuError> {
        self.registers.x = self.registers.a;

        self.set_flags(self.registers.x);

        self.memory.cycle();

        Ok(())
    }

    fn tsx(&mut self) -> Result<(), CpuError> {
        self.registers.x = self.registers.sp;

        self.set_flags(self.registers.x);

        self.memory.cycle();

        Ok(())
    }

    fn clc(&mut self) -> Result<(), CpuError> {
        self.registers.p.remove(StatusRegister::CARRY);

        self.memory.cycle();

        Ok(())
    }

    fn sec(&mut self) -> Result<(), CpuError> {
        self.registers.p.insert(StatusRegister::CARRY);

        self.memory.cycle();

        Ok(())
    }

    fn cli(&mut self) -> Result<(), CpuError> {
        self.registers.p.remove(StatusRegister::INTERUPT);

        self.memory.cycle();

        Ok(())
    }

    fn sei(&mut self) -> Result<(), CpuError> {
        self.registers.p.insert(StatusRegister::INTERUPT);

        self.memory.cycle();

        Ok(())
    }

    fn clv(&mut self) -> Result<(), CpuError> {
        self.registers.p.remove(StatusRegister::OVERFLOW);

        self.memory.cycle();

        Ok(())
    }

    fn cld(&mut self) -> Result<(), CpuError> {
        self.registers.p.remove(StatusRegister::DECIMAL);

        self.memory.cycle();

        Ok(())
    }

    fn sed(&mut self) -> Result<(), CpuError> {
        self.registers.p.insert(StatusRegister::DECIMAL);

        self.memory.cycle();

        Ok(())
    }

    fn jmp_a(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.pc = address;

        Ok(())
    }

    fn jmp_ad(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.pc = self.get_ptr_a(address)?;

        Ok(())
    }

    fn jsr(&mut self, address: u16) -> Result<(), CpuError> {
        self.push_address(self.registers.pc - 1)?;

        self.registers.pc = address;

        self.memory.cycle();

        Ok(())
    }

    fn rti(&mut self) -> Result<(), CpuError> {
        self.registers.p = StatusRegister::from_bits_retain(self.pop()?)
            .union(StatusRegister::ENABLED)
            .difference(StatusRegister::BREAK);
        self.registers.pc = self.pop_address()?;

        self.memory.cycle_many(2);

        Ok(())
    }

    fn rts(&mut self) -> Result<(), CpuError> {
        self.registers.pc = self.pop_address()? + 1;

        self.memory.cycle_many(3);

        Ok(())
    }

    fn bpl(&mut self, rel: u8) -> Result<(), CpuError> {
        if !self.registers.p.contains(StatusRegister::NEGATIVE) {
            self.branch(rel)
        }

        Ok(())
    }

    fn bmi(&mut self, rel: u8) -> Result<(), CpuError> {
        if self.registers.p.contains(StatusRegister::NEGATIVE) {
            self.branch(rel)
        }

        Ok(())
    }

    fn bvc(&mut self, rel: u8) -> Result<(), CpuError> {
        if !self.registers.p.contains(StatusRegister::OVERFLOW) {
            self.branch(rel)
        }

        Ok(())
    }

    fn bvs(&mut self, rel: u8) -> Result<(), CpuError> {
        if self.registers.p.contains(StatusRegister::OVERFLOW) {
            self.branch(rel)
        }

        Ok(())
    }

    fn bcc(&mut self, rel: u8) -> Result<(), CpuError> {
        if !self.registers.p.contains(StatusRegister::CARRY) {
            self.branch(rel)
        }

        Ok(())
    }

    fn bcs(&mut self, rel: u8) -> Result<(), CpuError> {
        if self.registers.p.contains(StatusRegister::CARRY) {
            self.branch(rel)
        }

        Ok(())
    }

    fn bne(&mut self, rel: u8) -> Result<(), CpuError> {
        if !self.registers.p.contains(StatusRegister::ZERO) {
            self.branch(rel)
        }

        Ok(())
    }

    fn beq(&mut self, rel: u8) -> Result<(), CpuError> {
        if self.registers.p.contains(StatusRegister::ZERO) {
            self.branch(rel)
        }

        Ok(())
    }

    fn cpx_i(&mut self, value: u8) -> Result<(), CpuError> {
        self.cmp(self.registers.x, value);

        Ok(())
    }

    fn cpx_z(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_zp(offset)?;
        self.cmp(self.registers.x, value);

        Ok(())
    }

    fn cpx_a(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.get_a(address)?;
        self.cmp(self.registers.x, value);

        Ok(())
    }

    fn cpy_i(&mut self, value: u8) -> Result<(), CpuError> {
        self.cmp(self.registers.y, value);

        Ok(())
    }

    fn cpy_z(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_zp(offset)?;
        self.cmp(self.registers.y, value);

        Ok(())
    }

    fn cpy_a(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.get_a(address)?;
        self.cmp(self.registers.y, value);

        Ok(())
    }

    fn ldy_i(&mut self, value: u8) -> Result<(), CpuError> {
        self.registers.y = value;

        self.set_flags(self.registers.y);

        Ok(())
    }

    fn ldy_z(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.y = self.get_zp(offset)?;

        self.set_flags(self.registers.y);

        Ok(())
    }

    fn ldy_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.y = self.get_zpo(offset, self.registers.x)?;

        self.set_flags(self.registers.y);

        Ok(())
    }

    fn ldy_a(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.y = self.get_a(address)?;

        self.set_flags(self.registers.y);

        Ok(())
    }

    fn ldy_ax(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.y = self.get_ao(address, self.registers.x)?;

        self.set_flags(self.registers.y);

        Ok(())
    }

    fn ldx_i(&mut self, value: u8) -> Result<(), CpuError> {
        self.registers.x = value;

        self.set_flags(self.registers.x);

        Ok(())
    }

    fn ldx_z(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.x = self.get_zp(offset)?;

        self.set_flags(self.registers.x);

        Ok(())
    }

    fn ldx_zy(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.x = self.get_zpo(offset, self.registers.y)?;

        self.set_flags(self.registers.x);

        Ok(())
    }

    fn ldx_a(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.x = self.get_a(address)?;

        self.set_flags(self.registers.x);

        Ok(())
    }

    fn ldx_ay(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.x = self.get_ao(address, self.registers.y)?;

        self.set_flags(self.registers.x);

        Ok(())
    }

    fn ora_i(&mut self, value: u8) -> Result<(), CpuError> {
        self.registers.a |= value;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn ora_z(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a |= self.get_zp(offset)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn ora_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a |= self.get_zpo(offset, self.registers.x)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn ora_a(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.a |= self.get_a(address)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn ora_ax(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.a |= self.get_ao(address, self.registers.x)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn ora_ay(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.a |= self.get_ao(address, self.registers.y)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn ora_dx(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a |= self.get_di(offset, self.registers.x)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn ora_dy(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a |= self.get_do(offset, self.registers.y)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn and_i(&mut self, value: u8) -> Result<(), CpuError> {
        self.registers.a &= value;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn and_z(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a &= self.get_zp(offset)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn and_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a &= self.get_zpo(offset, self.registers.x)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn and_a(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.a &= self.get_a(address)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn and_ax(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.a &= self.get_ao(address, self.registers.x)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn and_ay(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.a &= self.get_ao(address, self.registers.y)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn and_dx(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a &= self.get_di(offset, self.registers.x)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn and_dy(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a &= self.get_do(offset, self.registers.y)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn eor_i(&mut self, value: u8) -> Result<(), CpuError> {
        self.registers.a ^= value;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn eor_z(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a ^= self.get_zp(offset)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn eor_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a ^= self.get_zpo(offset, self.registers.x)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn eor_a(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.a ^= self.get_a(address)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn eor_ax(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.a ^= self.get_ao(address, self.registers.x)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn eor_ay(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.a ^= self.get_ao(address, self.registers.y)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn eor_dx(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a ^= self.get_di(offset, self.registers.x)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn eor_dy(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a ^= self.get_do(offset, self.registers.y)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn adc_i(&mut self, value: u8) -> Result<(), CpuError> {
        self.registers.a = self.add(self.registers.a, value);

        Ok(())
    }

    fn adc_z(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_zp(offset)?;
        self.registers.a = self.add(self.registers.a, value);

        Ok(())
    }

    fn adc_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_zpo(offset, self.registers.x)?;
        self.registers.a = self.add(self.registers.a, value);

        Ok(())
    }

    fn adc_a(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.get_a(address)?;
        self.registers.a = self.add(self.registers.a, value);

        Ok(())
    }

    fn adc_ax(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.get_ao(address, self.registers.x)?;
        self.registers.a = self.add(self.registers.a, value);

        Ok(())
    }

    fn adc_ay(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.get_ao(address, self.registers.y)?;
        self.registers.a = self.add(self.registers.a, value);

        Ok(())
    }

    fn adc_dx(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_di(offset, self.registers.x)?;
        self.registers.a = self.add(self.registers.a, value);

        Ok(())
    }

    fn adc_dy(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_do(offset, self.registers.y)?;
        self.registers.a = self.add(self.registers.a, value);

        Ok(())
    }

    fn sta_z(&mut self, offset: u8) -> Result<(), CpuError> {
        self.set_zp(offset, self.registers.a)?;

        Ok(())
    }

    fn sta_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        self.set_zpo(offset, self.registers.x, self.registers.a)?;

        self.memory.cycle();

        Ok(())
    }

    fn sta_a(&mut self, address: u16) -> Result<(), CpuError> {
        self.set_a(address, self.registers.a)?;

        Ok(())
    }

    fn sta_ax(&mut self, address: u16) -> Result<(), CpuError> {
        self.set_ao(address, self.registers.x, self.registers.a)?;

        self.memory.cycle();

        Ok(())
    }

    fn sta_ay(&mut self, address: u16) -> Result<(), CpuError> {
        self.set_ao(address, self.registers.y, self.registers.a)?;

        self.memory.cycle();

        Ok(())
    }

    fn sta_dx(&mut self, offset: u8) -> Result<(), CpuError> {
        self.set_di(offset, self.registers.x, self.registers.a)?;

        Ok(())
    }

    fn sta_dy(&mut self, offset: u8) -> Result<(), CpuError> {
        self.set_do(offset, self.registers.y, self.registers.a)?;

        self.memory.cycle();

        Ok(())
    }

    fn stx_z(&mut self, offset: u8) -> Result<(), CpuError> {
        self.set_zp(offset, self.registers.x)?;

        Ok(())
    }

    fn stx_zy(&mut self, offset: u8) -> Result<(), CpuError> {
        self.set_zpo(offset, self.registers.y, self.registers.x)?;

        self.memory.cycle();

        Ok(())
    }

    fn stx_a(&mut self, address: u16) -> Result<(), CpuError> {
        self.set_a(address, self.registers.x)?;

        Ok(())
    }

    fn sty_z(&mut self, offset: u8) -> Result<(), CpuError> {
        self.set_zp(offset, self.registers.y)?;

        Ok(())
    }

    fn sty_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        self.set_zpo(offset, self.registers.x, self.registers.y)?;

        self.memory.cycle();

        Ok(())
    }

    fn sty_a(&mut self, address: u16) -> Result<(), CpuError> {
        self.set_a(address, self.registers.y)?;

        Ok(())
    }

    fn lda_i(&mut self, value: u8) -> Result<(), CpuError> {
        self.registers.a = value;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn lda_z(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a = self.get_zp(offset)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn lda_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a = self.get_zpo(offset, self.registers.x)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn lda_a(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.a = self.get_a(address)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn lda_ax(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.a = self.get_ao(address, self.registers.x)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn lda_ay(&mut self, address: u16) -> Result<(), CpuError> {
        self.registers.a = self.get_ao(address, self.registers.y)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn lda_dx(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a = self.get_di(offset, self.registers.x)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn lda_dy(&mut self, offset: u8) -> Result<(), CpuError> {
        self.registers.a = self.get_do(offset, self.registers.y)?;

        self.set_flags(self.registers.a);

        Ok(())
    }

    fn cmp_i(&mut self, value: u8) -> Result<(), CpuError> {
        self.cmp(self.registers.a, value);

        Ok(())
    }

    fn cmp_z(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_zp(offset)?;
        self.cmp(self.registers.a, value);

        Ok(())
    }

    fn cmp_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_zpo(offset, self.registers.x)?;
        self.cmp(self.registers.a, value);

        Ok(())
    }

    fn cmp_a(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.get_a(address)?;
        self.cmp(self.registers.a, value);

        Ok(())
    }

    fn cmp_ax(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.get_ao(address, self.registers.x)?;
        self.cmp(self.registers.a, value);

        Ok(())
    }

    fn cmp_ay(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.get_ao(address, self.registers.y)?;
        self.cmp(self.registers.a, value);

        Ok(())
    }

    fn cmp_dx(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_di(offset, self.registers.x)?;
        self.cmp(self.registers.a, value);

        Ok(())
    }

    fn cmp_dy(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_do(offset, self.registers.y)?;
        self.cmp(self.registers.a, value);

        Ok(())
    }

    fn sbc_i(&mut self, value: u8) -> Result<(), CpuError> {
        self.registers.a = self.sub(self.registers.a, value);

        Ok(())
    }

    fn sbc_z(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_zp(offset)?;
        self.registers.a = self.sub(self.registers.a, value);

        Ok(())
    }

    fn sbc_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_zpo(offset, self.registers.x)?;
        self.registers.a = self.sub(self.registers.a, value);

        Ok(())
    }

    fn sbc_a(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.get_a(address)?;
        self.registers.a = self.sub(self.registers.a, value);

        Ok(())
    }

    fn sbc_ax(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.get_ao(address, self.registers.x)?;
        self.registers.a = self.sub(self.registers.a, value);

        Ok(())
    }

    fn sbc_ay(&mut self, address: u16) -> Result<(), CpuError> {
        let value = self.get_ao(address, self.registers.y)?;
        self.registers.a = self.sub(self.registers.a, value);

        Ok(())
    }

    fn sbc_dx(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_di(offset, self.registers.x)?;
        self.registers.a = self.sub(self.registers.a, value);

        Ok(())
    }

    fn sbc_dy(&mut self, offset: u8) -> Result<(), CpuError> {
        let value = self.get_do(offset, self.registers.y)?;
        self.registers.a = self.sub(self.registers.a, value);

        Ok(())
    }

    fn asl_g(&mut self) -> Result<(), CpuError> {
        self.registers.a = self.asl(self.registers.a);

        Ok(())
    }

    fn asl_z(&mut self, offset: u8) -> Result<(), CpuError> {
        let input = self.get_zp(offset)?;
        let value = self.asl(input);

        self.set_zp(offset, value)?;

        Ok(())
    }

    fn asl_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        let input = self.get_zpo(offset, self.registers.x)?;
        let value = self.asl(input);

        self.set_zpo(offset, self.registers.x, value)?;

        Ok(())
    }

    fn asl_a(&mut self, address: u16) -> Result<(), CpuError> {
        let input = self.get_a(address)?;
        let value = self.asl(input);

        self.set_a(address, value)?;

        Ok(())
    }

    fn asl_ax(&mut self, address: u16) -> Result<(), CpuError> {
        let input = self.get_ao(address, self.registers.x)?;
        let value = self.asl(input);

        self.set_ao(address, self.registers.x, value)?;
        self.memory.cycle();

        Ok(())
    }

    fn rol_g(&mut self) -> Result<(), CpuError> {
        self.registers.a = self.rol(self.registers.a);

        Ok(())
    }

    fn rol_z(&mut self, offset: u8) -> Result<(), CpuError> {
        let input = self.get_zp(offset)?;
        let value = self.rol(input);

        self.set_zp(offset, value)?;

        Ok(())
    }

    fn rol_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        let input = self.get_zpo(offset, self.registers.x)?;
        let value = self.rol(input);

        self.set_zpo(offset, self.registers.x, value)?;

        Ok(())
    }

    fn rol_a(&mut self, address: u16) -> Result<(), CpuError> {
        let input = self.get_a(address)?;
        let value = self.rol(input);

        self.set_a(address, value)?;

        Ok(())
    }

    fn rol_ax(&mut self, address: u16) -> Result<(), CpuError> {
        let input = self.get_ao(address, self.registers.x)?;
        let value = self.rol(input);

        self.set_ao(address, self.registers.x, value)?;
        self.memory.cycle();

        Ok(())
    }

    fn ror_g(&mut self) -> Result<(), CpuError> {
        self.registers.a = self.ror(self.registers.a);

        Ok(())
    }

    fn ror_z(&mut self, offset: u8) -> Result<(), CpuError> {
        let input = self.get_zp(offset)?;
        let value = self.ror(input);

        self.set_zp(offset, value)?;

        Ok(())
    }

    fn ror_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        let input = self.get_zpo(offset, self.registers.x)?;
        let value = self.ror(input);

        self.set_zpo(offset, self.registers.x, value)?;

        Ok(())
    }

    fn ror_a(&mut self, address: u16) -> Result<(), CpuError> {
        let input = self.get_a(address)?;
        let value = self.ror(input);

        self.set_a(address, value)?;

        Ok(())
    }

    fn ror_ax(&mut self, address: u16) -> Result<(), CpuError> {
        let input = self.get_ao(address, self.registers.x)?;
        let value = self.ror(input);

        self.set_ao(address, self.registers.x, value)?;
        self.memory.cycle();

        Ok(())
    }

    fn lsr_g(&mut self) -> Result<(), CpuError> {
        self.registers.a = self.lsr(self.registers.a);

        Ok(())
    }

    fn lsr_z(&mut self, offset: u8) -> Result<(), CpuError> {
        let input = self.get_zp(offset)?;
        let value = self.lsr(input);

        self.set_zp(offset, value)?;

        Ok(())
    }

    fn lsr_zx(&mut self, offset: u8) -> Result<(), CpuError> {
        let input = self.get_zpo(offset, self.registers.x)?;
        let value = self.lsr(input);

        self.set_zpo(offset, self.registers.x, value)?;

        Ok(())
    }

    fn lsr_a(&mut self, address: u16) -> Result<(), CpuError> {
        let input = self.get_a(address)?;
        let value = self.lsr(input);

        self.set_a(address, value)?;

        Ok(())
    }

    fn lsr_ax(&mut self, address: u16) -> Result<(), CpuError> {
        let input = self.get_ao(address, self.registers.x)?;
        let value = self.lsr(input);

        self.set_ao(address, self.registers.x, value)?;
        self.memory.cycle();

        Ok(())
    }
}

impl From<MemoryError> for CpuError {
    fn from(value: MemoryError) -> Self {
        Memory(value)
    }
}

impl Display for CpuError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidOp(op) => write!(f, "Invalid OP code ${op:02X}"),
            Memory(error) => error.fmt(f),
            Break => write!(f, "Hit break instruction"),
            Stop => write!(f, "Hit stop instruction")
        }
    }
}

impl Error for CpuError { }

impl<'a, C1: Controller, C2: Controller> Cpu<'a, C1, C2> {
    pub fn step(&mut self) -> Result<(), CpuError> {
        let pc = self.registers.pc;

        let next = |cpu: &mut Cpu<C1, C2>| {
            let pc = cpu.registers.pc;

            let value = cpu.memory.get(pc);

            cpu.registers.pc += 1;

            value.ok()
        };

        let result = self.decode(next);

        result.unwrap_or_else(|| {
            match self.memory.get(pc) {
                Ok(op) => Err(InvalidOp(op)),
                Err(error) => Err(Memory(error))
            }
        })
    }
}
