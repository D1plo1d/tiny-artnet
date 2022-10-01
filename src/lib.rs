#![no_std]
/// A no_std ArtNet 4 implementation for creating microcontroller-based ArtNet Nodes.
///
/// Implemented:
///
/// ✓ Node Discovery:
///     ✓ ArtPoll: Parser
///     ✓ ArtPollReply: Serialization
/// ✓ DMX Lighting Control:
///     ✓ ArtDMX: Parser
/// ✓ Re-Programming:
///     ✓ ArtCommand: Parser
///
/// Not Implemented: Literally everything else. Pull Request are welcome.
///
/// ArtNet 4 Spec: https://artisticlicence.com/WebSiteMaster/User%20Guides/art-net.pdf
extern crate tiny_artnet_bytes_no_atomic as bytes;

mod poll_reply;
pub use poll_reply::PollReply;

use core::ops::RangeInclusive;

use bytes::BufMut;
use nom::{
    bytes::complete::tag,
    number::complete as number,
    number::complete::{be_u16, le_u16},
    sequence::tuple,
    IResult,
};

const ID: &'static [u8] = b"Art-Net\0";
pub const PORT: u16 = 0x1936;

const DEFAULT_4_BYTES: &'static [u8; 4] = &[0; 4];
const DEFAULT_6_BYTES: &'static [u8; 6] = &[0; 6];

#[derive(Debug)]
pub enum Art<'a> {
    Poll(Poll),
    // PollReply(PollReply),
    Command(Command<'a>),
    Dmx(Dmx<'a>),
    Sync,
}

#[derive(Debug)]
pub enum Error<'a> {
    UnsupportedProtocolVersion(u16),
    UnsupportedOpCode(u16),
    ParserError(nom::Err<nom::error::Error<&'a [u8]>>),
}

impl<'a> From<nom::Err<nom::error::Error<&'a [u8]>>> for Error<'a> {
    fn from(err: nom::Err<nom::error::Error<&'a [u8]>>) -> Self {
        Error::ParserError(err)
    }
}

pub fn from_slice<'a>(s: &'a [u8]) -> Result<Art<'a>, Error<'a>> {
    // ID
    let (s, _) = tag(ID)(s)?;

    let (s, op_code) = le_u16(s)?;
    let (s, protocol_version): (&'a [u8], u16) = be_u16(s)?;

    if protocol_version > 14 {
        return Err(Error::UnsupportedProtocolVersion(protocol_version));
    }

    let message = match op_code {
        0x2000 => Art::Poll(parse_poll(s)?),
        // poll_reply::OP_POLL_REPLY => Art::PollReply(poll_reply::from_str(s)?),
        0x2400 => Art::Command(parse_command(s)?),
        0x5000 => Art::Dmx(parse_dmx(s)?),
        0x5200 => parse_sync(s).map(|_| Art::Sync)?,
        _ => return Err(Error::UnsupportedOpCode(op_code)),
    };

    Ok(message)
}

/// (ESTAManLo, ESTAManHi)
pub type ESTAManufacturerCode = (char, char);

fn parse_esta_manufacturer_code<'a>(s: &'a [u8]) -> IResult<&'a [u8], ESTAManufacturerCode> {
    let (s, (lo, hi)) = tuple((number::u8, number::u8))(s)?;
    Ok((s, (lo as char, hi as char)))
}

pub fn put_esta_manufacturer_code<B: BufMut>(
    buf: &mut B,
    manufacturer_code: &ESTAManufacturerCode,
) {
    buf.put_u8(manufacturer_code.0 as u8);
    buf.put_u8(manufacturer_code.1 as u8);
}

/// One of the 32,768 possible addresses to which a DMX frame can be
/// directed. The Port-Address is a 15-bit number composed of Net+Sub-Net+Universe.
///
/// Bits:
///     | 15 | 8-14 | 4-7    | 0-3      |
///     | 0  | Net  | SubNet | Universe |
#[derive(Debug)]
pub struct PortAddress {
    pub net: u8,
    pub sub_net: u8,
    pub universe: u8,
}

fn parse_port_address<'a>(s: &'a [u8]) -> IResult<&'a [u8], PortAddress> {
    use nom::bits::complete as bits;

    let (s, (sub_net, universe, _, net)): (&[u8], (u8, u8, u8, u8)) = nom::bits::bits(tuple((
        // Low Byte (SubUni)
        bits::take::<&[u8], u8, usize, nom::error::Error<(&[u8], usize)>>(4usize),
        bits::take(4usize),
        // High Byte (Net)
        bits::take(1usize),
        bits::take(7usize),
    )))(s)?;

    let port_address = PortAddress {
        net,
        sub_net,
        universe,
    };

    Ok((s, port_address))
}

