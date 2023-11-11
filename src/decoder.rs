/*
 Instruction Postfixes:
 _g  -> No Parameter
 _i  -> Immediate
 _z  -> Zero Page
 _zx -> Zero Page + X
 _zy -> Zero Page + Y
 _a  -> Absolute
 _ax -> Absolute + X
 _ay -> Absolute + Y
 _dx -> Indirect X
 _dy -> Indirect Y
 */

fn get_address<T, F: FnMut(&mut T) -> Option<u8>>(t: &mut T, next: &mut F) -> Option<u16> {
    let low = next(t)? as u16;
    let high = next(t)? as u16;

    Some((high << 8) | low)
}

pub trait Decoder<T>: Sized {
    fn brk(&mut self) -> T;
    fn stp(&mut self) -> T;

    fn nop_g(&mut self) -> T;
    fn nop_i(&mut self, value: u8) -> T;
    fn nop_z(&mut self, offset: u8) -> T;
    fn nop_zx(&mut self, offset: u8) -> T;
    fn nop_a(&mut self, address: u16) -> T;
    fn nop_ax(&mut self, address: u16) -> T;

    fn dex(&mut self) -> T;
    fn dey(&mut self) -> T;
    fn iny(&mut self) -> T;
    fn inx(&mut self) -> T;

    fn inc_z(&mut self, offset: u8) -> T;
    fn inc_zx(&mut self, offset: u8) -> T;
    fn inc_a(&mut self, address: u16) -> T;
    fn inc_ax(&mut self, address: u16) -> T;

    fn dec_z(&mut self, offset: u8) -> T;
    fn dec_zx(&mut self, offset: u8) -> T;
    fn dec_a(&mut self, address: u16) -> T;
    fn dec_ax(&mut self, address: u16) -> T;

    fn php(&mut self) -> T;
    fn plp(&mut self) -> T;
    fn pha(&mut self) -> T;
    fn pla(&mut self) -> T;

    fn bit_z(&mut self, offset: u8) -> T;
    fn bit_a(&mut self, address: u16) -> T;

    fn tay(&mut self) -> T;
    fn tya(&mut self) -> T;
    fn txa(&mut self) -> T;
    fn txs(&mut self) -> T;
    fn tax(&mut self) -> T;
    fn tsx(&mut self) -> T;

    fn clc(&mut self) -> T;
    fn sec(&mut self) -> T;
    fn cli(&mut self) -> T;
    fn sei(&mut self) -> T;
    fn clv(&mut self) -> T;
    fn cld(&mut self) -> T;
    fn sed(&mut self) -> T;

    fn jmp_a(&mut self, address: u16) -> T;
    fn jmp_ad(&mut self, address: u16) -> T;

    fn jsr(&mut self, address: u16) -> T;
    fn rti(&mut self) -> T;
    fn rts(&mut self) -> T;

    fn bpl(&mut self, rel: u8) -> T;
    fn bmi(&mut self, rel: u8) -> T;
    fn bvc(&mut self, rel: u8) -> T;
    fn bvs(&mut self, rel: u8) -> T;
    fn bcc(&mut self, rel: u8) -> T;
    fn bcs(&mut self, rel: u8) -> T;
    fn bne(&mut self, rel: u8) -> T;
    fn beq(&mut self, rel: u8) -> T;

    fn cpx_i(&mut self, value: u8) -> T;
    fn cpx_z(&mut self, offset: u8) -> T;
    fn cpx_a(&mut self, address: u16) -> T;

    fn cpy_i(&mut self, value: u8) -> T;
    fn cpy_z(&mut self, offset: u8) -> T;
    fn cpy_a(&mut self, address: u16) -> T;

    fn ldy_i(&mut self, value: u8) -> T;
    fn ldy_z(&mut self, offset: u8) -> T;
    fn ldy_zx(&mut self, offset: u8) -> T;
    fn ldy_a(&mut self, address: u16) -> T;
    fn ldy_ax(&mut self, address: u16) -> T;

    fn ldx_i(&mut self, value: u8) -> T;
    fn ldx_z(&mut self, offset: u8) -> T;
    fn ldx_zy(&mut self, offset: u8) -> T;
    fn ldx_a(&mut self, address: u16) -> T;
    fn ldx_ay(&mut self, address: u16) -> T;

