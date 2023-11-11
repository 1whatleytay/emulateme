use crate::decoder::Decoder;

pub struct Disassembler {
    pub pc: u16
}

fn compute_target(rel: u8, pc: u16) -> u16 {
    let rel = (rel as i8) as i16;
    let pc = pc as i16;

    pc.wrapping_add(rel).wrapping_add(2) as u16
}

fn format_i(instruction: &str, value: u8) -> String {
    format!("{instruction} #${value:02X}")
}

fn format_z(instruction: &str, offset: u8) -> String {
    format!("{instruction} ${offset:02X}")
}

fn format_zx(instruction: &str, offset: u8) -> String {
    format!("{instruction} ${offset:02X},X")
}

fn format_zy(instruction: &str, offset: u8) -> String {
    format!("{instruction} ${offset:02X},Y")
}

fn format_a(instruction: &str, address: u16) -> String {
    format!("{instruction} ${address:04X}")
}

fn format_ax(instruction: &str, address: u16) -> String {
    format!("{instruction} ${address:04X},X")
}

fn format_ay(instruction: &str, address: u16) -> String {
    format!("{instruction} ${address:04X},Y")
}

fn format_dx(instruction: &str, offset: u8) -> String {
    format!("{instruction} (${offset:02X},X)")
}

fn format_dy(instruction: &str, offset: u8) -> String {
    format!("{instruction} (${offset:02X}),Y")
}

fn format_rel(instruction: &str, rel: u8, pc: u16) -> String {
    format!("{instruction} ${:04X}", compute_target(rel, pc))
}

fn format_dest(instruction: &str, address: u16) -> String {
    format!("{instruction} ${:04X}", address)
}

fn format_dest_id(instruction: &str, address: u16) -> String {
    format!("{instruction} (${:04X})", address)
}

