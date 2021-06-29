//! `props-rs` is a library which parses a `.properties` file format as specified
//! by the [Oracle documentation](https://docs.oracle.com/cd/E23095_01/Platform.93/ATGProgGuide/html/s0204propertiesfileformat01.html)
//!
//!
//! ```
//! use props_rs::*;
//!
//! let properties = br"
//! key1=value1
//! key2=value2
//! key3=value3
//! ";
//! let parsed = parse(properties).unwrap();
//! let properties = to_map(parsed);
//!
//! assert_eq!("value1", properties.get("key1").unwrap());
//! assert_eq!("value2", properties.get("key2").unwrap());
//! assert_eq!("value3", properties.get("key3").unwrap());
//! ```
#![deny(missing_docs)]
#![deny(missing_crate_level_docs)]

mod parser;
pub use parser::Property;
use std::collections::HashMap;

/// Parses a properties file and returns a [`Vec`] of properties. There may
/// potentially be properties with duplicate keys in the returned [`Vec`].
///
/// Use the [`to_map`] convenience function to convert the vec into a set of
/// properties with unique keys.
pub fn parse(input: &[u8]) -> Result<Vec<Property>, nom::Err<nom::error::Error<&[u8]>>> {
    match parser::parser(input) {
        Ok((_, v)) => Ok(v),
        Err(e) => Err(e),
    }
}

/// A convenience function which converts a [`Vec`] of [`Property`] into a set of
/// [`Property`] stored in a [`HashMap`]
pub fn to_map(props: Vec<Property>) -> HashMap<String, String> {
    let mut map = HashMap::with_capacity(props.len());
    for prop in props.iter() {
        map.insert(prop.key.clone(), prop.value.clone());
    }
    map
}

#[cfg(test)]
mod tests {
    use super::parse;
    use super::to_map;

    #[test]
    pub fn test_parse_simple() {
        let v = br"
property=test
property2=test
";
        let res = parse(v);
        assert_eq!(2, res.unwrap().len());
    }

    #[test]
    pub fn test_broken_parse() {
        let v = br"=test
";
        assert_eq!(true, parse(v).is_err());
    }

    #[test]
    pub fn test_map_conversion() {
        let v = br"
property=test
property2=test
property=t
";
        let res = to_map(parse(v).unwrap());
        assert_eq!(2, res.len());
        assert_eq!("t", res.get("property").unwrap());
        assert_eq!("test", res.get("property2").unwrap());
    }
}