    fn ora_i(&mut self, value: u8) -> T;
    fn ora_z(&mut self, offset: u8) -> T;
    fn ora_zx(&mut self, offset: u8) -> T;
    fn ora_a(&mut self, address: u16) -> T;
    fn ora_ax(&mut self, address: u16) -> T;
    fn ora_ay(&mut self, address: u16) -> T;
    fn ora_dx(&mut self, offset: u8) -> T;
    fn ora_dy(&mut self, offset: u8) -> T;

    fn and_i(&mut self, value: u8) -> T;
    fn and_z(&mut self, offset: u8) -> T;
    fn and_zx(&mut self, offset: u8) -> T;
    fn and_a(&mut self, address: u16) -> T;
    fn and_ax(&mut self, address: u16) -> T;
    fn and_ay(&mut self, address: u16) -> T;
    fn and_dx(&mut self, offset: u8) -> T;
    fn and_dy(&mut self, offset: u8) -> T;

    fn eor_i(&mut self, value: u8) -> T;
    fn eor_z(&mut self, offset: u8) -> T;
    fn eor_zx(&mut self, offset: u8) -> T;
    fn eor_a(&mut self, address: u16) -> T;
    fn eor_ax(&mut self, address: u16) -> T;
    fn eor_ay(&mut self, address: u16) -> T;
    fn eor_dx(&mut self, offset: u8) -> T;
    fn eor_dy(&mut self, offset: u8) -> T;

    fn adc_i(&mut self, value: u8) -> T;
    fn adc_z(&mut self, offset: u8) -> T;
    fn adc_zx(&mut self, offset: u8) -> T;
    fn adc_a(&mut self, address: u16) -> T;
    fn adc_ax(&mut self, address: u16) -> T;
    fn adc_ay(&mut self, address: u16) -> T;
    fn adc_dx(&mut self, offset: u8) -> T;
    fn adc_dy(&mut self, offset: u8) -> T;

    fn sta_z(&mut self, offset: u8) -> T;
    fn sta_zx(&mut self, offset: u8) -> T;
    fn sta_a(&mut self, address: u16) -> T;
    fn sta_ax(&mut self, address: u16) -> T;
    fn sta_ay(&mut self, address: u16) -> T;
    fn sta_dx(&mut self, offset: u8) -> T;
    fn sta_dy(&mut self, offset: u8) -> T;

    fn stx_z(&mut self, offset: u8) -> T;
    fn stx_zy(&mut self, offset: u8) -> T;
    fn stx_a(&mut self, address: u16) -> T;

    fn sty_z(&mut self, offset: u8) -> T;
    fn sty_zx(&mut self, offset: u8) -> T;
    fn sty_a(&mut self, address: u16) -> T;

    fn lda_i(&mut self, value: u8) -> T;
    fn lda_z(&mut self, offset: u8) -> T;
    fn lda_zx(&mut self, offset: u8) -> T;
    fn lda_a(&mut self, address: u16) -> T;
    fn lda_ax(&mut self, address: u16) -> T;
    fn lda_ay(&mut self, address: u16) -> T;
    fn lda_dx(&mut self, offset: u8) -> T;
    fn lda_dy(&mut self, offset: u8) -> T;

    fn cmp_i(&mut self, value: u8) -> T;
    fn cmp_z(&mut self, offset: u8) -> T;
    fn cmp_zx(&mut self, offset: u8) -> T;
    fn cmp_a(&mut self, address: u16) -> T;
    fn cmp_ax(&mut self, address: u16) -> T;
    fn cmp_ay(&mut self, address: u16) -> T;
    fn cmp_dx(&mut self, offset: u8) -> T;
    fn cmp_dy(&mut self, offset: u8) -> T;

    fn sbc_i(&mut self, value: u8) -> T;
    fn sbc_z(&mut self, offset: u8) -> T;
    fn sbc_zx(&mut self, offset: u8) -> T;
    fn sbc_a(&mut self, address: u16) -> T;
    fn sbc_ax(&mut self, address: u16) -> T;
    fn sbc_ay(&mut self, address: u16) -> T;
    fn sbc_dx(&mut self, offset: u8) -> T;
    fn sbc_dy(&mut self, offset: u8) -> T;

