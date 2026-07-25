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
}
