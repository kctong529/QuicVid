use uuid::Uuid;

pub fn hello(session_id: Uuid) -> String {
    format!("HELLO {session_id}\n")
}

pub fn parse_hello(message: &str) -> anyhow::Result<Uuid> {
    let session_text = message
        .trim()
        .strip_prefix("HELLO ")
        .ok_or_else(|| anyhow::anyhow!("invalid control hello: {message:?}"))?;

    Ok(Uuid::parse_str(session_text)?)
}

pub fn acknowledgement(session_id: Uuid) -> String {
    format!("OK {session_id}\n")
}

pub fn validate_acknowledgement(message: &str, expected_session_id: Uuid) -> anyhow::Result<()> {
    let expected = format!("OK {expected_session_id}");

    if message.trim() != expected {
        anyhow::bail!(
            "unexpected control response: expected {:?}, got {:?}",
            expected,
            message.trim()
        );
    }

    Ok(())
}

pub fn done(session_id: Uuid, frame_count: u64) -> String {
    format!("DONE {session_id} {frame_count}\n")
}

pub fn parse_done(message: &str) -> anyhow::Result<(Uuid, u64)> {
    let mut parts = message.split_whitespace();

    let message_type = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing control message type"))?;

    if message_type != "DONE" {
        anyhow::bail!("expected DONE control message, got {message_type:?}");
    }

    let session_text = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing DONE session ID"))?;

    let frame_count_text = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing DONE frame count"))?;

    if parts.next().is_some() {
        anyhow::bail!("unexpected extra fields in DONE message");
    }

    let session_id = Uuid::parse_str(session_text)?;
    let frame_count = frame_count_text.parse::<u64>()?;

    Ok((session_id, frame_count))
}

pub fn done_acknowledgement(session_id: Uuid) -> String {
    format!("DONE_OK {session_id}\n")
}

pub fn validate_done_acknowledgement(
    message: &str,
    expected_session_id: Uuid,
) -> anyhow::Result<()> {
    let expected = format!("DONE_OK {expected_session_id}");

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
    fn hello_round_trip_preserves_session_id() {
        let session_id = Uuid::new_v4();

        let message = hello(session_id);
        let parsed = parse_hello(&message).unwrap();

        assert_eq!(parsed, session_id);
    }

    #[test]
    fn parse_hello_rejects_invalid_uuid() {
        let result = parse_hello("HELLO not-a-uuid\n");

        assert!(result.is_err());
    }

    #[test]
    fn parse_hello_rejects_wrong_message_type() {
        let session_id = Uuid::new_v4();
        let message = format!("GOODBYE {session_id}\n");

        let result = parse_hello(&message);

        assert!(result.is_err());
    }

    #[test]
    fn acknowledgement_accepts_matching_session_id() {
        let session_id = Uuid::new_v4();
        let response = acknowledgement(session_id);

        let result = validate_acknowledgement(&response, session_id);

        assert!(result.is_ok());
    }

    #[test]
    fn acknowledgement_rejects_different_session_id() {
        let expected_session = Uuid::new_v4();
        let other_session = Uuid::new_v4();
        let response = acknowledgement(other_session);

        let result = validate_acknowledgement(&response, expected_session);

        assert!(result.is_err());
    }

    #[test]
    fn done_round_trip_preserves_values() {
        let session_id = Uuid::new_v4();
        let message = done(session_id, 300);

        let (parsed_session, parsed_count) = parse_done(&message).unwrap();

        assert_eq!(parsed_session, session_id);
        assert_eq!(parsed_count, 300);
    }

    #[test]
    fn parse_done_rejects_invalid_frame_count() {
        let session_id = Uuid::new_v4();
        let message = format!("DONE {session_id} bananas\n");

        assert!(parse_done(&message).is_err());
    }

    #[test]
    fn parse_done_rejects_extra_fields() {
        let session_id = Uuid::new_v4();
        let message = format!("DONE {session_id} 10 extra\n");

        assert!(parse_done(&message).is_err());
    }

    #[test]
    fn done_acknowledgement_accepts_matching_session() {
        let session_id = Uuid::new_v4();
        let response = done_acknowledgement(session_id);

        assert!(validate_done_acknowledgement(&response, session_id).is_ok());
    }

    #[test]
    fn done_acknowledgement_rejects_wrong_session() {
        let expected = Uuid::new_v4();
        let other = Uuid::new_v4();
        let response = done_acknowledgement(other);

        assert!(validate_done_acknowledgement(&response, expected).is_err());
    }
}