impl Decoder<String> for Disassembler {
    fn brk(&mut self) -> String {
        "BRK".to_string()
    }
    fn stp(&mut self) -> String {
        "STP".to_string()
    }
    fn nop_g(&mut self) -> String {
        "NOP".to_string()
    }
    fn nop_i(&mut self, value: u8) -> String {
        format_i("NOP", value)
    }
    fn nop_z(&mut self, offset: u8) -> String {
        format_z("NOP", offset)
    }
    fn nop_zx(&mut self, offset: u8) -> String {
        format_zx("NOP", offset)
    }
    fn nop_a(&mut self, address: u16) -> String {
        format_a("NOP", address)
    }
    fn nop_ax(&mut self, address: u16) -> String {
        format_ax("NOP", address)
    }
    fn dex(&mut self) -> String {
        "DEX".to_string()
    }
    fn dey(&mut self) -> String {
        "DEY".to_string()
    }
    fn iny(&mut self) -> String {
        "INY".to_string()
    }
    fn inx(&mut self) -> String {
        "INX".to_string()
    }
    fn inc_z(&mut self, offset: u8) -> String {
        format_z("INC", offset)
    }
    fn inc_zx(&mut self, offset: u8) -> String {
        format_zx("INC", offset)
    }
    fn inc_a(&mut self, address: u16) -> String {
        format_a("INC", address)
    }
    fn inc_ax(&mut self, address: u16) -> String {
        format_ax("INC", address)
    }
    fn dec_z(&mut self, offset: u8) -> String {
        format_z("DEC", offset)
    }
    fn dec_zx(&mut self, offset: u8) -> String {
        format_zx("DEC", offset)
    }
    fn dec_a(&mut self, address: u16) -> String {
        format_a("DEC", address)
    }
    fn dec_ax(&mut self, address: u16) -> String {
        format_ax("DEC", address)
    }
    fn php(&mut self) -> String {
        "PHP".to_string()
    }
    fn plp(&mut self) -> String {
        "PLP".to_string()
    }
    fn pha(&mut self) -> String {
        "PHA".to_string()
    }
    fn pla(&mut self) -> String {
        "PLA".to_string()
    }
    fn bit_z(&mut self, offset: u8) -> String {
        format_z("BIT", offset)
    }
    fn bit_a(&mut self, address: u16) -> String {
        format_a("BIT", address)
    }
    fn tay(&mut self) -> String {
        "TAY".to_string()
    }
    fn tya(&mut self) -> String {
        "TYA".to_string()
    }
    fn txa(&mut self) -> String {
        "TXA".to_string()
    }
    fn txs(&mut self) -> String {
        "TXS".to_string()
    }
    fn tax(&mut self) -> String {
        "TAX".to_string()
    }
    fn tsx(&mut self) -> String {
        "TSX".to_string()
    }
    fn clc(&mut self) -> String {
        "CLC".to_string()
    }
    fn sec(&mut self) -> String {
        "SEC".to_string()
    }
    fn cli(&mut self) -> String {
        "CLI".to_string()
    }
    fn sei(&mut self) -> String {
        "SEI".to_string()
    }
    fn clv(&mut self) -> String {
        "CLV".to_string()
    }
    fn cld(&mut self) -> String {
        "CLD".to_string()
    }
    fn sed(&mut self) -> String {
        "SED".to_string()
    }
    fn jmp_a(&mut self, address: u16) -> String {
        format_dest("JMP", address)
    }
    fn jmp_ad(&mut self, address: u16) -> String {
        format_dest_id("JMP", address)
    }
    fn jsr(&mut self, address: u16) -> String {
        format_dest("JSR", address)
    }
    fn rti(&mut self) -> String {
        "RTI".to_string()
    }
    fn rts(&mut self) -> String {
        "RTS".to_string()
    }
    fn bpl(&mut self, rel: u8) -> String {
        format_rel("BPL", rel, self.pc)
    }
    fn bmi(&mut self, rel: u8) -> String {
        format_rel("BMI", rel, self.pc)
    }
    fn bvc(&mut self, rel: u8) -> String {
        format_rel("BVC", rel, self.pc)
    }
    fn bvs(&mut self, rel: u8) -> String {
        format_rel("BVS", rel, self.pc)
    }
    fn bcc(&mut self, rel: u8) -> String {
        format_rel("BCC", rel, self.pc)
    }
    fn bcs(&mut self, rel: u8) -> String {
        format_rel("BCS", rel, self.pc)
    }
    fn bne(&mut self, rel: u8) -> String {
        format_rel("BNE", rel, self.pc)
    }
    fn beq(&mut self, rel: u8) -> String {
        format_rel("BEQ", rel, self.pc)
    }
    fn cpx_i(&mut self, value: u8) -> String {
        format_i("CPX", value)
    }
    fn cpx_z(&mut self, offset: u8) -> String {
        format_z("CPX", offset)
    }
    fn cpx_a(&mut self, address: u16) -> String {
        format_a("CPX", address)
    }
    fn cpy_i(&mut self, value: u8) -> String {
        format_i("CPY", value)
    }
    fn cpy_z(&mut self, offset: u8) -> String {
        format_z("CPY", offset)
    }
    fn cpy_a(&mut self, address: u16) -> String {
        format_a("CPY", address)
    }
    fn ldy_i(&mut self, value: u8) -> String {
        format_i("LDY", value)
    }
    fn ldy_z(&mut self, offset: u8) -> String {
        format_z("LDY", offset)
    }
    fn ldy_zx(&mut self, offset: u8) -> String {
        format_zx("LDY", offset)
    }
    fn ldy_a(&mut self, address: u16) -> String {
        format_a("LDY", address)
    }
    fn ldy_ax(&mut self, address: u16) -> String {
        format_ax("LDY", address)
    }
    fn ldx_i(&mut self, value: u8) -> String {
        format_i("LDX", value)
    }
    fn ldx_z(&mut self, offset: u8) -> String {
        format_z("LDX", offset)
    }
    fn ldx_zy(&mut self, offset: u8) -> String {
        format_zy("LDX", offset)
    }
    fn ldx_a(&mut self, address: u16) -> String {
        format_a("LDX", address)
    }
    fn ldx_ay(&mut self, address: u16) -> String {
        format_ay("LDX", address)
    }
    fn ora_i(&mut self, value: u8) -> String {
        format_i("ORA", value)
    }
    fn ora_z(&mut self, offset: u8) -> String {
        format_z("ORA", offset)
    }
    fn ora_zx(&mut self, offset: u8) -> String {
        format_zx("ORA", offset)
    }
    fn ora_a(&mut self, address: u16) -> String {
        format_a("ORA", address)
    }
    fn ora_ax(&mut self, address: u16) -> String {
        format_ax("ORA", address)
    }
    fn ora_ay(&mut self, address: u16) -> String {
        format_ay("ORA", address)
    }
    fn ora_dx(&mut self, offset: u8) -> String {
        format_dx("ORA", offset)
    }
    fn ora_dy(&mut self, offset: u8) -> String {
        format_dy("ORA", offset)
    }
    fn and_i(&mut self, value: u8) -> String {
        format_i("AND", value)
    }
    fn and_z(&mut self, offset: u8) -> String {
        format_z("AND", offset)
    }
    fn and_zx(&mut self, offset: u8) -> String {
        format_zx("AND", offset)
    }
    fn and_a(&mut self, address: u16) -> String {
        format_a("AND", address)
    }
    fn and_ax(&mut self, address: u16) -> String {
        format_ax("AND", address)
    }
    fn and_ay(&mut self, address: u16) -> String {
        format_ay("AND", address)
    }
    fn and_dx(&mut self, offset: u8) -> String {
        format_dx("AND", offset)
    }
    fn and_dy(&mut self, offset: u8) -> String {
        format_dy("AND", offset)
    }
    fn eor_i(&mut self, value: u8) -> String {
        format_i("EOR", value)
    }
    fn eor_z(&mut self, offset: u8) -> String {
        format_z("EOR", offset)
    }
    fn eor_zx(&mut self, offset: u8) -> String {
        format_zx("EOR", offset)
    }
    fn eor_a(&mut self, address: u16) -> String {
        format_a("EOR", address)
    }
    fn eor_ax(&mut self, address: u16) -> String {
        format_ax("EOR", address)
    }
    fn eor_ay(&mut self, address: u16) -> String {
        format_ay("EOR", address)
    }
    fn eor_dx(&mut self, offset: u8) -> String {
        format_dx("EOR", offset)
    }
    fn eor_dy(&mut self, offset: u8) -> String {
        format_dy("EOR", offset)
    }
    fn adc_i(&mut self, value: u8) -> String {
        format_i("ADC", value)
    }
    fn adc_z(&mut self, offset: u8) -> String {
        format_z("ADC", offset)
    }
    fn adc_zx(&mut self, offset: u8) -> String {
        format_zx("ADC", offset)
    }
    fn adc_a(&mut self, address: u16) -> String {
        format_a("ADC", address)
    }
    fn adc_ax(&mut self, address: u16) -> String {
        format_ax("ADC", address)
    }
    fn adc_ay(&mut self, address: u16) -> String {
        format_ay("ADC", address)
    }
    fn adc_dx(&mut self, offset: u8) -> String {
        format_dx("ADC", offset)
    }
    fn adc_dy(&mut self, offset: u8) -> String {
        format_dy("ADC", offset)
    }
    fn sta_z(&mut self, offset: u8) -> String {
        format_z("STA", offset)
    }
    fn sta_zx(&mut self, offset: u8) -> String {
        format_zx("STA", offset)
    }
    fn sta_a(&mut self, address: u16) -> String {
        format_a("STA", address)
    }
    fn sta_ax(&mut self, address: u16) -> String {
        format_ax("STA", address)
    }
    fn sta_ay(&mut self, address: u16) -> String {
        format_ay("STA", address)
    }
    fn sta_dx(&mut self, offset: u8) -> String {
        format_dx("STA", offset)
    }
    fn sta_dy(&mut self, offset: u8) -> String {
        format_dy("STA", offset)
    }
    fn stx_z(&mut self, offset: u8) -> String {
        format_z("STX", offset)
    }
    fn stx_zy(&mut self, offset: u8) -> String {
        format_zy("STX", offset)
    }
    fn stx_a(&mut self, address: u16) -> String {
        format_a("STX", address)
    }
    fn sty_z(&mut self, offset: u8) -> String {
        format_z("STY", offset)
    }
    fn sty_zx(&mut self, offset: u8) -> String {
        format_zx("STY", offset)
    }
    fn sty_a(&mut self, address: u16) -> String {
        format_a("STY", address)
    }
    fn lda_i(&mut self, value: u8) -> String {
        format_i("LDA", value)
    }
    fn lda_z(&mut self, offset: u8) -> String {
        format_z("LDA", offset)
    }
    fn lda_zx(&mut self, offset: u8) -> String {
        format_zx("LDA", offset)
    }
    fn lda_a(&mut self, address: u16) -> String {
        format_a("LDA", address)
    }
    fn lda_ax(&mut self, address: u16) -> String {
        format_ax("LDA", address)
    }
    fn lda_ay(&mut self, address: u16) -> String {
        format_ay("LDA", address)
    }
    fn lda_dx(&mut self, offset: u8) -> String {
        format_dx("LDA", offset)
    }
    fn lda_dy(&mut self, offset: u8) -> String {
        format_dy("LDA", offset)
    }
    fn cmp_i(&mut self, value: u8) -> String {
        format_i("CMP", value)
    }
    fn cmp_z(&mut self, offset: u8) -> String {
        format_z("CMP", offset)
    }
    fn cmp_zx(&mut self, offset: u8) -> String {
        format_zx("CMP", offset)
    }
    fn cmp_a(&mut self, address: u16) -> String {
        format_a("CMP", address)
    }
    fn cmp_ax(&mut self, address: u16) -> String {
        format_ax("CMP", address)
    }
    fn cmp_ay(&mut self, address: u16) -> String {
        format_ay("CMP", address)
    }
    fn cmp_dx(&mut self, offset: u8) -> String {
        format_dx("CMP", offset)
    }
    fn cmp_dy(&mut self, offset: u8) -> String {
        format_dy("CMP", offset)
    }
    fn sbc_i(&mut self, value: u8) -> String {
        format_i("SBC", value)
    }
    fn sbc_z(&mut self, offset: u8) -> String {
        format_z("SBC", offset)
    }
    fn sbc_zx(&mut self, offset: u8) -> String {
        format_zx("SBC", offset)
    }
    fn sbc_a(&mut self, address: u16) -> String {
        format_a("SBC", address)
    }
    fn sbc_ax(&mut self, address: u16) -> String {
        format_ax("SBC", address)
    }
    fn sbc_ay(&mut self, address: u16) -> String {
        format_ay("SBC", address)
    }
    fn sbc_dx(&mut self, offset: u8) -> String {
        format_dx("SBC", offset)
    }
    fn sbc_dy(&mut self, offset: u8) -> String {
        format_dy("SBC", offset)
    }
    fn asl_g(&mut self) -> String {
        "ASL A".to_string()
    }
    fn asl_z(&mut self, offset: u8) -> String {
        format_z("ASL", offset)
    }
    fn asl_zx(&mut self, offset: u8) -> String {
        format_zx("ASL", offset)
    }
    fn asl_a(&mut self, address: u16) -> String {
        format_a("ASL", address)
    }
    fn asl_ax(&mut self, address: u16) -> String {
        format_ax("ASL", address)
    }
    fn rol_g(&mut self) -> String {
        "ROL A".to_string()
    }
    fn rol_z(&mut self, offset: u8) -> String {
        format_z("ROL", offset)
    }
    fn rol_zx(&mut self, offset: u8) -> String {
        format_zx("ROL", offset)
    }
    fn rol_a(&mut self, address: u16) -> String {
        format_a("ROL", address)
    }
    fn rol_ax(&mut self, address: u16) -> String {
        format_ax("ROL", address)
    }
    fn ror_g(&mut self) -> String {
        "ROR A".to_string()
    }
    fn ror_z(&mut self, offset: u8) -> String {
        format_z("ROR", offset)
    }
    fn ror_zx(&mut self, offset: u8) -> String {
        format_zx("ROR", offset)
    }
    fn ror_a(&mut self, address: u16) -> String {
        format_a("ROR", address)
    }
    fn ror_ax(&mut self, address: u16) -> String {
        format_ax("ROR", address)
    }
    fn lsr_g(&mut self) -> String {
        "LSR A".to_string()
    }
    fn lsr_z(&mut self, offset: u8) -> String {
        format_z("LSR", offset)
    }
    fn lsr_zx(&mut self, offset: u8) -> String {
        format_zx("LSR", offset)
    }
    fn lsr_a(&mut self, address: u16) -> String {
        format_a("LSR", address)
    }
    fn lsr_ax(&mut self, address: u16) -> String {
        format_ax("LSR", address)
    }
}
