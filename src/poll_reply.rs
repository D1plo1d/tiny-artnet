use bytes::BufMut;

use crate::put_esta_manufacturer_code;

const OP_POLL_REPLY: u16 = 0x2100;

#[derive(Debug)]
pub struct PollReply<'a> {
    pub ip_address: &'a [u8; 4],
    pub port: u16,
    pub firmware_version: u16,
    // Note: I am less sure about how the following fields work in practice. If there are ergonomic improvements to be had I am open to Pull Requests:
    /// Bits 14-8 of the 15 bit Port-Address are encoded into the bottom 7 bits of this field.
    pub net_switch: u8,
    /// Bits 7-4 of the 15 bit Port-Address are encoded into the bottom 4 bits of this field.
    pub sub_switch: u8,
    /// The Oem word describes the equipment vendor and the feature set available. Bit 15 high indicates extended features available.
    pub oem: u16,
    /// The firmware version of the User Bios Extension Area (UBEA) If the UBEA is not programmed, this field contains zero.
    pub ubea_version: u8,
    /// General Status register containing bit fields as follows:
    /// 7-6 Indicator state.
    ///     00 Indicator state unknown.
    ///     01 Indicators in Locate / Identify Mode.
    ///     10 Indicators in Mute Mode.
    ///     11 Indicators in Normal Mode.
    /// 5-4 Port-Address Programming Authority
    ///     00 Port-Address Programming Authority unknown.
    ///     01 All Port-Address set by front panel controls.
    ///     10 All or part of Port-Address programmed by network or Web browser.
    ///     11 Not used.
    /// 3 Not implemented, transmit as zero, receivers do not test.
    /// 2
    ///     0 = Normal firmware boot (from flash). Nodes that do not support dual boot, clear this field to zero.
    ///     1 = Booted from ROM.
    /// 1
    ///     0 = Not capable of Remote Device Management (RDM).
    ///     1 = Capable of Remote Device Management (RDM).
    /// 0
    ///     0 = UBEA not present or corrupt
    ///     1 = UBEA present
    pub status1: u8,
    pub esta_manufacturer_code: crate::ESTAManufacturerCode,
    /// Note: The spec specifies ASCII characters only
    pub short_name: &'a str,
    /// Note: The spec specifies ASCII characters only
    pub long_name: &'a str,
    /// Note: The spec specifies ASCII characters only
    pub node_report: &'a str,
    pub num_ports: u16,
    pub port_types: &'a [u8; 4],
    pub good_input: &'a [u8; 4],
    pub good_output_a: &'a [u8; 4],
    pub swin: &'a [u8; 4],
    pub swout: &'a [u8; 4],
    pub acn_priority: u8,
    pub sw_macro: u8,
    pub sw_remote: u8,
    pub style: u8,
    pub mac_address: &'a [u8; 6],
    pub bind_ip_address: &'a [u8; 4],
    pub bind_index: u8,
    pub status2: u8,
    pub good_output_b: &'a [u8; 4],
    pub status3: u8,
    /// RDMnet & LLRP Default Responder UID
    pub default_responder_uid: &'a [u8; 6],
}

impl<'a> Default for PollReply<'a> {
    fn default() -> Self {
        Self {
            ip_address: crate::DEFAULT_4_BYTES,
            port: Default::default(),
            firmware_version: Default::default(),
            net_switch: Default::default(),
            sub_switch: Default::default(),
            oem: Default::default(),
            ubea_version: Default::default(),
            status1: 0b1100_0000, // Indicator Mode: Normal
            esta_manufacturer_code: Default::default(),
            short_name: Default::default(),
            long_name: Default::default(),
            node_report: Default::default(),
            num_ports: Default::default(),
            port_types: crate::DEFAULT_4_BYTES,
            good_input: crate::DEFAULT_4_BYTES,
            good_output_a: crate::DEFAULT_4_BYTES,
            swin: crate::DEFAULT_4_BYTES,
            swout: crate::DEFAULT_4_BYTES,
            acn_priority: Default::default(),
            sw_macro: Default::default(),
            sw_remote: Default::default(),
            style: Default::default(),
            mac_address: crate::DEFAULT_6_BYTES,
            bind_ip_address: crate::DEFAULT_4_BYTES,
            bind_index: Default::default(),
            status2: Default::default(),
            good_output_b: crate::DEFAULT_4_BYTES,
            status3: Default::default(),
            default_responder_uid: crate::DEFAULT_6_BYTES,
        }
    }
}

impl<'a> PollReply<'a> {
    /// Serializes the PollReply into the provided buffer.
    ///
    /// Note: short name, long name and report will be truncated to 18, 64, and 64 bytes respectively
    pub fn serialize(&self, mut buf: &mut [u8]) -> usize {
        let initial_buf_len = buf.len();

        buf.put_slice(crate::ID);
        buf.put_u16_le(OP_POLL_REPLY);
        buf.put_slice(self.ip_address);
        buf.put_u16_le(self.port);
        buf.put_u16(self.firmware_version);
        buf.put_u8(self.net_switch);
        buf.put_u8(self.sub_switch);
        buf.put_u16(self.oem);
        buf.put_u8(self.ubea_version);
        buf.put_u8(self.status1);
        put_esta_manufacturer_code(&mut buf, &self.esta_manufacturer_code);

        crate::put_padded_str::<18, _>(&mut buf, &self.short_name);
        crate::put_padded_str::<64, _>(&mut buf, &self.long_name);
        crate::put_padded_str::<64, _>(&mut buf, &self.node_report);

        buf.put_u16(self.num_ports);
        buf.put_slice(self.port_types);
        buf.put_slice(self.good_input);
        buf.put_slice(self.good_output_a);
        buf.put_slice(self.swin);
        buf.put_slice(self.swout);
        buf.put_u8(self.acn_priority);
        buf.put_u8(self.sw_macro);
        buf.put_u8(self.sw_remote);
        // Spare
        buf.put_slice(&[0u8; 3]);
        buf.put_u8(self.style);
        buf.put_slice(self.mac_address);
        buf.put_slice(self.bind_ip_address);
        buf.put_u8(self.bind_index);
        buf.put_u8(self.status2);
        buf.put_slice(self.good_output_b);
        buf.put_u8(self.status3);
        buf.put_slice(self.default_responder_uid);
        // Filler
        buf.put_slice(&[0u8; 15]);

        return initial_buf_len - buf.len();
    }
}

// TODO: Poll Reply Parser
// pub fn from_str<'a>(s: &'a [u8]) -> Result<PollReply<'a>, crate::Error<'a>> {
// }
