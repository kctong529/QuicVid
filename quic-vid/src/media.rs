use uuid::Uuid;

pub const MEDIA_DATAGRAM_TYPE: u8 = 0x01;
pub const MEDIA_HEADER_SIZE: usize = 37;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaDatagram {
    pub session_id: Uuid,
    pub frame_id: u64,
    pub sent_at_ms: u64,
    pub chunk_index: u16,
    pub chunk_count: u16,
    pub payload: Vec<u8>,
}

impl MediaDatagram {
    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        if self.chunk_count == 0 {
            anyhow::bail!("chunk_count must be greater than zero");
        }

        if self.chunk_index >= self.chunk_count {
            anyhow::bail!(
                "chunk_index {} must be smaller than chunk_count {}",
                self.chunk_index,
                self.chunk_count
            );
        }

        let mut bytes = Vec::with_capacity(MEDIA_HEADER_SIZE + self.payload.len());

        bytes.push(MEDIA_DATAGRAM_TYPE);
        bytes.extend_from_slice(self.session_id.as_bytes());
        bytes.extend_from_slice(&self.frame_id.to_be_bytes());
        bytes.extend_from_slice(&self.sent_at_ms.to_be_bytes());
        bytes.extend_from_slice(&self.chunk_index.to_be_bytes());
        bytes.extend_from_slice(&self.chunk_count.to_be_bytes());
        bytes.extend_from_slice(&self.payload);

        Ok(bytes)
    }
}

impl MediaDatagram {
    pub fn decode(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() < MEDIA_HEADER_SIZE {
            anyhow::bail!(
                "media datagram too short: expected at least {} bytes, got {}",
                MEDIA_HEADER_SIZE,
                bytes.len()
            );
        }

        if bytes[0] != MEDIA_DATAGRAM_TYPE {
            anyhow::bail!("unexpected media datagram type: {}", bytes[0]);
        }

        let session_id = Uuid::from_slice(&bytes[1..17])?;

        let frame_id = u64::from_be_bytes(bytes[17..25].try_into()?);

        let sent_at_ms = u64::from_be_bytes(bytes[25..33].try_into()?);

        let chunk_index = u16::from_be_bytes(bytes[33..35].try_into()?);

        let chunk_count = u16::from_be_bytes(bytes[35..37].try_into()?);

        if chunk_count == 0 {
            anyhow::bail!("chunk_count must be greater than zero");
        }

        if chunk_index >= chunk_count {
            anyhow::bail!(
                "chunk_index {} must be smaller than chunk_count {}",
                chunk_index,
                chunk_count
            );
        }

        Ok(Self {
            session_id,
            frame_id,
            sent_at_ms,
            chunk_index,
            chunk_count,
            payload: bytes[MEDIA_HEADER_SIZE..].to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_media_datagram() {
        let datagram = MediaDatagram {
            session_id: Uuid::new_v4(),
            frame_id: 42,
            sent_at_ms: 123_456_789,
            chunk_index: 2,
            chunk_count: 5,
            payload: vec![1, 2, 3, 4],
        };

        let encoded = datagram.encode().unwrap();
        let decoded = MediaDatagram::decode(&encoded).unwrap();

        assert_eq!(decoded, datagram);
    }

    #[test]
    fn decode_rejects_short_datagram() {
        let bytes = vec![0; MEDIA_HEADER_SIZE - 1];

        assert!(MediaDatagram::decode(&bytes).is_err());
    }

    #[test]
    fn decode_rejects_wrong_type() {
        let datagram = MediaDatagram {
            session_id: Uuid::new_v4(),
            frame_id: 1,
            sent_at_ms: 1000,
            chunk_index: 0,
            chunk_count: 1,
            payload: vec![],
        };

        let mut encoded = datagram.encode().unwrap();
        encoded[0] = 0xff;

        assert!(MediaDatagram::decode(&encoded).is_err());
    }

    #[test]
    fn encode_rejects_zero_chunk_count() {
        let datagram = MediaDatagram {
            session_id: Uuid::new_v4(),
            frame_id: 1,
            sent_at_ms: 1000,
            chunk_index: 0,
            chunk_count: 0,
            payload: vec![],
        };

        assert!(datagram.encode().is_err());
    }

    #[test]
    fn encode_rejects_out_of_range_chunk_index() {
        let datagram = MediaDatagram {
            session_id: Uuid::new_v4(),
            frame_id: 1,
            sent_at_ms: 1000,
            chunk_index: 2,
            chunk_count: 2,
            payload: vec![],
        };

        assert!(datagram.encode().is_err());
    }

    #[test]
    fn single_chunk_fake_frame_is_valid() {
        let datagram = MediaDatagram {
            session_id: Uuid::new_v4(),
            frame_id: 0,
            sent_at_ms: 0,
            chunk_index: 0,
            chunk_count: 1,
            payload: vec![0x42; 256],
        };

        let decoded = MediaDatagram::decode(&datagram.encode().unwrap()).unwrap();

        assert_eq!(decoded.chunk_index, 0);
        assert_eq!(decoded.chunk_count, 1);
        assert_eq!(decoded.payload.len(), 256);
    }
}
