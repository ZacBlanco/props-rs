//! A nom parser for Java properties files
use nom::branch::alt;
use nom::bytes::complete::{tag, take_till};
use nom::combinator::{complete, eof, opt, value};

use nom::character::complete::{none_of, one_of};
use nom::multi::{many0, many1, many_till, separated_list0, separated_list1};

use nom::IResult;

/// A property representing a parsed configuration key-value pair.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Property {
    /// A string representing the identifier for a particular property
    pub key: String,
    /// A string representing the value for a particular property.
    pub value: String,
}

/// Consumes a sequence of spaces, tabs, and form feeds ("\f")
fn consume_whitespaces(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = many0(one_of(" \t\u{c}"))(input)?;
    Ok((input, ()))
}

/// Consumes a single EOL of "\r\n", "\r" or "\n"
fn consume_eol(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = alt((complete(tag("\r\n")), tag("\r"), tag("\n")))(input)?;
    Ok((input, ()))
}

/// Consumes an EOL or EOF
fn consume_eol_or_eof(input: &[u8]) -> IResult<&[u8], ()> {
    alt((value((), eof), consume_eol))(input)
}

/// Consumes a single blank line
fn blank_line(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = consume_whitespaces(input)?;
    consume_eol_or_eof(input)
}

/// Consumes a line with a comment
fn comment_line(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = consume_whitespaces(input)?;
    let (input, _) = one_of("#!")(input)?;
    let (input, _) = take_till(eol)(input)?;
    consume_eol_or_eof(input)
}

/// Returns whether or not a byte (as a character) represents a EOL character
/// (line feed `\r` or newline `\n`)
fn eol(c: u8) -> bool {
    c as char == '\r' || c as char == '\n'
}