    fn asl_g(&mut self) -> T;
    fn asl_z(&mut self, offset: u8) -> T;
    fn asl_zx(&mut self, offset: u8) -> T;
    fn asl_a(&mut self, address: u16) -> T;
    fn asl_ax(&mut self, address: u16) -> T;

    fn rol_g(&mut self) -> T;
    fn rol_z(&mut self, offset: u8) -> T;
    fn rol_zx(&mut self, offset: u8) -> T;
    fn rol_a(&mut self, address: u16) -> T;
    fn rol_ax(&mut self, address: u16) -> T;

    fn ror_g(&mut self) -> T;
    fn ror_z(&mut self, offset: u8) -> T;
    fn ror_zx(&mut self, offset: u8) -> T;
    fn ror_a(&mut self, address: u16) -> T;
    fn ror_ax(&mut self, address: u16) -> T;

    fn lsr_g(&mut self) -> T;
    fn lsr_z(&mut self, offset: u8) -> T;
    fn lsr_zx(&mut self, offset: u8) -> T;
    fn lsr_a(&mut self, address: u16) -> T;
    fn lsr_ax(&mut self, address: u16) -> T;

    fn decode<F: FnMut(&mut Self) -> Option<u8>>(&mut self, mut next: F) -> Option<T> {
        let op = next(self)?;

        Some(match op {
            0x00 => self.brk(),
            0x01 => { let x = next(self)?; self.ora_dx(x) }, // NOOO
            0x02 => self.stp(),
            0x03 => return None,
            0x04 => { let x = next(self)?; self.nop_z(x) },
            0x05 => { let x = next(self)?; self.ora_z(x) },
            0x06 => { let x = next(self)?; self.asl_z(x) },
            0x07 => return None,
            0x08 => self.php(),
            0x09 => { let x = next(self)?; self.ora_i(x) },
            0x0A => self.asl_g(),
            0x0B => return None,
            0x0C => { let x = get_address(self, &mut next)?; self.nop_a(x) }, // NOP
            0x0D => { let x = get_address(self, &mut next)?; self.ora_a(x) },
            0x0E => { let x = get_address(self, &mut next)?; self.asl_a(x) },
            0x0F => return None,
            0x10 => { let x = next(self)?; self.bpl(x) },
            0x11 => { let x = next(self)?; self.ora_dy(x) },
            0x12 => self.stp(),
            0x13 => return None,
            0x14 => { let x = next(self)?; self.nop_zx(x) }, // NOP
            0x15 => { let x = next(self)?; self.ora_zx(x) },
            0x16 => { let x = next(self)?; self.asl_zx(x) },
            0x17 => return None,
            0x18 => self.clc(),
            0x19 => { let x = get_address(self, &mut next)?; self.ora_ay(x) },
            0x1A => self.nop_g(), // NOP
            0x1B => return None,
            0x1C => { let x = get_address(self, &mut next)?; self.nop_ax(x) }, // NOP
            0x1D => { let x = get_address(self, &mut next)?; self.ora_ax(x) },
            0x1E => { let x = get_address(self, &mut next)?; self.asl_ax(x) },
            0x1F => return None,
            0x20 => { let x = get_address(self, &mut next)?; self.jsr(x) },
            0x21 => { let x = next(self)?; self.and_dx(x) },
            0x22 => self.stp(),
            0x23 => return None,
            0x24 => { let x = next(self)?; self.bit_z(x) },
            0x25 => { let x = next(self)?; self.and_z(x) },
            0x26 => { let x = next(self)?; self.rol_z(x) },
            0x27 => return None,
            0x28 => self.plp(),
            0x29 => { let x = next(self)?; self.and_i(x) },
            0x2A => self.rol_g(),
            0x2B => return None,
            0x2C => { let x = get_address(self, &mut next)?; self.bit_a(x) },
            0x2D => { let x = get_address(self, &mut next)?; self.and_a(x) },
            0x2E => { let x = get_address(self, &mut next)?; self.rol_a(x) },
            0x2F => return None,
            0x30 => { let x = next(self)?; self.bmi(x) },
            0x31 => { let x = next(self)?; self.and_dy(x) },
            0x32 => self.stp(),
            0x33 => return None,
            0x34 => { let x = next(self)?; self.nop_zx(x) }, // NOP
            0x35 => { let x = next(self)?; self.and_zx(x) },
            0x36 => { let x = next(self)?; self.rol_zx(x) },
            0x37 => return None,
            0x38 => self.sec(),
            0x39 => { let x = get_address(self, &mut next)?; self.and_ay(x) },
            0x3A => self.nop_g(), // NOP
            0x3B => return None,
            0x3C => { let x = get_address(self, &mut next)?; self.nop_ax(x) }, // NOP
            0x3D => { let x = get_address(self, &mut next)?; self.and_ax(x) },
            0x3E => { let x = get_address(self, &mut next)?; self.rol_ax(x) },
            0x3F => return None,
            0x40 => self.rti(),
            0x41 => { let x = next(self)?; self.eor_dx(x) },
            0x42 => self.stp(),
            0x43 => return None,
            0x44 => { let x = next(self)?; self.nop_z(x) }, // NOP
            0x45 => { let x = next(self)?; self.eor_z(x) },
            0x46 => { let x = next(self)?; self.lsr_z(x) },
            0x47 => return None,
            0x48 => self.pha(),
            0x49 => { let x = next(self)?; self.eor_i(x) },
            0x4A => self.lsr_g(),
            0x4B => return None,
            0x4C => { let x = get_address(self, &mut next)?; self.jmp_a(x) },
            0x4D => { let x = get_address(self, &mut next)?; self.eor_a(x) },
            0x4E => { let x = get_address(self, &mut next)?; self.lsr_a(x) },
            0x4F => return None,
            0x50 => { let x = next(self)?; self.bvc(x) },
            0x51 => { let x = next(self)?; self.eor_dy(x) },
            0x52 => self.stp(),
            0x53 => return None,
            0x54 => { let x = next(self)?; self.nop_zx(x) }, // NOP
            0x55 => { let x = next(self)?; self.eor_zx(x) },
            0x56 => { let x = next(self)?; self.lsr_zx(x) },
            0x57 => return None,
            0x58 => self.cli(),
            0x59 => { let x = get_address(self, &mut next)?; self.eor_ay(x) },
            0x5A => self.nop_g(), // NOP
            0x5B => return None,
            0x5C => { let x = get_address(self, &mut next)?; self.nop_ax(x) }, // NOP
            0x5D => { let x = get_address(self, &mut next)?; self.eor_ax(x) },
            0x5E => { let x = get_address(self, &mut next)?; self.lsr_ax(x) },
            0x5F => return None,
            0x60 => self.rts(),
            0x61 => { let x = next(self)?; self.adc_dx(x) },
            0x62 => self.stp(),
            0x63 => return None,
            0x64 => { let x = next(self)?; self.nop_z(x) }, // NOP
            0x65 => { let x = next(self)?; self.adc_z(x) },
            0x66 => { let x = next(self)?; self.ror_z(x) },
            0x67 => return None,
            0x68 => self.pla(),
            0x69 => { let x = next(self)?; self.adc_i(x) },
            0x6A => self.ror_g(),
            0x6B => return None,
            0x6C => { let x = get_address(self, &mut next)?; self.jmp_ad(x) },
            0x6D => { let x = get_address(self, &mut next)?; self.adc_a(x) },
            0x6E => { let x = get_address(self, &mut next)?; self.ror_a(x) },
            0x6F => return None,
            0x70 => { let x = next(self)?; self.bvs(x) },
            0x71 => { let x = next(self)?; self.adc_dy(x) },
            0x72 => self.stp(),
            0x73 => return None,
            0x74 => { let x = next(self)?; self.nop_zx(x) }, // NOP
            0x75 => { let x = next(self)?; self.adc_zx(x) },
            0x76 => { let x = next(self)?; self.ror_zx(x) },
            0x77 => return None,
            0x78 => self.sei(),
            0x79 => { let x = get_address(self, &mut next)?; self.adc_ay(x) },
            0x7A => self.nop_g(), // NOP
            0x7B => return None,
            0x7C => { let x = get_address(self, &mut next)?; self.nop_ax(x) }, // NOP
            0x7D => { let x = get_address(self, &mut next)?; self.adc_ax(x) },
            0x7E => { let x = get_address(self, &mut next)?; self.ror_ax(x) },
            0x7F => return None, // NOP
            0x80 => { let x = next(self)?; self.nop_i(x) }, // NOP
            0x81 => { let x = next(self)?; self.sta_dx(x) },
            0x82 => { let x = next(self)?; self.nop_i(x) }, // NOP
            0x83 => return None,
            0x84 => { let x = next(self)?; self.sty_z(x) },
            0x85 => { let x = next(self)?; self.sta_z(x) },
            0x86 => { let x = next(self)?; self.stx_z(x) },
            0x87 => return None,
            0x88 => self.dey(),
            0x89 => { let x = next(self)?; self.nop_i(x) }, // NOP
            0x8A => self.txa(),
            0x8B => return None,
            0x8C => { let x = get_address(self, &mut next)?; self.sty_a(x) },
            0x8D => { let x = get_address(self, &mut next)?; self.sta_a(x) },
            0x8E => { let x = get_address(self, &mut next)?; self.stx_a(x) },
            0x8F => return None,
            0x90 => { let x = next(self)?; self.bcc(x) },
            0x91 => { let x = next(self)?; self.sta_dy(x) },
            0x92 => self.stp(),
            0x93 => return None,
            0x94 => { let x = next(self)?; self.sty_zx(x) },
            0x95 => { let x = next(self)?; self.sta_zx(x) },
            0x96 => { let x = next(self)?; self.stx_zy(x) },
            0x97 => return None,
            0x98 => self.tya(),
            0x99 => { let x = get_address(self, &mut next)?; self.sta_ay(x) },
            0x9A => self.txs(),
            0x9B => return None,
            0x9C => return None, // SHY
            0x9D => { let x = get_address(self, &mut next)?; self.sta_ax(x) },
            0x9E => return None, // SHX
            0x9F => return None,
            0xA0 => { let x = next(self)?; self.ldy_i(x) },
            0xA1 => { let x = next(self)?; self.lda_dx(x) },
            0xA2 => { let x = next(self)?; self.ldx_i(x) },
            0xA3 => return None,
            0xA4 => { let x = next(self)?; self.ldy_z(x) },
            0xA5 => { let x = next(self)?; self.lda_z(x) },
            0xA6 => { let x = next(self)?; self.ldx_z(x) },
            0xA7 => return None,
            0xA8 => self.tay(),
            0xA9 => { let x = next(self)?; self.lda_i(x) },
            0xAA => self.tax(),
            0xAB => return None,
            0xAC => { let x = get_address(self, &mut next)?; self.ldy_a(x) },
            0xAD => { let x = get_address(self, &mut next)?; self.lda_a(x) },
            0xAE => { let x = get_address(self, &mut next)?; self.ldx_a(x) },
            0xAF => return None,
            0xB0 => { let x = next(self)?; self.bcs(x) },
            0xB1 => { let x = next(self)?; self.lda_dy(x) },
            0xB2 => self.stp(),
            0xB3 => return None,
            0xB4 => { let x = next(self)?; self.ldy_zx(x) },
            0xB5 => { let x = next(self)?; self.lda_zx(x) },
            0xB6 => { let x = next(self)?; self.ldx_zy(x) },
            0xB7 => return None,
            0xB8 => self.clv(),
            0xB9 => { let x = get_address(self, &mut next)?; self.lda_ay(x) },
            0xBA => self.tsx(),
            0xBB => return None,
            0xBC => { let x = get_address(self, &mut next)?; self.ldy_ax(x) },
            0xBD => { let x = get_address(self, &mut next)?; self.lda_ax(x) },
            0xBE => { let x = get_address(self, &mut next)?; self.ldx_ay(x) },
            0xBF => return None,
            0xC0 => { let x = next(self)?; self.cpy_i(x) },
            0xC1 => { let x = next(self)?; self.cmp_dx(x) },
            0xC2 => { let x = next(self)?; self.nop_i(x) }, // NOP
            0xC3 => return None,
            0xC4 => { let x = next(self)?; self.cpy_z(x) },
            0xC5 => { let x = next(self)?; self.cmp_z(x) },
            0xC6 => { let x = next(self)?; self.dec_z(x) },
            0xC7 => return None,
            0xC8 => self.iny(),
            0xC9 => { let x = next(self)?; self.cmp_i(x) },
            0xCA => self.dex(),
            0xCB => return None,
            0xCC => { let x = get_address(self, &mut next)?; self.cpy_a(x) },
            0xCD => { let x = get_address(self, &mut next)?; self.cmp_a(x) },
            0xCE => { let x = get_address(self, &mut next)?; self.dec_a(x) },
            0xCF => return None,
            0xD0 => { let x = next(self)?; self.bne(x) },
            0xD1 => { let x = next(self)?; self.cmp_dy(x) },
            0xD2 => self.stp(),
            0xD3 => return None,
            0xD4 => { let x = next(self)?; self.nop_zx(x) }, // NOP
            0xD5 => { let x = next(self)?; self.cmp_zx(x) },
            0xD6 => { let x = next(self)?; self.dec_zx(x) },
            0xD7 => return None,
            0xD8 => self.cld(),
            0xD9 => { let x = get_address(self, &mut next)?; self.cmp_ay(x) },
            0xDA => self.nop_g(), // NOP
            0xDB => return None,
            0xDC => { let x = get_address(self, &mut next)?; self.nop_ax(x) }, // NOP
            0xDD => { let x = get_address(self, &mut next)?; self.cmp_ax(x) },
            0xDE => { let x = get_address(self, &mut next)?; self.dec_ax(x) },
            0xDF => return None,
            0xE0 => { let x = next(self)?; self.cpx_i(x) },
            0xE1 => { let x = next(self)?; self.sbc_dx(x) },
            0xE2 => { let x = next(self)?; self.nop_i(x) }, // NOP
            0xE3 => return None,
            0xE4 => { let x = next(self)?; self.cpx_z(x) },
            0xE5 => { let x = next(self)?; self.sbc_z(x) },
            0xE6 => { let x = next(self)?; self.inc_z(x) },
            0xE7 => return None,
            0xE8 => self.inx(),
            0xE9 => { let x = next(self)?; self.sbc_i(x) },
            0xEA => self.nop_g(),
            0xEB => { let x = next(self)?; self.sbc_i(x) }, // Unofficial
            0xEC => { let x = get_address(self, &mut next)?; self.cpx_a(x) },
            0xED => { let x = get_address(self, &mut next)?; self.sbc_a(x) },
            0xEE => { let x = get_address(self, &mut next)?; self.inc_a(x) },
            0xEF => return None,
            0xF0 => { let x = next(self)?; self.beq(x) },
            0xF1 => { let x = next(self)?; self.sbc_dy(x) },
            0xF2 => self.stp(),
            0xF3 => return None,
            0xF4 => { let x = next(self)?; self.nop_zx(x) }, // NOP
            0xF5 => { let x = next(self)?; self.sbc_zx(x) },
            0xF6 => { let x = next(self)?; self.inc_zx(x) },
            0xF7 => return None,
            0xF8 => self.sed(),
            0xF9 => { let x = get_address(self, &mut next)?; self.sbc_ay(x) },
            0xFA => self.nop_g(), // NOP
            0xFB => return None,
            0xFC => { let x = get_address(self, &mut next)?; self.nop_ax(x) }, // NOP
            0xFD => { let x = get_address(self, &mut next)?; self.sbc_ax(x) },
            0xFE => { let x = get_address(self, &mut next)?; self.inc_ax(x) },
            0xFF => return None
        })
    }
}

pub fn decoder_iterator<T, F: FnMut(u16) -> Option<u8>>(mut f: F) -> impl FnMut(&mut T) -> Option<u8> {
    let mut i = 0;
    
    move |_| {
        let value = f(i);
        
        i += 1;
        
        value
    }
}

