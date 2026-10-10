// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::errors::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Encoding {
    Utf8,
    Hex,
    Base64,
    Latin1,
    Ascii,
}

impl Encoding {
    const BASE64_ALPHABET: &'static [u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    fn hex_nibble(digit: u8) -> Result<u8, Error> {
        match digit {
            b'0'..=b'9' => Ok(digit - b'0'),
            b'a'..=b'f' => Ok(digit - b'a' + 10),
            b'A'..=b'F' => Ok(digit - b'A' + 10),
            _ => Err(Error::conversion(format!(
                "agentc:fs: invalid hex digit {:?}",
                char::from(digit)
            ))),
        }
    }

    fn base64_value(character: u8) -> Result<u8, Error> {
        Self::BASE64_ALPHABET
            .iter()
            .position(|candidate| *candidate == character)
            .map(|index| index as u8)
            .ok_or_else(|| {
                Error::conversion(format!(
                    "agentc:fs: invalid base64 character {:?}",
                    char::from(character)
                ))
            })
    }

    pub fn parse(label: &str) -> Result<Self, Error> {
        match label.to_ascii_lowercase().as_str() {
            "utf8" | "utf-8" => Ok(Self::Utf8),
            "hex" => Ok(Self::Hex),
            "base64" => Ok(Self::Base64),
            "latin1" | "binary" => Ok(Self::Latin1),
            "ascii" => Ok(Self::Ascii),
            _ => Err(Error::conversion(format!("agentc:fs: unsupported encoding {label:?}"))),
        }
    }

    pub fn decode(&self, bytes: &[u8]) -> String {
        match self {
            Self::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
            Self::Hex => bytes
                .iter()
                .flat_map(|byte| {
                    [
                        char::from_digit((byte >> 4) as u32, 16).unwrap(),
                        char::from_digit((byte & 0x0f) as u32, 16).unwrap(),
                    ]
                })
                .collect(),
            Self::Base64 => {
                let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);

                for chunk in bytes.chunks(3) {
                    let first = chunk[0];
                    let second = chunk.get(1).copied();
                    let third = chunk.get(2).copied();

                    text.push(Self::BASE64_ALPHABET[(first >> 2) as usize] as char);
                    text.push(
                        Self::BASE64_ALPHABET
                            [(((first & 0x03) << 4) | (second.unwrap_or_default() >> 4)) as usize]
                            as char,
                    );

                    match (second, third) {
                        (Some(second), Some(third)) => {
                            text.push(
                                Self::BASE64_ALPHABET
                                    [(((second & 0x0f) << 2) | (third >> 6)) as usize]
                                    as char,
                            );
                            text.push(Self::BASE64_ALPHABET[(third & 0x3f) as usize] as char);
                        }
                        (Some(second), None) => {
                            text.push(
                                Self::BASE64_ALPHABET[((second & 0x0f) << 2) as usize] as char,
                            );
                            text.push('=');
                        }
                        (None, None) => {
                            text.push('=');
                            text.push('=');
                        }
                        (None, Some(_)) => unreachable!(),
                    }
                }

                text
            }
            Self::Latin1 => bytes
                .iter()
                .map(|byte| char::from(*byte))
                .collect(),
            Self::Ascii => bytes
                .iter()
                .map(|byte| char::from(byte & 0x7f))
                .collect(),
        }
    }

    pub fn encode(&self, text: &str) -> Result<Vec<u8>, Error> {
        match self {
            Self::Utf8 => Ok(text.as_bytes().to_vec()),
            Self::Hex => {
                if text.len() % 2 != 0 {
                    return Err(Error::conversion("agentc:fs: hex input has an odd length"));
                }

                text.as_bytes()
                    .chunks(2)
                    .map(|pair| Ok((Self::hex_nibble(pair[0])? << 4) | Self::hex_nibble(pair[1])?))
                    .collect()
            }
            Self::Base64 => {
                if text.len() % 4 != 0 {
                    return Err(Error::conversion("agentc:fs: base64 input has an invalid length"));
                }

                let mut bytes = Vec::with_capacity(text.len() / 4 * 3);

                for chunk in text.as_bytes().chunks(4) {
                    let first = Self::base64_value(chunk[0])?;
                    let second = Self::base64_value(chunk[1])?;

                    bytes.push((first << 2) | (second >> 4));

                    match (chunk[2], chunk[3]) {
                        (b'=', b'=') => {}
                        (b'=', _) => {
                            return Err(Error::conversion(
                                "agentc:fs: base64 input has an invalid length",
                            ));
                        }
                        (third, b'=') => {
                            let third = Self::base64_value(third)?;

                            bytes.push(((second & 0x0f) << 4) | (third >> 2));
                        }
                        (third, fourth) => {
                            let third = Self::base64_value(third)?;
                            let fourth = Self::base64_value(fourth)?;

                            bytes.push(((second & 0x0f) << 4) | (third >> 2));
                            bytes.push(((third & 0x03) << 6) | fourth);
                        }
                    }
                }

                Ok(bytes)
            }
            Self::Latin1 => Ok(text
                .chars()
                .map(|character| (character as u32 & 0xff) as u8)
                .collect()),
            Self::Ascii => Ok(text
                .chars()
                .map(|character| (character as u32 & 0x7f) as u8)
                .collect()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Encoding;

    #[test]
    fn parse_accepts_every_supported_label() {
        assert_eq!(Encoding::parse("utf8").unwrap(), Encoding::Utf8);
        assert_eq!(Encoding::parse("utf-8").unwrap(), Encoding::Utf8);
        assert_eq!(Encoding::parse("hex").unwrap(), Encoding::Hex);
        assert_eq!(Encoding::parse("base64").unwrap(), Encoding::Base64);
        assert_eq!(Encoding::parse("latin1").unwrap(), Encoding::Latin1);
        assert_eq!(Encoding::parse("binary").unwrap(), Encoding::Latin1);
        assert_eq!(Encoding::parse("ascii").unwrap(), Encoding::Ascii);
    }

    #[test]
    fn parse_rejects_an_unknown_label() {
        assert!(matches!(
            Encoding::parse("utf16"),
            Err(agentc_executor_typescript::guestjs::errors::Error::Conversion {
                message,
                ..
            }) if message == "agentc:fs: unsupported encoding \"utf16\""
        ));
    }

    #[test]
    fn utf8_round_trips() {
        assert_eq!(Encoding::Utf8.encode("hello").unwrap(), b"hello");
        assert_eq!(Encoding::Utf8.decode(b"hello"), "hello");
    }

    #[test]
    fn hex_round_trips() {
        let bytes = b"hello";

        assert_eq!(Encoding::Hex.decode(bytes), "68656c6c6f");
        assert_eq!(
            Encoding::Hex
                .encode("68656c6c6f")
                .unwrap(),
            bytes
        );
    }

    #[test]
    fn hex_rejects_an_odd_length() {
        assert!(matches!(
            Encoding::Hex.encode("abc"),
            Err(agentc_executor_typescript::guestjs::errors::Error::Conversion {
                message,
                ..
            }) if message == "agentc:fs: hex input has an odd length"
        ));
    }

    #[test]
    fn base64_round_trips_with_padding() {
        let bytes = b"hi";

        assert_eq!(Encoding::Base64.decode(bytes), "aGk=");
        assert_eq!(Encoding::Base64.encode("aGk=").unwrap(), bytes);
    }

    #[test]
    fn base64_rejects_an_invalid_character() {
        assert!(matches!(
            Encoding::Base64.encode("aG$="),
            Err(agentc_executor_typescript::guestjs::errors::Error::Conversion {
                message,
                ..
            }) if message == "agentc:fs: invalid base64 character '$'"
        ));
    }

    #[test]
    fn latin1_round_trips_every_byte() {
        let bytes = (0u8..=255).collect::<Vec<_>>();

        assert_eq!(
            Encoding::Latin1
                .encode(&Encoding::Latin1.decode(&bytes))
                .unwrap(),
            bytes,
        );
    }

    #[test]
    fn ascii_masks_the_high_bit() {
        assert_eq!(Encoding::Ascii.decode(&[0xff]), "\u{7f}");
        assert_eq!(Encoding::Ascii.encode("ÿ").unwrap(), vec![0x7f]);
    }

    #[test]
    fn utf8_decode_replaces_invalid_sequences() {
        assert_eq!(Encoding::Utf8.decode(&[0xff]), "\u{fffd}");
    }
}
