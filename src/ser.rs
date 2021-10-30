pub mod hex {
    use std::fmt;

    use serde::{
        de::{self, Deserializer, Visitor},
        ser::Serializer,
    };

    pub fn serialize<S>(value: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(value))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(HexVisitor)
    }

    struct HexVisitor;

    impl<'de> Visitor<'de> for HexVisitor {
        type Value = [u8; 64];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a 64-byte array encoded as hex string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.len() != 128 {
                return Err(E::custom("value must be exactly 128 characters long"));
            }

            let mut data = [0; 64];
            hex::decode_to_slice(v, &mut data).map_err(E::custom)?;

            Ok(data)
        }
    }

    #[cfg(test)]
    mod tests {
        use serde::{Deserialize, Serialize};
        use serde_test::{assert_de_tokens_error, assert_tokens, Token};

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Test {
            #[serde(with = "super")]
            key: [u8; 64],
        }

        #[test]
        fn valid() {
            const HEX:&str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
            let test = Test { key: [255; 64] };

            assert_tokens(
                &test,
                &[
                    Token::Struct {
                        name: "Test",
                        len: 1,
                    },
                    Token::Str("key"),
                    Token::Str(HEX),
                    Token::StructEnd,
                ],
            );
        }

        #[test]
        fn invalid_size() {
            const SHORT: &str = "fff";
            const LONG: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

            assert_de_tokens_error::<Test>(
                &[
                    Token::Struct {
                        name: "Test",
                        len: 1,
                    },
                    Token::Str("key"),
                    Token::Str(SHORT),
                    Token::StructEnd,
                ],
                "value must be exactly 128 characters long",
            );

            assert_de_tokens_error::<Test>(
                &[
                    Token::Struct {
                        name: "Test",
                        len: 1,
                    },
                    Token::Str("key"),
                    Token::Str(LONG),
                    Token::StructEnd,
                ],
                "value must be exactly 128 characters long",
            );
        }

        #[test]
        fn invalid_hex() {
            const HEX: &str = "zfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

            assert_de_tokens_error::<Test>(
                &[
                    Token::Struct {
                        name: "Test",
                        len: 1,
                    },
                    Token::Str("key"),
                    Token::Str(HEX),
                    Token::StructEnd,
                ],
                "Invalid character 'z' at position 0",
            );
        }
    }
}