impl PortAddress {
    /// Combines the Net, SubNet and Universe into a single usize index. Note this is not the same as the little endian u16 sent over the wire.
    pub fn as_index(&self) -> usize {
        (self.net as usize >> 14) + (self.sub_net as usize >> 7) + (self.universe as usize)
    }
}

// Appends a Nul terminated ASCII string truncated (or padded) to N bytes
fn put_padded_str<const N: usize, B: BufMut>(mut buf: B, input: &str) {
    let mut padded_bytes = [0; N];

    let bytes = input.as_bytes();
    // Truncate to N minus 1 to leave 1 byte for the NUL character
    let truncated_bytes = if bytes.len() > N - 1 {
        &bytes[..N - 1]
    } else {
        &bytes[..]
    };

    // Put the truncated bytes into the padded buffer - this guarentees that the length is always N
    (&mut padded_bytes[..]).put_slice(truncated_bytes);

    buf.put_slice(&padded_bytes);
}

#[derive(Debug)]
pub struct Poll {
    pub flags: u8,
    pub min_diagnostic_priority: u8,
    pub target_port_addresses: RangeInclusive<u16>,
}

fn parse_poll<'a>(s: &'a [u8]) -> Result<Poll, Error<'a>> {
    let (s, flags) = number::u8(s)?;
    let (s, min_diagnostic_priority) = number::u8(s)?;

    let target_port_addresses = if !s.is_empty() {
        let (s, target_port_top): (&'a [u8], u16) = be_u16(s)?;
        let (_s, target_port_bottom): (&'a [u8], u16) = be_u16(s)?;

        target_port_top..=target_port_bottom
    } else {
        0..=u16::MAX
    };

    Ok(Poll {
        flags,
        min_diagnostic_priority,
        target_port_addresses,
    })
}

#[derive(Debug)]
pub struct Command<'a> {
    pub esta_manufacturer_code: ESTAManufacturerCode,
    pub data: &'a [u8],
}

fn parse_command<'a>(s: &'a [u8]) -> Result<Command<'a>, Error<'a>> {
    let (s, esta_manufacturer_code) = parse_esta_manufacturer_code(s)?;
    let (s, length): (&'a [u8], u16) = le_u16(s)?;

    let data = &s[..length as usize];

    Ok(Command {
        esta_manufacturer_code,
        data,
    })
}

#[derive(Debug)]
pub struct Dmx<'a> {
    /// The sequence number is used to ensure that
    /// ArtDmx packets are used in the correct order.
    /// When Art-Net is carried over a medium such as
    /// the Internet, it is possible that ArtDmx packets
    /// will reach the receiver out of order.
    ///
    /// This field is incremented in the range 0x01 to
    /// 0xff to allow the receiving node to re-sequence
    /// packets.
    ///
    /// The Sequence field is set to 0x00 to disable this
    /// feature.
    pub sequence: u8,
    /// The physical input port from which DMX512
    /// data was input. This field is used by the
    /// receiving device to discriminate between
    /// packets with identical Port-Address that have
    /// been generated by different input ports and so
    /// need to be merged.
    pub physical: u8,
    ///  one of the 32,768 possible addresses to which a DMX frame can be
    /// directed. The Port-Address is a 15-bit number composed of Net+Sub-Net+Universe.
    ///
    /// Bits:
    ///     | 15 | 8-14 | 4-7    | 0-3      |
    ///     | 0  | Net  | SubNet | Universe |
    pub port_address: PortAddress,
    pub data: &'a [u8],
}

fn parse_dmx<'a>(s: &'a [u8]) -> Result<Dmx<'a>, Error<'a>> {
    let (s, sequence) = number::u8(s)?;
    let (s, physical) = number::u8(s)?;
    let (s, port_address) = parse_port_address(s)?;

    let (s, length): (&'a [u8], u16) = be_u16(s)?;

    let data = &s[..length as usize];

    Ok(Dmx {
        sequence,
        physical,
        port_address,
        data,
    })
}

fn parse_sync<'a>(s: &'a [u8]) -> Result<(), Error<'a>> {
    let (s, _aux1) = number::u8(s)?;
    let (_s, _aux2) = number::u8(s)?;

    Ok(())
}
