use cssparser::{
    BasicParseErrorKind, ParseError, ParseErrorKind, Parser, ParserInput, Token as CssToken,
};
use proc_macro::TokenStream;
use quote::quote;
use syn::LitStr;

pub(crate) fn css(input: TokenStream) -> TokenStream {
    let literal = syn::parse_macro_input!(input as LitStr);
    match tokenize_css(&literal.value()) {
        Ok(tokens) => match encode_token_stream(&tokens) {
            Ok(bytes) => {
                let bytes = bytes.iter();
                quote!(::dioxus_lynx::css::CSSTokenStream::from_static(&[#(#bytes),*])).into()
            }
            Err(message) => syn::Error::new(literal.span(), message)
                .to_compile_error()
                .into(),
        },
        Err(message) => syn::Error::new(literal.span(), message)
            .to_compile_error()
            .into(),
    }
}

fn tokenize_css(source: &str) -> Result<Vec<dioxus_lynx_sys::css::Token>, String> {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let mut tokens = Vec::new();
    collect_tokens(&mut parser, &mut tokens).map_err(|error| format_parse_error(error, source))?;
    Ok(tokens)
}

fn encode_token_stream(tokens: &[dioxus_lynx_sys::css::Token]) -> Result<Vec<u8>, String> {
    let token_count = u32::try_from(tokens.len())
        .map_err(|_| "CSS token stream contains too many tokens".to_string())?;
    let mut records =
        Vec::with_capacity(tokens.len() * dioxus_lynx_sys::css::CSS_TOKEN_STREAM_RECORD_LEN);
    let mut payload = Vec::new();

    for token in tokens {
        let payload_offset = u32::try_from(payload.len())
            .map_err(|_| "CSS token stream payload is too large".to_string())?;
        let value_bytes = token.value.as_bytes();
        let payload_len = u32::try_from(value_bytes.len())
            .map_err(|_| "CSS token stream token payload is too large".to_string())?;

        records.push(token.token_type);
        records.push(0);
        records.extend_from_slice(&0u16.to_le_bytes());
        records.extend_from_slice(&payload_offset.to_le_bytes());
        records.extend_from_slice(&payload_len.to_le_bytes());
        payload.extend_from_slice(value_bytes);
    }

    let payload_len = u32::try_from(payload.len())
        .map_err(|_| "CSS token stream payload is too large".to_string())?;
    let mut bytes = Vec::with_capacity(
        dioxus_lynx_sys::css::CSS_TOKEN_STREAM_HEADER_LEN + records.len() + payload.len(),
    );
    bytes.extend_from_slice(&dioxus_lynx_sys::css::CSS_TOKEN_STREAM_MAGIC);
    bytes.push(dioxus_lynx_sys::css::CSS_TOKEN_STREAM_VERSION);
    bytes.push(dioxus_lynx_sys::css::CSS_TOKEN_STREAM_HEADER_LEN as u8);
    bytes.push(dioxus_lynx_sys::css::CSS_TOKEN_STREAM_RECORD_LEN as u8);
    bytes.push(0);
    bytes.extend_from_slice(&token_count.to_le_bytes());
    bytes.extend_from_slice(&payload_len.to_le_bytes());
    bytes.extend_from_slice(&records);
    bytes.extend_from_slice(&payload);

    Ok(bytes)
}

fn collect_tokens<'i, 't>(
    input: &mut Parser<'i, 't>,
    tokens: &mut Vec<dioxus_lynx_sys::css::Token>,
) -> Result<(), ParseError<'i, String>> {
    loop {
        let token = match input.next_including_whitespace_and_comments() {
            Ok(token) => token.clone(),
            Err(error) if matches!(error.kind, BasicParseErrorKind::EndOfInput) => return Ok(()),
            Err(error) => return Err(error.into()),
        };

        if token.is_parse_error() {
            return Err(input.new_custom_error(format!("invalid CSS token `{token:?}`")));
        }

        match token {
            CssToken::Function(name) => {
                tokens.push(dioxus_lynx_sys::css::Token::new(
                    dioxus_lynx_sys::css::FUNCTION_TOKEN,
                    name.to_string(),
                ));
                input.parse_nested_block(|input| collect_tokens(input, tokens))?;
                tokens.push(dioxus_lynx_sys::css::Token::new(
                    dioxus_lynx_sys::css::RIGHT_PARENTHESES_TOKEN,
                    ")",
                ));
            }
            CssToken::ParenthesisBlock => {
                tokens.push(dioxus_lynx_sys::css::Token::new(
                    dioxus_lynx_sys::css::LEFT_PARENTHESES_TOKEN,
                    "(",
                ));
                input.parse_nested_block(|input| collect_tokens(input, tokens))?;
                tokens.push(dioxus_lynx_sys::css::Token::new(
                    dioxus_lynx_sys::css::RIGHT_PARENTHESES_TOKEN,
                    ")",
                ));
            }
            CssToken::SquareBracketBlock => {
                tokens.push(dioxus_lynx_sys::css::Token::new(
                    dioxus_lynx_sys::css::LEFT_SQUARE_BRACKET_TOKEN,
                    "[",
                ));
                input.parse_nested_block(|input| collect_tokens(input, tokens))?;
                tokens.push(dioxus_lynx_sys::css::Token::new(
                    dioxus_lynx_sys::css::RIGHT_SQUARE_BRACKET_TOKEN,
                    "]",
                ));
            }
            CssToken::CurlyBracketBlock => {
                tokens.push(dioxus_lynx_sys::css::Token::new(
                    dioxus_lynx_sys::css::LEFT_CURLY_BRACKET_TOKEN,
                    "{",
                ));
                input.parse_nested_block(|input| collect_tokens(input, tokens))?;
                tokens.push(dioxus_lynx_sys::css::Token::new(
                    dioxus_lynx_sys::css::RIGHT_CURLY_BRACKET_TOKEN,
                    "}",
                ));
            }
            token => tokens.push(token_to_spec(token)),
        }
    }
}

fn token_to_spec(token: CssToken<'_>) -> dioxus_lynx_sys::css::Token {
    match token {
        CssToken::Ident(value) => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::IDENT_TOKEN, value.to_string())
        }
        CssToken::AtKeyword(value) => dioxus_lynx_sys::css::Token::new(
            dioxus_lynx_sys::css::AT_KEYWORD_TOKEN,
            value.to_string(),
        ),
        CssToken::Hash(value) | CssToken::IDHash(value) => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::HASH_TOKEN, value.to_string())
        }
        CssToken::QuotedString(value) => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::STRING_TOKEN, value.to_string())
        }
        CssToken::UnquotedUrl(value) => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::URL_TOKEN, value.to_string())
        }
        CssToken::Delim(value) => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::DELIM_TOKEN, value.to_string())
        }
        CssToken::Number {
            has_sign,
            value,
            int_value,
        } => dioxus_lynx_sys::css::Token::new(
            dioxus_lynx_sys::css::NUMBER_TOKEN,
            number_text(has_sign, value, int_value),
        ),
        CssToken::Percentage {
            has_sign,
            unit_value,
            int_value,
        } => dioxus_lynx_sys::css::Token::new(
            dioxus_lynx_sys::css::PERCENTAGE_TOKEN,
            number_text(has_sign, unit_value * 100.0, int_value),
        ),
        CssToken::Dimension {
            has_sign,
            value,
            int_value,
            unit,
        } => dioxus_lynx_sys::css::Token::new(
            dioxus_lynx_sys::css::DIMENSION_TOKEN,
            format!("{}{}", number_text(has_sign, value, int_value), unit),
        ),
        CssToken::WhiteSpace(value) => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::WHITESPACE_TOKEN, value)
        }
        CssToken::Comment(value) => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::COMMENT_TOKEN, value)
        }
        CssToken::Colon => dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::COLON_TOKEN, ":"),
        CssToken::Semicolon => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::SEMICOLON_TOKEN, ";")
        }
        CssToken::Comma => dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::COMMA_TOKEN, ","),
        CssToken::IncludeMatch => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::DELIM_TOKEN, "~=")
        }
        CssToken::DashMatch => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::DELIM_TOKEN, "|=")
        }
        CssToken::PrefixMatch => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::DELIM_TOKEN, "^=")
        }
        CssToken::SuffixMatch => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::DELIM_TOKEN, "$=")
        }
        CssToken::SubstringMatch => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::DELIM_TOKEN, "*=")
        }
        CssToken::CDO => dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::CDO_TOKEN, "<!--"),
        CssToken::CDC => dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::CDC_TOKEN, "-->"),
        CssToken::BadUrl(value) => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::BAD_URL_TOKEN, value.to_string())
        }
        CssToken::BadString(value) => dioxus_lynx_sys::css::Token::new(
            dioxus_lynx_sys::css::BAD_STRING_TOKEN,
            value.to_string(),
        ),
        CssToken::CloseParenthesis => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::RIGHT_PARENTHESES_TOKEN, ")")
        }
        CssToken::CloseSquareBracket => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::RIGHT_SQUARE_BRACKET_TOKEN, "]")
        }
        CssToken::CloseCurlyBracket => {
            dioxus_lynx_sys::css::Token::new(dioxus_lynx_sys::css::RIGHT_CURLY_BRACKET_TOKEN, "}")
        }
        CssToken::Function(_)
        | CssToken::ParenthesisBlock
        | CssToken::SquareBracketBlock
        | CssToken::CurlyBracketBlock => {
            unreachable!("block-opening tokens are handled by collect_tokens")
        }
    }
}

