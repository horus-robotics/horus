use super::checksum::Checksum;
use byteorder::{ByteOrder, LittleEndian};
use rpos_drv::{Message, ProtocolDecoder, ProtocolEncoder};
use std::cmp::min;
use std::io::Write;

use super::errors::*;

const RPLIDAR_CMD_SYNC_BYTE: u8 = 0xA5;
const RPLIDAR_CMDFLAG_HAS_PAYLOAD: u8 = 0x80;

const RPLIDAR_ANS_SYNC_BYTES: [u8; 2] = [0xA5, 0x5A];

const RPLIDAR_ANS_PKTFLAG_LOOP: u8 = 0x1;

const RPLIDAR_ANS_HEADER_SIZE_MASK: u32 = 0x3FFFFFFF;
const RPLIDAR_ANS_HEADER_SUBTYPE_SHIFT: usize = 30;

/// The size of RPLIDAR protocol answer header (not including the two sync bytes)
const RPLIDAR_ANS_HEADER_SIZE: usize = 5;

#[derive(Debug, Clone, PartialEq)]
enum DecodeStatus {
    WaitSyncByte(usize),
    WaitAnsHeader,
    ReceiveResponse,
}

/// The implementation of RPLIDAR host protocol
#[derive(Debug, Clone, PartialEq)]
pub struct RplidarHostProtocol {
    status: DecodeStatus,
    ans_header: Vec<u8>,
    ans_flag: u8,
    response_size: usize,
    decoding_msg: Message, // decode_buffer: RingByteBuffer
}

impl RplidarHostProtocol {
    pub fn new() -> RplidarHostProtocol {
        RplidarHostProtocol {
            status: DecodeStatus::WaitSyncByte(0),
            ans_header: Vec::new(),
            ans_flag: 0,
            response_size: 0,
            decoding_msg: Message::new(0),
        }
    }

    fn start_wait_sync_bytes(&mut self, sync_byte_index: usize) {
        self.status = DecodeStatus::WaitSyncByte(sync_byte_index);
        self.ans_header.clear();
        self.ans_flag = 0;
        self.response_size = 0;
    }

    fn start_wait_ans_header(&mut self) {
        self.status = DecodeStatus::WaitAnsHeader;
        self.ans_header.clear();
        self.ans_flag = 0;
        self.response_size = 0;
    }

    fn start_receive_response(&mut self) {
        self.status = DecodeStatus::ReceiveResponse;
        self.decoding_msg.data.clear();
    }

    fn decode_sync_bytes(&mut self, buf: &[u8]) -> Result<usize> {
        if let DecodeStatus::WaitSyncByte(i) = self.status {
            if buf[0] != RPLIDAR_ANS_SYNC_BYTES[i] {
                self.start_wait_sync_bytes(0);
                return Ok(1);
            } else if i == RPLIDAR_ANS_SYNC_BYTES.len() - 1 {
                self.start_wait_ans_header();
                return Ok(1);
            } else {
                self.start_wait_sync_bytes(i + 1);
                return Ok(1);
            }
        } else {
            return Err(Error::ProtocolError("sync byte status error".to_owned()));
        }
    }

    fn decode_ans_header(&mut self, buf: &[u8]) -> Result<(usize, Option<Message>)> {
        let bytes_to_read = RPLIDAR_ANS_HEADER_SIZE - self.ans_header.len();
        let bytes_actual_read = min(bytes_to_read, buf.len());
        self.ans_header
            .extend_from_slice(&buf[0..bytes_actual_read]);

        if bytes_to_read == bytes_actual_read {
            self.decode_ans_header_metadata();
            if self.response_size == 0 {
                if (self.ans_flag & RPLIDAR_ANS_PKTFLAG_LOOP) == RPLIDAR_ANS_PKTFLAG_LOOP {
                    return Err(Error::ProtocolError("received loop answer with no response size".to_owned()));
                } else {
                    let answer = Ok((bytes_actual_read, Some(self.decoding_msg.clone())));
                    self.reset_decoder();
                    return answer;
                }
            } else {
                self.start_receive_response();
                return Ok((bytes_actual_read, None));
            }
        } else {
            return Ok((bytes_actual_read, None));
        }
    }

    fn decode_response(&mut self, buf: &[u8]) -> Result<(usize, Option<Message>)> {
        let bytes_to_read = self.response_size - self.decoding_msg.data.len();
        let bytes_actual_read = min(bytes_to_read, buf.len());
        self.decoding_msg
            .data
            .extend_from_slice(&buf[0..bytes_actual_read]);

        if self.decoding_msg.data.len() == self.response_size {
            let answer = Ok((bytes_actual_read, Some(self.decoding_msg.clone())));
            if self.ans_flag & RPLIDAR_ANS_PKTFLAG_LOOP == RPLIDAR_ANS_PKTFLAG_LOOP {
                self.start_receive_response();
            } else {
                self.start_wait_sync_bytes(0);
            }
            return answer;
        } else {
            return Ok((bytes_actual_read, None));
        }
    }

