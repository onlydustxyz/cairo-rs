#![allow(dead_code)]
#![cfg_attr(not(feature = "std"), no_std)]

use nom::error::Error;
use nom::error::ErrorKind;
use nom::error::ParseError;
use nom::Err;
use nom::IResult;

/// A parser similar to `nom::bytes::complete::take_until()`, except that this
/// one does not stop at balanced opening and closing tags. It is designed to
/// work inside the `nom::sequence::delimited()` parser.
///
/// # Basic usage
/// ```
/// use nom::bytes::complete::tag;
/// use nom::sequence::delimited;
/// use cairo_take_until_unbalanced::take_until_unbalanced;
///
/// let mut parser = delimited(tag("<"), take_until_unbalanced('<', '>'), tag(">"));
/// assert_eq!(parser("<<inside>inside>abc"), Ok(("abc", "<inside>inside")));
/// ```
/// It skips nested brackets until it finds an extra unbalanced closing bracket. Escaped brackets
/// like `\<` and `\>` are not considered as brackets and are not counted. This function is
/// very similar to `nom::bytes::complete::take_until(">")`, except it also takes nested brackets.
/// NOTE: trimmed down from https://docs.rs/parse-hyperlinks to fix a pending out-of-bounds access.
pub fn take_until_unbalanced(
    opening_bracket: char,
    closing_bracket: char,
) -> impl Fn(&str) -> IResult<&str, &str> {
    move |i: &str| {
        let mut index = 0;
        let mut bracket_counter = 0;
        while let Some(n) = &i
            .get(index..)
            .ok_or_else(|| Err::Error(Error::from_error_kind(i, ErrorKind::TakeUntil)))?
            .find(&[opening_bracket, closing_bracket, '\\'][..])
        {
            index += n;
            let mut it = i
                .get(index..)
                .ok_or_else(|| Err::Error(Error::from_error_kind(i, ErrorKind::TakeUntil)))?
                .chars();
            match it.next().unwrap_or_default() {
                c if c == '\\' => {
                    // Skip the escape char `\`.
                    index += '\\'.len_utf8();
                    // Skip also the following char.
                    let c = it.next().unwrap_or_default();
                    index += c.len_utf8();
                }
                c if c == opening_bracket => {
                    bracket_counter += 1;
                    index += opening_bracket.len_utf8();
                }
                c if c == closing_bracket => {
                    // Closing bracket.
                    bracket_counter -= 1;
                    index += closing_bracket.len_utf8();
                }
                // Can not happen.
                _ => unreachable!(),
            };
            // We found the unmatched closing bracket.
            if bracket_counter == -1 {
                // We do not consume it.
                index -= closing_bracket.len_utf8();
                let remaining = i
                    .get(index..)
                    .ok_or_else(|| Err::Error(Error::from_error_kind(i, ErrorKind::TakeUntil)))?;
                let matching = i
                    .get(0..index)
                    .ok_or_else(|| Err::Error(Error::from_error_kind(i, ErrorKind::TakeUntil)))?;
                return Ok((remaining, matching));
            };
        }

        if bracket_counter == 0 {
            Ok(("", i))
        } else {
            Err(Err::Error(Error::from_error_kind(i, ErrorKind::TakeUntil)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_take_until_unmatched() {
        assert_eq!(take_until_unbalanced('(', ')')("abc"), Ok(("", "abc")));
        assert_eq!(
            take_until_unbalanced('(', ')')("url)abc"),
            Ok((")abc", "url"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u()rl)abc"),
            Ok((")abc", "u()rl"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())rl)abc"),
            Ok((")abc", "u(())rl"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())r()l)abc"),
            Ok((")abc", "u(())r()l"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())r()labc"),
            Ok(("", "u(())r()labc"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')(r#"u\((\))r()labc"#),
            Ok(("", r#"u\((\))r()labc"#))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())r(labc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "u(())r(labc",
                ErrorKind::TakeUntil
            )))
        );
        assert_eq!(
            take_until_unbalanced('€', 'ü')("€uü€€üürlüabc"),
            Ok(("üabc", "€uü€€üürl"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())r()labc\\"),
            Err(nom::Err::Error(nom::error::Error::new(
                "u(())r()labc\\",
                ErrorKind::TakeUntil
            )))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u\\rl)abc"),
            Ok((")abc", "u\\rl"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u\\\\rl)abc"),
            Ok((")abc", "u\\\\rl"))
        );
        // 'µ' used to check for escaped multi-byte character
        assert_eq!(
            take_until_unbalanced('(', ')')("u\\µrl)"),
            Ok((")", "u\\µrl"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u\\µ)rl"),
            Ok((")rl", "u\\µ"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("urlabc\\"),
            Err(nom::Err::Error(nom::error::Error::new(
                "urlabc\\",
                ErrorKind::TakeUntil
            )))
        );
        assert_eq!(take_until_unbalanced('(', ')')("abc"), Ok(("", "abc")));
        assert_eq!(
            take_until_unbalanced('(', ')')("(abc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "(abc",
                ErrorKind::TakeUntil
            )))
        );
    }
}
