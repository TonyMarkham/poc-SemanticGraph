pub const DEFAULT_TEXT_CAP: usize = 512;

pub fn sanitize_transcript_text(text: &str, max_len: usize) -> String {
    let mut sanitized = String::new();

    for character in text.chars() {
        if sanitized.len() >= max_len {
            break;
        }

        let replacement = if character.is_ascii_control()
            && character != '\t'
            && character != '\n'
            && character != '\r'
        {
            ' '
        } else {
            character
        };

        if sanitized.len() + replacement.len_utf8() > max_len {
            break;
        }
        sanitized.push(replacement);
    }

    sanitized
}

#[cfg(test)]
mod tests {
    use crate::sanitize::sanitize_transcript_text;

    #[test]
    fn replaces_control_characters_except_common_whitespace() {
        let text = "ok\u{0000}\tline\nnext\rend\u{001f}";

        assert_eq!("ok \tline\nnext\rend ", sanitize_transcript_text(text, 100));
    }

    #[test]
    fn caps_text_without_splitting_multibyte_characters() {
        assert_eq!("abcd", sanitize_transcript_text("abcdé", 4));
    }
}