fn number_text(has_sign: bool, value: f32, int_value: Option<i32>) -> String {
    let mut text = match int_value {
        Some(value) => value.to_string(),
        None => value.to_string(),
    };

    if has_sign && !text.starts_with('-') && !text.starts_with('+') {
        text.insert(0, '+');
    }

    text
}

fn format_parse_error(error: ParseError<'_, String>, source: &str) -> String {
    let location = format!(
        "line {}, column {}",
        error.location.line + 1,
        error.location.column
    );
    let reason = match error.kind {
        ParseErrorKind::Basic(kind) => kind.to_string(),
        ParseErrorKind::Custom(error) => error,
    };
    let source = source.trim();

    if source.is_empty() {
        format!("{reason} at {location}")
    } else {
        format!("{reason} at {location} near `{}`", truncate_source(source))
    }
}

fn truncate_source(source: &str) -> String {
    const MAX_SOURCE_LEN: usize = 80;
    if source.len() <= MAX_SOURCE_LEN {
        source.to_string()
    } else {
        format!("{}...", &source[..MAX_SOURCE_LEN])
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn tokenizes_css_text() {
        let tokens = super::tokenize_css(".item { color: red; }").unwrap();

        assert!(tokens.iter().any(|token| {
            token.token_type == dioxus_lynx_sys::css::IDENT_TOKEN && token.value == "color"
        }));
        assert!(
            tokens
                .iter()
                .any(|token| token.token_type == dioxus_lynx_sys::css::LEFT_CURLY_BRACKET_TOKEN)
        );
    }

    #[test]
    fn encodes_css_token_stream_bytes() {
        let tokens = super::tokenize_css(".item { color: red; }").unwrap();
        let bytes = super::encode_token_stream(&tokens).unwrap();

        assert!(bytes.starts_with(&dioxus_lynx_sys::css::CSS_TOKEN_STREAM_MAGIC));
        assert_eq!(bytes[4], dioxus_lynx_sys::css::CSS_TOKEN_STREAM_VERSION);
        assert_eq!(
            bytes[5],
            dioxus_lynx_sys::css::CSS_TOKEN_STREAM_HEADER_LEN as u8
        );
        assert_eq!(
            bytes[6],
            dioxus_lynx_sys::css::CSS_TOKEN_STREAM_RECORD_LEN as u8
        );
        assert!(bytes.len() > dioxus_lynx_sys::css::CSS_TOKEN_STREAM_HEADER_LEN);
    }
}
