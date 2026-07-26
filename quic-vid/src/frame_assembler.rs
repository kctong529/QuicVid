use crate::media::MediaDatagram;
use std::collections::HashMap;
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

#[derive(Debug, PartialEq, Eq)]
pub struct CompletedFrame {
    pub session_id: Uuid,
    pub frame_id: u64,
    pub sent_at_ms: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct FrameAssembler {
    frames: HashMap<u64, PartialFrame>,
}

impl FrameAssembler {
    pub fn push(&mut self, media: MediaDatagram) -> anyhow::Result<Option<CompletedFrame>> {
        let frame_id = media.frame_id;

        let complete = {
            let partial = self
                .frames
                .entry(frame_id)
                .or_insert_with(|| PartialFrame::new(&media));

            partial.insert(media)?;
            partial.is_complete()
        };

        if !complete {
            return Ok(None);
        }

        let partial = self
            .frames
            .remove(&frame_id)
            .expect("completed frame must still exist");

        let bytes = partial.reassemble()?;

        Ok(Some(CompletedFrame {
            session_id: partial.session_id,
            frame_id: partial.frame_id,
            sent_at_ms: partial.sent_at_ms,
            bytes,
        }))
    }

    pub fn incomplete_frame_count(&self) -> usize {
        self.frames.len()
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

    #[test]
    fn assembler_emits_completed_frame() {
        let session_id = Uuid::new_v4();

        let first = chunk(session_id, 42, 0, 2, &[1, 2]);

        let second = chunk(session_id, 42, 1, 2, &[3, 4]);

        let mut assembler = FrameAssembler::default();

        assert!(assembler.push(first).unwrap().is_none());

        let completed = assembler.push(second).unwrap().unwrap();

        assert_eq!(completed.session_id, session_id);
        assert_eq!(completed.frame_id, 42);
        assert_eq!(completed.sent_at_ms, 123_456);
        assert_eq!(completed.bytes, vec![1, 2, 3, 4]);

        assert_eq!(assembler.incomplete_frame_count(), 0);
    }

    #[test]
    fn assembler_handles_interleaved_frames() {
        let session_id = Uuid::new_v4();

        let frame_10_chunk_0 = chunk(session_id, 10, 0, 2, &[1]);

        let frame_11_chunk_0 = chunk(session_id, 11, 0, 2, &[3]);

        let frame_10_chunk_1 = chunk(session_id, 10, 1, 2, &[2]);

        let frame_11_chunk_1 = chunk(session_id, 11, 1, 2, &[4]);

        let mut assembler = FrameAssembler::default();

        assert!(assembler.push(frame_10_chunk_0).unwrap().is_none());

        assert!(assembler.push(frame_11_chunk_0).unwrap().is_none());

        assert_eq!(assembler.incomplete_frame_count(), 2);

        let frame_10 = assembler.push(frame_10_chunk_1).unwrap().unwrap();

        assert_eq!(frame_10.frame_id, 10);
        assert_eq!(frame_10.bytes, vec![1, 2]);

        assert_eq!(assembler.incomplete_frame_count(), 1);

        let frame_11 = assembler.push(frame_11_chunk_1).unwrap().unwrap();

        assert_eq!(frame_11.frame_id, 11);
        assert_eq!(frame_11.bytes, vec![3, 4]);

        assert_eq!(assembler.incomplete_frame_count(), 0);
    }

    #[test]
    fn assembler_handles_out_of_order_chunks() {
        let session_id = Uuid::new_v4();

        let chunk_0 = chunk(session_id, 42, 0, 3, &[1]);

        let chunk_1 = chunk(session_id, 42, 1, 3, &[2]);

        let chunk_2 = chunk(session_id, 42, 2, 3, &[3]);

        let mut assembler = FrameAssembler::default();

        assert!(assembler.push(chunk_2).unwrap().is_none());
        assert!(assembler.push(chunk_0).unwrap().is_none());

        let completed = assembler.push(chunk_1).unwrap().unwrap();

        assert_eq!(completed.bytes, vec![1, 2, 3]);
    }

    #[test]
    fn assembler_tolerates_duplicate_chunk_before_completion() {
        let session_id = Uuid::new_v4();

        let first = chunk(session_id, 42, 0, 2, &[1]);

        let duplicate = chunk(session_id, 42, 0, 2, &[9]);

        let second = chunk(session_id, 42, 1, 2, &[2]);

        let mut assembler = FrameAssembler::default();

        assert!(assembler.push(first).unwrap().is_none());
        assert!(assembler.push(duplicate).unwrap().is_none());

        let completed = assembler.push(second).unwrap().unwrap();

        assert_eq!(completed.bytes, vec![1, 2]);
    }

    #[test]
    fn assembler_rejects_inconsistent_frame_metadata() {
        let session_id = Uuid::new_v4();

        let first = chunk(session_id, 42, 0, 2, &[1]);

        let mut inconsistent = chunk(session_id, 42, 1, 2, &[2]);

        inconsistent.sent_at_ms = 999_999;

        let mut assembler = FrameAssembler::default();

        assert!(assembler.push(first).unwrap().is_none());

        assert!(assembler.push(inconsistent).is_err());
    }
}
