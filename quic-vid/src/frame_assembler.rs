use crate::media::MediaDatagram;
use uuid::Uuid;

#[derive(Debug)]
pub struct PartialFrame {
    session_id: Uuid,
    frame_id: u64,
    sent_at_ms: u64,
    chunk_count: u16,
    chunks: Vec<Option<Vec<u8>>>,
}

impl PartialFrame {
    pub fn new(media: &MediaDatagram) -> Self {
        Self {
            session_id: media.session_id,
            frame_id: media.frame_id,
            sent_at_ms: media.sent_at_ms,
            chunk_count: media.chunk_count,
            chunks: vec![None; usize::from(media.chunk_count)],
        }
    }

    pub fn insert(&mut self, media: MediaDatagram) -> anyhow::Result<()> {
        self.validate_metadata(&media)?;

        let index = usize::from(media.chunk_index);

        if index >= self.chunks.len() {
            anyhow::bail!(
                "chunk index {} is outside frame chunk count {}",
                media.chunk_index,
                self.chunk_count
            );
        }

        if self.chunks[index].is_none() {
            self.chunks[index] = Some(media.payload);
        }

        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        self.chunks.iter().all(Option::is_some)
    }

    pub fn reassemble(&self) -> anyhow::Result<Vec<u8>> {
        if !self.is_complete() {
            anyhow::bail!("frame {} is incomplete", self.frame_id);
        }

        let total_size: usize = self
            .chunks
            .iter()
            .filter_map(|chunk| chunk.as_ref())
            .map(Vec::len)
            .sum();

        let mut frame = Vec::with_capacity(total_size);

        for chunk in &self.chunks {
            frame.extend_from_slice(chunk.as_ref().expect("frame completeness checked above"));
        }

        Ok(frame)
    }

    fn validate_metadata(&self, media: &MediaDatagram) -> anyhow::Result<()> {
        if media.session_id != self.session_id {
            anyhow::bail!(
                "chunk session mismatch: expected {}, got {}",
                self.session_id,
                media.session_id
            );
        }

        if media.frame_id != self.frame_id {
            anyhow::bail!(
                "chunk frame mismatch: expected {}, got {}",
                self.frame_id,
                media.frame_id
            );
        }

        if media.sent_at_ms != self.sent_at_ms {
            anyhow::bail!(
                "chunk timestamp mismatch: expected {}, got {}",
                self.sent_at_ms,
                media.sent_at_ms
            );
        }

        if media.chunk_count != self.chunk_count {
            anyhow::bail!(
                "chunk count mismatch: expected {}, got {}",
                self.chunk_count,
                media.chunk_count
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(
        session_id: Uuid,
        frame_id: u64,
        chunk_index: u16,
        chunk_count: u16,
        payload: &[u8],
    ) -> MediaDatagram {
        MediaDatagram {
            session_id,
            frame_id,
            sent_at_ms: 123_456,
            chunk_index,
            chunk_count,
            payload: payload.to_vec(),
        }
    }

    #[test]
    fn single_chunk_frame_completes() {
        let session_id = Uuid::new_v4();
        let media = chunk(session_id, 42, 0, 1, &[1, 2, 3]);

        let mut frame = PartialFrame::new(&media);

        assert!(!frame.is_complete());

        frame.insert(media).unwrap();

        assert!(frame.is_complete());
        assert_eq!(frame.reassemble().unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn incomplete_frame_cannot_be_reassembled() {
        let session_id = Uuid::new_v4();

        let first = chunk(session_id, 42, 0, 2, &[1, 2]);

        let mut frame = PartialFrame::new(&first);
        frame.insert(first).unwrap();

        assert!(!frame.is_complete());
        assert!(frame.reassemble().is_err());
    }

    #[test]
    fn out_of_order_chunks_reassemble_in_index_order() {
        let session_id = Uuid::new_v4();

        let first = chunk(session_id, 42, 0, 3, &[1, 2]);

        let second = chunk(session_id, 42, 1, 3, &[3, 4]);

        let third = chunk(session_id, 42, 2, 3, &[5, 6]);

        let mut frame = PartialFrame::new(&first);

        frame.insert(third).unwrap();
        frame.insert(first).unwrap();
        frame.insert(second).unwrap();

        assert!(frame.is_complete());

        assert_eq!(frame.reassemble().unwrap(), vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn duplicate_chunk_does_not_corrupt_frame() {
        let session_id = Uuid::new_v4();

        let first = chunk(session_id, 42, 0, 2, &[1, 2]);

        let duplicate = chunk(session_id, 42, 0, 2, &[9, 9]);

        let second = chunk(session_id, 42, 1, 2, &[3, 4]);

        let mut frame = PartialFrame::new(&first);

        frame.insert(first).unwrap();
        frame.insert(duplicate).unwrap();
        frame.insert(second).unwrap();

        assert_eq!(frame.reassemble().unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn different_frame_id_is_rejected() {
        let session_id = Uuid::new_v4();

        let first = chunk(session_id, 42, 0, 2, &[1]);

        let wrong = chunk(session_id, 43, 1, 2, &[2]);

        let mut frame = PartialFrame::new(&first);

        frame.insert(first).unwrap();

        assert!(frame.insert(wrong).is_err());
    }

    #[test]
    fn inconsistent_chunk_count_is_rejected() {
        let session_id = Uuid::new_v4();

        let first = chunk(session_id, 42, 0, 2, &[1]);

        let inconsistent = chunk(session_id, 42, 1, 3, &[2]);

        let mut frame = PartialFrame::new(&first);

        frame.insert(first).unwrap();

        assert!(frame.insert(inconsistent).is_err());
    }

    #[test]
    fn different_session_is_rejected() {
        let first = chunk(Uuid::new_v4(), 42, 0, 2, &[1]);

        let wrong = chunk(Uuid::new_v4(), 42, 1, 2, &[2]);

        let mut frame = PartialFrame::new(&first);

        frame.insert(first).unwrap();

        assert!(frame.insert(wrong).is_err());
    }

    #[test]
    fn inconsistent_timestamp_is_rejected() {
        let session_id = Uuid::new_v4();

        let first = chunk(session_id, 42, 0, 2, &[1]);

        let mut wrong = chunk(session_id, 42, 1, 2, &[2]);

        wrong.sent_at_ms = 999_999;

        let mut frame = PartialFrame::new(&first);

        frame.insert(first).unwrap();

        assert!(frame.insert(wrong).is_err());
    }

    #[test]
    fn out_of_range_chunk_index_is_rejected() {
        let session_id = Uuid::new_v4();

        let first = chunk(session_id, 42, 0, 2, &[1]);

        let invalid = chunk(session_id, 42, 2, 2, &[2]);

        let mut frame = PartialFrame::new(&first);

        frame.insert(first).unwrap();

        assert!(frame.insert(invalid).is_err());
    }
}
