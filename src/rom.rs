use nom::bytes::complete::{tag, take as take_bytes};
use nom::IResult;
use nom::number::complete::{u8 as take_u8};
use nom::bits::complete::{bool, take as take_bits};

#[derive(Clone, Debug)]
pub enum Mirroring {
    Horizontal,
    Vertical
}

#[derive(Clone, Debug)]
pub struct Flags {
    pub mirroring: Mirroring,
    pub battery_ram: bool,
    pub has_trainer: bool,
    pub four_screen: bool,
    pub uni_system: bool,
    pub play_choice: bool,
    pub nes2_test: u8,
    pub mapper: u8,
}

#[derive(Clone, Debug)]
pub struct Rom {
    pub flags: Flags,
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>
}

pub fn parse_flags(bytes: &[u8]) -> IResult<(&[u8], usize), Flags> {
    let bits = (bytes, 0);

    let (bits, lower_mapper): (_, u8) = take_bits(4usize)(bits)?;
    let (bits, four_screen) = bool(bits)?;
    let (bits, has_trainer) = bool(bits)?;
    let (bits, battery_ram) = bool(bits)?;

    let (bits, mirroring) = bool(bits)?;
    let mirroring = if mirroring { Mirroring::Vertical } else { Mirroring::Horizontal };

    let (bits, upper_mapper): (_, u8) = take_bits(4usize)(bits)?;
    let (bits, nes2_test): (_, u8) = take_bits(2usize)(bits)?;
    let (bits, play_choice) = bool(bits)?;
    let (bits, uni_system) = bool(bits)?;

    let mapper = (upper_mapper << 4) | lower_mapper;

    Ok((bits, Flags {
        mirroring,
        battery_ram,
        has_trainer,
        four_screen,
        uni_system,
        play_choice,
        nes2_test,
        mapper,
    }))
}

pub fn parse_rom(bytes: &[u8]) -> IResult<&[u8], Rom> {
    let (bytes, _) = tag([b'N', b'E', b'S', 0x1A])(bytes)?;

    let (bytes, prg_size) = take_u8(bytes)?;
    let (bytes, chr_size) = take_u8(bytes)?;

    let ((bytes, _), flags) = parse_flags(bytes)
        .map_err(|e| e.map_input(|(bytes, _)| bytes))?;

    let (bytes, _) = take_bytes(8usize)(bytes)?;

    let prg_size = 16384 * (prg_size as usize);
    let chr_size = 8192 * (chr_size as usize);

    let (bytes, prg_rom) = take_bytes(prg_size)(bytes)?;
    let (bytes, chr_rom) = take_bytes(chr_size)(bytes)?;

    Ok((bytes, Rom {
        flags,
        prg_rom: prg_rom.to_vec(),
        chr_rom: chr_rom.to_vec(),
    }))
}
