use uuid::Uuid;

pub const MEDIA_DATAGRAM_TYPE: u8 = 0x01;
pub const MEDIA_HEADER_SIZE: usize = 37;

fn required_chunk_count(frame_size: usize, max_chunk_payload: usize) -> anyhow::Result<u16> {
    if max_chunk_payload == 0 {
        anyhow::bail!("maximum chunk payload must be greater than zero");
    }

    if frame_size == 0 {
        anyhow::bail!("media frame must not be empty");
    }

    let chunk_count = frame_size.div_ceil(max_chunk_payload);

    chunk_count.try_into().map_err(|_| {
        anyhow::anyhow!(
            "frame requires {} chunks, exceeding maximum supported chunk count {}",
            chunk_count,
            u16::MAX
        )
    })
}

pub fn fragment_frame(
    session_id: Uuid,
    frame_id: u64,
    sent_at_ms: u64,
    frame: &[u8],
    max_chunk_payload: usize,
) -> anyhow::Result<Vec<MediaDatagram>> {
    let chunk_count = required_chunk_count(frame.len(), max_chunk_payload)?;

    let chunks = frame
        .chunks(max_chunk_payload)
        .enumerate()
        .map(|(index, payload)| MediaDatagram {
            session_id,
            frame_id,
            sent_at_ms,
            chunk_index: index as u16,
            chunk_count,
            payload: payload.to_vec(),
        })
        .collect();

    Ok(chunks)
}

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

    #[test]
    fn one_byte_frame_requires_one_chunk() {
        assert_eq!(required_chunk_count(1, 1251).unwrap(), 1);
    }

    #[test]
    fn frame_at_payload_limit_requires_one_chunk() {
        assert_eq!(required_chunk_count(1251, 1251).unwrap(), 1);
    }

    #[test]
    fn frame_over_payload_limit_requires_two_chunks() {
        assert_eq!(required_chunk_count(1252, 1251).unwrap(), 2);
    }

    #[test]
    fn exact_two_chunk_frame_requires_two_chunks() {
        assert_eq!(required_chunk_count(2502, 1251).unwrap(), 2);
    }

    #[test]
    fn frame_over_two_chunk_boundary_requires_three_chunks() {
        assert_eq!(required_chunk_count(2503, 1251).unwrap(), 3);
    }

    #[test]
    fn measured_jpeg_requires_twelve_chunks() {
        assert_eq!(required_chunk_count(13_839, 1251).unwrap(), 12);
    }

    #[test]
    fn zero_chunk_payload_is_rejected() {
        assert!(required_chunk_count(100, 0).is_err());
    }

    #[test]
    fn empty_frame_is_rejected() {
        assert!(required_chunk_count(0, 1251).is_err());
    }

    #[test]
    fn chunk_count_larger_than_u16_is_rejected() {
        let frame_size = usize::from(u16::MAX) + 1;

        assert!(required_chunk_count(frame_size, 1).is_err());
    }

    #[test]
    fn fragment_small_frame_into_single_chunk() {
        let session_id = Uuid::new_v4();
        let frame = vec![1, 2, 3, 4];

        let chunks = fragment_frame(session_id, 42, 123_456, &frame, 1251).unwrap();

        assert_eq!(chunks.len(), 1);

        let chunk = &chunks[0];

        assert_eq!(chunk.session_id, session_id);
        assert_eq!(chunk.frame_id, 42);
        assert_eq!(chunk.sent_at_ms, 123_456);
        assert_eq!(chunk.chunk_index, 0);
        assert_eq!(chunk.chunk_count, 1);
        assert_eq!(chunk.payload, frame);
    }

    #[test]
    fn fragment_frame_over_limit_into_two_chunks() {
        let frame = vec![0xaa; 1252];

        let chunks = fragment_frame(Uuid::new_v4(), 1, 1000, &frame, 1251).unwrap();

        assert_eq!(chunks.len(), 2);

        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[0].chunk_count, 2);
        assert_eq!(chunks[0].payload.len(), 1251);

        assert_eq!(chunks[1].chunk_index, 1);
        assert_eq!(chunks[1].chunk_count, 2);
        assert_eq!(chunks[1].payload.len(), 1);
    }

    #[test]
    fn fragment_exact_multiple_has_full_final_chunk() {
        let frame = vec![0xbb; 2502];

        let chunks = fragment_frame(Uuid::new_v4(), 2, 2000, &frame, 1251).unwrap();

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].payload.len(), 1251);
        assert_eq!(chunks[1].payload.len(), 1251);
    }

    #[test]
    fn fragmented_chunks_preserve_frame_metadata() {
        let session_id = Uuid::new_v4();

        let chunks = fragment_frame(session_id, 42, 987_654, &vec![0xcc; 3000], 1000).unwrap();

        assert_eq!(chunks.len(), 3);

        for (index, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.session_id, session_id);
            assert_eq!(chunk.frame_id, 42);
            assert_eq!(chunk.sent_at_ms, 987_654);
            assert_eq!(chunk.chunk_index, index as u16);
            assert_eq!(chunk.chunk_count, 3);
        }
    }

    #[test]
    fn fragmented_payloads_reconstruct_original_frame() {
        let frame: Vec<u8> = (0..10_000).map(|value| (value % 256) as u8).collect();

        let chunks = fragment_frame(Uuid::new_v4(), 42, 123_456, &frame, 1251).unwrap();

        let reconstructed: Vec<u8> = chunks
            .iter()
            .flat_map(|chunk| chunk.payload.iter().copied())
            .collect();

        assert_eq!(reconstructed, frame);
    }

    #[test]
    fn measured_jpeg_fragments_into_twelve_chunks() {
        let frame = vec![0xdd; 13_839];

        let chunks = fragment_frame(Uuid::new_v4(), 0, 0, &frame, 1251).unwrap();

        assert_eq!(chunks.len(), 12);
        assert_eq!(chunks.last().unwrap().payload.len(), 78);
    }

    #[test]
    fn fragment_frame_rejects_empty_frame() {
        assert!(fragment_frame(Uuid::new_v4(), 0, 0, &[], 1251,).is_err());
    }

    #[test]
    fn fragment_frame_rejects_zero_payload_limit() {
        assert!(fragment_frame(Uuid::new_v4(), 0, 0, &[1, 2, 3], 0,).is_err());
    }
}
