use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hello {
    pub media_run_id: Uuid,
    pub session_id: Uuid,
}

pub fn hello(media_run_id: Uuid, session_id: Uuid) -> String {
    format!("HELLO {media_run_id} {session_id}\n")
}

pub fn parse_hello(message: &str) -> anyhow::Result<Hello> {
    let mut parts = message.split_whitespace();

    let message_type = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing control message type"))?;

    if message_type != "HELLO" {
        anyhow::bail!("expected HELLO control message, got {message_type:?}");
    }

    let media_run_text = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing HELLO media-run ID"))?;
    let session_text = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing HELLO session ID"))?;

    if parts.next().is_some() {
        anyhow::bail!("unexpected extra fields in HELLO message");
    }

    Ok(Hello {
        media_run_id: Uuid::parse_str(media_run_text)?,
        session_id: Uuid::parse_str(session_text)?,
    })
}

pub fn acknowledgement(media_run_id: Uuid, session_id: Uuid) -> String {
    format!("OK {media_run_id} {session_id}\n")
}

pub fn validate_acknowledgement(
    message: &str,
    expected_media_run_id: Uuid,
    expected_session_id: Uuid,
) -> anyhow::Result<()> {
    let expected = format!("OK {expected_media_run_id} {expected_session_id}");

    if message.trim() != expected {
        anyhow::bail!(
            "unexpected control response: expected {:?}, got {:?}",
            expected,
            message.trim()
        );
    }

    Ok(())
}

pub fn done(media_run_id: Uuid, session_id: Uuid, final_frame_exclusive: u64) -> String {
    format!("DONE {media_run_id} {session_id} {final_frame_exclusive}\n")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Done {
    pub media_run_id: Uuid,
    pub session_id: Uuid,
    pub final_frame_exclusive: u64,
}

pub fn parse_done(message: &str) -> anyhow::Result<Done> {
    let mut parts = message.split_whitespace();

    let message_type = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing control message type"))?;

    if message_type != "DONE" {
        anyhow::bail!("expected DONE control message, got {message_type:?}");
    }

    let media_run_text = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing DONE media-run ID"))?;
    let session_text = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing DONE session ID"))?;
    let final_frame_text = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing DONE final frame"))?;

    if parts.next().is_some() {
        anyhow::bail!("unexpected extra fields in DONE message");
    }

    Ok(Done {
        media_run_id: Uuid::parse_str(media_run_text)?,
        session_id: Uuid::parse_str(session_text)?,
        final_frame_exclusive: final_frame_text.parse::<u64>()?,
    })
}

pub fn done_acknowledgement(media_run_id: Uuid, session_id: Uuid) -> String {
    format!("DONE_OK {media_run_id} {session_id}\n")
}

pub fn validate_done_acknowledgement(
    message: &str,
    expected_media_run_id: Uuid,
    expected_session_id: Uuid,
) -> anyhow::Result<()> {
    let expected = format!("DONE_OK {expected_media_run_id} {expected_session_id}");

    if message.trim() != expected {
        anyhow::bail!(
            "unexpected DONE response: expected {:?}, got {:?}",
            expected,
            message.trim()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_round_trip_preserves_media_run_and_session_ids() {
        let media_run_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        let message = hello(media_run_id, session_id);
        let parsed = parse_hello(&message).unwrap();

        assert_eq!(parsed.media_run_id, media_run_id);
        assert_eq!(parsed.session_id, session_id);
    }

    #[test]
    fn parse_hello_rejects_invalid_uuid() {
        let session_id = Uuid::new_v4();
        let result = parse_hello(&format!("HELLO not-a-uuid {session_id}\n"));

        assert!(result.is_err());
    }

    #[test]
    fn parse_hello_rejects_missing_identity() {
        let media_run_id = Uuid::new_v4();

        assert!(parse_hello(&format!("HELLO {media_run_id}\n")).is_err());
    }

    #[test]
    fn parse_hello_rejects_wrong_message_type() {
        let media_run_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let message = format!("GOODBYE {media_run_id} {session_id}\n");

        assert!(parse_hello(&message).is_err());
    }

    #[test]
    fn parse_hello_rejects_extra_fields() {
        let media_run_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let message = format!("HELLO {media_run_id} {session_id} extra\n");

        assert!(parse_hello(&message).is_err());
    }

    #[test]
    fn acknowledgement_accepts_matching_identities() {
        let media_run_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let response = acknowledgement(media_run_id, session_id);

        assert!(validate_acknowledgement(&response, media_run_id, session_id).is_ok());
    }

    #[test]
    fn acknowledgement_rejects_different_media_run() {
        let expected_media_run = Uuid::new_v4();
        let other_media_run = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let response = acknowledgement(other_media_run, session_id);

        assert!(validate_acknowledgement(&response, expected_media_run, session_id).is_err());
    }

    #[test]
    fn acknowledgement_rejects_different_session() {
        let media_run_id = Uuid::new_v4();
        let expected_session = Uuid::new_v4();
        let other_session = Uuid::new_v4();
        let response = acknowledgement(media_run_id, other_session);

        assert!(validate_acknowledgement(&response, media_run_id, expected_session).is_err());
    }

    #[test]
    fn done_round_trip_preserves_media_run_session_and_timeline() {
        let media_run_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let message = done(media_run_id, session_id, 300);

        let parsed = parse_done(&message).unwrap();

        assert_eq!(parsed.media_run_id, media_run_id);
        assert_eq!(parsed.session_id, session_id);
        assert_eq!(parsed.final_frame_exclusive, 300);
    }

    #[test]
    fn parse_done_rejects_invalid_final_frame() {
        let media_run_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let message = format!("DONE {media_run_id} {session_id} bananas\n");

        assert!(parse_done(&message).is_err());
    }

    #[test]
    fn parse_done_rejects_missing_identity() {
        let media_run_id = Uuid::new_v4();
        let message = format!("DONE {media_run_id} 10\n");

        assert!(parse_done(&message).is_err());
    }

    #[test]
    fn parse_done_rejects_extra_fields() {
        let media_run_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let message = format!("DONE {media_run_id} {session_id} 10 extra\n");

        assert!(parse_done(&message).is_err());
    }

    #[test]
    fn done_acknowledgement_accepts_matching_identities() {
        let media_run_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let response = done_acknowledgement(media_run_id, session_id);

        assert!(validate_done_acknowledgement(&response, media_run_id, session_id).is_ok());
    }

    #[test]
    fn done_acknowledgement_rejects_wrong_media_run() {
        let expected_media_run = Uuid::new_v4();
        let other_media_run = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let response = done_acknowledgement(other_media_run, session_id);

        assert!(validate_done_acknowledgement(&response, expected_media_run, session_id).is_err());
    }

    #[test]
    fn done_acknowledgement_rejects_wrong_session() {
        let media_run_id = Uuid::new_v4();
        let expected_session = Uuid::new_v4();
        let other_session = Uuid::new_v4();
        let response = done_acknowledgement(media_run_id, other_session);

        assert!(validate_done_acknowledgement(&response, media_run_id, expected_session).is_err());
    }
}