    /// when we finished reading the first five bytes of answer header, we will know the message type and payload size of answer header
    fn decode_ans_header_metadata(&mut self) {
        self.decoding_msg = Message::new(self.ans_header[4]);
        let size_q30_subtype = LittleEndian::read_u32(&self.ans_header[0..4]);
        self.ans_flag = (size_q30_subtype >> RPLIDAR_ANS_HEADER_SUBTYPE_SHIFT) as u8;
        self.response_size = (size_q30_subtype & RPLIDAR_ANS_HEADER_SIZE_MASK) as usize;
    }
}

impl ProtocolDecoder for RplidarHostProtocol {
    /// Decode bytes and return consumed bytes and message
    fn decode(&mut self, buf: &[u8]) -> Result<(usize, Option<Message>)> {
        let mut i = 0;

        while i < buf.len() {
            match self.status {
                DecodeStatus::WaitSyncByte(_) => {
                    i += self.decode_sync_bytes(&buf[i..buf.len()])?;
                }
                DecodeStatus::WaitAnsHeader => {
                    let (read, msg) = self.decode_ans_header(&buf[i..buf.len()])?;
                    if let Some(_) = msg {
                        return Ok((i + read, msg));
                    }
                    i += read;
                }
                DecodeStatus::ReceiveResponse => {
                    let (read, msg) = self.decode_response(&buf[i..buf.len()])?;
                    if let Some(_) = msg {
                        return Ok((i + read, msg));
                    }
                    i += read;
                }
            }
        }

        return Ok((i, None));
    }

    /// Reset the decoder status
    fn reset_decoder(&mut self) {
        self.start_wait_sync_bytes(0);
    }
}

impl ProtocolEncoder for RplidarHostProtocol {
    /// Encode message into byte array
    /// Always encode commands
    fn encode(&mut self, msg: &Message, bytes: &mut [u8]) -> Result<usize> {
        let estimated_encoded_size = self.estimate_encoded_size(msg)?;

        if estimated_encoded_size > bytes.len() {
            return Err(Error::BufferTooSmall);
        }

        if msg.data.len() > 255 {
            return Err(Error::OperationFail("payload too big".to_owned()));
        }

        let cmd = if msg.data.len() != 0 {
            msg.cmd | RPLIDAR_CMDFLAG_HAS_PAYLOAD
        } else {
            msg.cmd
        };

        bytes[0] = RPLIDAR_CMD_SYNC_BYTE;
        bytes[1] = cmd;

        if msg.data.len() != 0 {
            let mut checksum = Checksum::new();

            checksum.push_slice(&bytes[0..2]);
            checksum.push(msg.data.len() as u8);
            checksum.push_slice(&msg.data);

            bytes[2] = msg.data.len() as u8;
            bytes[3..3 + msg.data.len()].clone_from_slice(&msg.data);
            bytes[3 + msg.data.len()] = checksum.checksum();

            return Ok(4 + msg.data.len());
        } else {
            return Ok(2);
        }
    }

    /// Estimate encoded message size (must be greater than or equal to the actual encoded size)
    fn estimate_encoded_size(&mut self, msg: &Message) -> Result<usize> {
        if msg.data.len() > 255 {
            return Err(Error::OperationFail("payload too big".to_owned()));
        }

        if msg.data.len() > 0 {
            return Ok(4 + msg.data.len());
        } else {
            return Ok(2);
        }
    }

    /// Write message to `std::io::Write` targets
    fn write_to(&mut self, msg: &Message, dest: &mut impl Write) -> Result<usize> {
        let estimated_encoded_size = self.estimate_encoded_size(msg)?;
        let mut buf = vec![0; estimated_encoded_size];
        let encoded_size = self.encode(msg, &mut buf[0..estimated_encoded_size])?;
        return match dest.write_all(&buf[0..encoded_size]) {
            Ok(()) => Ok(encoded_size),
            Err(err) => Err(Error::from(err)),
        };
    }

    /// Reset encoder
    fn reset_encoder(&mut self) {}
}

#[cfg(test)]
mod tests {

    use rpos_drv::{Message, ProtocolEncoder, Result};

    fn encode<T: ProtocolEncoder>(protocol: &mut T, msg: &Message) -> Result<Vec<u8>> {
        let encoded_bytes = protocol.estimate_encoded_size(&msg)?;
        let mut buf = vec![0; encoded_bytes];
        let encoded_bytes = protocol.encode(&msg, &mut buf[0..encoded_bytes])?;
        buf.truncate(encoded_bytes);
        return Ok(buf);
    }

    #[test]
    fn protocol_encode() {
        let mut protocol = super::RplidarHostProtocol::new();

        assert_eq!(
            encode(&mut protocol, &Message::new(0x25))
                .unwrap()
                .as_slice(),
            [0xA5, 0x25]
        );

        assert_eq!(
            encode(&mut protocol, &Message::with_data(0x82, &[0; 5]))
                .unwrap()
                .as_slice(),
            [0xA5, 0x82, 0x05, 0, 0, 0, 0, 0, 0x22]
        );
    }

}