/// Consumes a single line escape and any whitespaces after it
fn consume_line(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = tag(r"\")(input)?;
    let (input, _) = consume_eol(input)?;
    let (input, _) = consume_whitespaces(input)?;
    Ok((input, ()))
}

/// Consumes a set of alternating lines and whiespaces. Stopping once there is no more alternating
fn consume_whitespaces_and_lines(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = separated_list0(many1(consume_line), consume_whitespaces)(input)?;
    Ok((input, ()))
}

/// Consumes a character that exists in a key
fn char_in_key(input: &[u8]) -> IResult<&[u8], char> {
    none_of(":=\n\r \t\u{c}\\")(input)
}

/// Consumes a character which exists in a value
fn char_in_value(input: &[u8]) -> IResult<&[u8], char> {
    none_of("\n\r\\")(input)
}

/// matches a single character and returns its escaped equivalent e.g. `'t' -> '\t'`
fn escaped_char_to_char(v: char) -> char {
    match v {
        't' => '\t',
        'n' => '\n',
        'f' => '\u{c}',
        'r' => '\r',
        '\\' => '\\',
        _ => v,
    }
}

/// consumes an escaped character in a key or value
fn escape_in_key_or_value(input: &[u8]) -> IResult<&[u8], char> {
    let (input, _) = tag(r"\")(input)?;
    let (input, c) = none_of("u\r\n")(input)?;
    Ok((input, escaped_char_to_char(c)))
}

/// consumes a character in a key
fn one_char_in_key(input: &[u8]) -> IResult<&[u8], char> {
    alt((escape_in_key_or_value, char_in_key))(input)
}

/// consumes a character in a value
fn one_char_in_value(input: &[u8]) -> IResult<&[u8], char> {
    alt((escape_in_key_or_value, char_in_value))(input)
}

/// Consumes and returns a `String` representing the key to a property.
fn consume_key(input: &[u8]) -> IResult<&[u8], String> {
    // use many1(consume_line) because many0 always returns true and causes a separated list error.
    let (input, chars) = separated_list1(many1(consume_line), many1(one_char_in_key))(input)?;
    Ok((input, chars.into_iter().flatten().collect::<String>()))
}

/// Consumes and returns a `String` representing the value of a property.
fn consume_value(input: &[u8]) -> IResult<&[u8], String> {
    // use many1(consume_line) because many0 always returns true and causes a separated list error.
    let (input, chars) = separated_list0(many1(consume_line), many0(one_char_in_value))(input)?;
    Ok((input, chars.into_iter().flatten().collect::<String>()))
}

/// Consumes an entire line (or set of lines) representing a key-value property
fn kv_line(input: &[u8]) -> IResult<&[u8], Property> {
    let (input, _) = consume_whitespaces_and_lines(input)?;
    let (input, key) = consume_key(input)?;
    let (input, _) = consume_whitespaces_and_lines(input)?;
    let (input, _) = opt(complete(one_of(":=")))(input)?;
    let (input, _) = consume_whitespaces_and_lines(input)?;
    let (input, value) = consume_value(input)?;
    let (input, _) = consume_eol_or_eof(input)?;
    Ok((input, Property { key, value }))
}

type ParsedProps<'a> = (Vec<Option<Property>>, &'a [u8]);

/// The full parser which consumes comments, blanks, and Property lines.
fn _fparser(input: &[u8]) -> IResult<&[u8], ParsedProps> {
    many_till(
        alt((
            value(None, complete(comment_line)),
            value(None, complete(blank_line)),
            opt(complete(kv_line)),
        )),
        eof,
    )(input)
}

/// Public parser function
pub fn parser(input: &[u8]) -> IResult<&[u8], Vec<Property>> {
    let (input, props) = _fparser(input)?;
    let v = props.0.into_iter().flatten().collect();
    Ok((input, v))
}

#[cfg(test)]
mod test {
    use super::*;
    use nom::error::dbg_dmp;

    macro_rules! assert_done {
        ($t:expr, $v:expr) => {
            assert_eq!($t, Ok((&b""[..], $v)))
        };
    }

    macro_rules! assert_done_partial {
        ($t:expr, $v:expr, $s:tt) => {
            assert_eq!($t, Ok((&$s[..], $v)))
        };
    }

    macro_rules! assert_incomplete {
        ($t:expr) => {
            let r = $t;
            assert!(r.is_err(), "Expected IResult::Incomplete, got {:?}", r);
        };
    }

    #[test]
    fn test_key() {
        // simple test
        assert_done!(consume_key(b"hello"), String::from("hello"));

        // A space ends the key
        assert_done_partial!(
            consume_key(b"hello world"),
            String::from("hello"),
            b" world"
        );

        // A colon ends the key
        assert_done_partial!(
            consume_key(b"hello:world"),
            String::from("hello"),
            b":world"
        );

        // An equal sign ends the key
        assert_done_partial!(
            consume_key(b"hello=world"),
            String::from("hello"),
            b"=world"
        );

        // An eol ends the key
        assert_done_partial!(
            consume_key(b"hello\nworld"),
            String::from("hello"),
            b"\nworld"
        );
        assert_done_partial!(
            consume_key(b"hello\rworld"),
            String::from("hello"),
            b"\rworld"
        );

        // These characters are valid
        assert_done!(
            consume_key(b"@#$%^&*()_+-`~?/.>,<|][{};\""),
            String::from("@#$%^&*()_+-`~?/.>,<|][{};\"")
        );

        // Spaces can be escaped
        assert_done!(
            consume_key(br"key\ with\ spaces"),
            String::from("key with spaces")
        );

        // Colons can be escaped
        assert_done!(
            consume_key(br"key\:with\:colons"),
            String::from("key:with:colons")
        );

        // Equals can be escaped
        assert_done!(
            consume_key(br"key\=with\=equals"),
            String::from("key=with=equals")
        );

        // Special characters can be escaped
        assert_done!(
            consume_key(br"now\nwith\rsome\fspecial\tcharacters\\"),
            String::from("now\nwith\rsome\u{c}special\tcharacters\\")
        );

        // Escapes on non escapable characters are ignored
        assert_done!(
            consume_key(br"w\iths\omeran\domch\arse\sca\pe\d"),
            String::from("withsomerandomcharsescaped")
        );

        // No input is not a key
        assert_incomplete!(consume_key(b""));

        // With logical line splits
        assert_done!(
            dbg_dmp(consume_key, "ell")(b"abc\\\n   def"),
            String::from("abcdef")
        );
        assert_done!(
            dbg_dmp(consume_key, "ell")(b"gh\\\n    \\\r    \\\r\nij\\\n\t kl"),
            String::from("ghijkl")
        );
    }

    /// utf-8 not yet implemented
    #[allow(dead_code)]
    fn test_utf8_keys() {
        // Unicode esacpes
        assert_done!(
            consume_key(br"\u0048\u0065\u006c\u006c\u006f"),
            String::from("Hello")
        );

        // A byte above 127 is interpreted as a latin-1 extended character with
        // the same Unicode code point value.
        assert_done!(consume_key(&[0xA9]), String::from("\u{a9}"));

        // An \u escape must be followed by 4 hex digits.
        assert_done_partial!(
            consume_key(br"abc\uhello"),
            String::from("abc"),
            br"\uhello"
        );
    }

    #[test]
    fn test_value() {
        // basic case
        assert_done!(consume_value(b"hello"), String::from("hello"));

        // colons and equal signs are valid
        assert_done!(consume_value(b"h:l=o"), String::from("h:l=o"));

        // spaces are valid, even at the end
        assert_done!(
            consume_value(b"hello world  "),
            String::from("hello world  ")
        );

        // These are valid characters
        assert_done!(
            consume_value(b"/~`!@#$%^&*()-_=+[{]};:'\",<.>/?|"),
            String::from("/~`!@#$%^&*()-_=+[{]};:'\",<.>/?|")
        );

        // An eol ends the value
        assert_done_partial!(
            consume_value(b"hello\nworld"),
            String::from("hello"),
            b"\nworld"
        );
        assert_done_partial!(
            consume_value(b"hello\rworld"),
            String::from("hello"),
            b"\rworld"
        );

        // Special characters can be escaped
        assert_done!(
            consume_value(br"now\nwith\rsome\fspecial\tcharacters\\"),
            String::from("now\nwith\rsome\u{c}special\tcharacters\\")
        );

        // Escapes on non escapable characters are ignored
        assert_done!(
            consume_value(br"w\iths\omeran\domch\arse\sca\pe\d"),
            String::from("withsomerandomcharsescaped")
        );

        // No input is a valid value
        assert_done!(consume_value(b""), String::from(""));

        // With logical line splits
        assert_done!(consume_value(b"abc\\\n   def"), String::from("abcdef"));
        assert_done!(
            consume_value(b"gh\\\n    \\\r    \\\r\nij\\\n\t kl"),
            String::from("ghijkl")
        );
    }

    /// utf-8 not yet implemented
    #[allow(dead_code)]
    fn test_utf8_values() {
        // Unicode esacpes
        assert_done!(
            consume_value(br"\u0048\u0065\u006c\u006c\u006f"),
            String::from("Hello")
        );

        // A byte above 127 is interpreted as a latin-1 extended character with
        // the same Unicode code point value.
        assert_done!(consume_value(&[0xA9]), String::from("\u{a9}"));

        // An \u escape must be followed by 4 hex digits.
        assert_done_partial!(
            consume_value(br"abc\uhello"),
            String::from("abc"),
            br"\uhello"
        );
    }

    #[test]
    fn test_kv_line() {
        let parsed = kv_line(b"key=value");
        assert_eq!(
            parsed.unwrap().1,
            Property {
                key: String::from("key"),
                value: String::from("value")
            }
        );
    }

    #[test]
    fn test_full_parse_simple() {
        let prop = br"key.1=value1
key.two=value2

";
        let parsed = _fparser(prop);
        let props = parsed.unwrap().1;
        println!("{:?}", props.0);
        assert_eq!(3, props.0.len());
        let props: Vec<Property> = props.0.into_iter().flatten().collect();
        assert_eq!(2, props.len());
        assert_eq!(props[0].key, "key.1");
        assert_eq!(props[0].value, "value1");
        assert_eq!(props[1].key, "key.two");
        assert_eq!(props[1].value, "value2")
    }

    #[test]
    fn test_pub_parser() {
        let prop = br"key.1=value1
key.two=value2

";
        let parsed = parser(prop);
        let props = parsed.unwrap().1;
        assert_eq!(2, props.len());
        assert_eq!(props[0].key, "key.1");
        assert_eq!(props[0].value, "value1");
        assert_eq!(props[1].key, "key.two");
        assert_eq!(props[1].value, "value2")
    }
}
