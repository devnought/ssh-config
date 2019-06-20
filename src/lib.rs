// Many incorrect assumptions were made when creating this initially.
// See the following for a better description on the format:
// https://www.cyberciti.biz/faq/create-ssh-config-file-on-linux-unix/
// https://linux.die.net/man/5/ssh_config

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while1},
    character::complete::{multispace1, not_line_ending, space1},
    combinator::{map, not, peek},
    multi::many0,
    sequence::tuple,
    IResult,
};

#[derive(PartialEq, Debug)]
pub struct Host<'a> {
    name: &'a str,
    properties: Vec<Property<'a>>,
}

#[derive(PartialEq, Debug)]
pub struct Property<'a> {
    key: &'a str,
    value: &'a str,
}

pub fn parse(data: &str) -> Result<Vec<Host>, ()> {
    hosts(data).map(|(_, hosts)| hosts).map_err(|_| ())
}

fn string(i: &str) -> IResult<&str, &str> {
    take_while1(|c: char| !c.is_whitespace() && c != '=' && c != '#')(i)
}

fn comment(i: &str) -> IResult<&str, &str> {
    let parser = tuple((tag("#"), not_line_ending));
    let (input, (_, _)) = parser(i)?;

    Ok((input, ""))
}

fn space_or_equals(i: &str) -> IResult<&str, &str> {
    let parser = alt((tag("="), space1));
    let (input, _) = parser(i)?;

    Ok((input, ""))
}

fn space_or_comment(i: &str) -> IResult<&str, &str> {
    let parser = alt((comment, multispace1));
    let (input, _) = parser(i)?;

    Ok((input, ""))
}

fn spaces_or_comments(i: &str) -> IResult<&str, &str> {
    let parser = many0(space_or_comment);
    let (input, _) = parser(i)?;

    Ok((input, ""))
}

fn not_comment(i: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c != '\r' && c != '\n' && c != '#')(i)
}

fn host_line(i: &str) -> IResult<&str, &str> {
    let parser = tuple((tag_no_case("host"), space_or_equals, string));
    let (input, (_, _, name)) = parser(i)?;

    Ok((input, name))
}

// TODO: Property should become something like
// Property { key, tokens }
// or
// Property { key, values }
// to handle the case where a key has multple values, like for `LocalForward`
fn property_line(i: &str) -> IResult<&str, Property> {
    not(peek(host_line))(i)?;

    let parser = tuple((string, space_or_equals, not_comment));
    let (input, (key, _, value)) = parser(i)?;

    Ok((
        input,
        Property {
            key,
            value: value.trim(),
        },
    ))
}

fn properties(i: &str) -> IResult<&str, Vec<Property>> {
    let parser = many0(tuple((spaces_or_comments, property_line)));
    let (input, lines) = map(parser, |props| props.into_iter().map(|(_, x)| x).collect())(i)?;

    Ok((input, lines))
}

fn host_block(i: &str) -> IResult<&str, Host> {
    let parser = tuple((spaces_or_comments, host_line, properties));
    let (input, (_, name, properties)) = parser(i)?;

    let host = Host { name, properties };

    Ok((input, host))
}

fn hosts(i: &str) -> IResult<&str, Vec<Host>> {
    let parser = many0(tuple((spaces_or_comments, host_block, spaces_or_comments)));
    let (input, hosts) = map(parser, |hosts| {
        hosts.into_iter().map(|(_, h, _)| h).collect()
    })(i)?;

    Ok((input, hosts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string() {
        if string("").is_ok() {
            panic!("Should not be able to parse empty string as valid string");
        }
    }

    #[test]
    fn valid_string() {
        let (input, host) = string("Host").expect("Could not parse 'Host'");
        assert_eq!("", input);
        assert_eq!("Host", host);
    }

    #[test]
    fn valid_string_space() {
        let (input, host) = string("Host dev").expect("Could not parse 'Host dev'");
        assert_eq!(" dev", input);
        assert_eq!("Host", host);
    }

    #[test]
    fn space_or_equals_is_equals() {
        let (input, _) = space_or_equals("=").expect("Could not parse equals");
        assert_eq!("", input);
    }

    #[test]
    fn space_or_equals_is_space() {
        let (input, _) = space_or_equals("      ").expect("Could not parse space");
        assert_eq!("", input);
    }

    #[test]
    fn space_or_comment_spaces() {
        let (input, _) = space_or_comment("   ").expect("Could not parse whitespace");
        assert_eq!("", input);
    }

    #[test]
    fn space_or_comment_comment() {
        let (input, _) = space_or_comment("#this is a comment").expect("Could not parse comment");
        assert_eq!("", input);
    }

    #[test]
    fn space_or_comment_spaces_and_comment() {
        let (input, _) =
            space_or_comment("      #comment").expect("Could not parse space and comment");
        assert_eq!("#comment", input);

        let (input, _) = space_or_comment(input).expect("Could not parse remaining comment");
        assert_eq!("", input);
    }

    #[test]
    fn spaces_and_comments_both() {
        let (input, _) = spaces_or_comments("     #comment\n\n\n#comment      \n\n")
            .expect("Could not parse spaces and comment");
        assert_eq!("", input);
    }

    #[test]
    fn many_properties() {
        let (input, properties) = properties("   \n\n\n      asd 123 345\n\n\nDave yes\n")
            .expect("Could not parse properties");

        let expected_properties = vec![
            Property {
                key: "asd",
                value: "123 345",
            },
            Property {
                key: "Dave",
                value: "yes",
            },
        ];

        assert_eq!("\n", input);
        assert_eq!(expected_properties, properties);
    }

    #[test]
    fn many_properties_comments() {
        let data = r"
            HostName butt   #no butts   
            Asd=123
            #moar comment


            Blah whatever";

        let (input, properties) =
            properties(data).expect("Could not parse a mix of properties and comments");

        let expected_properties = vec![
            Property {
                key: "HostName",
                value: "butt",
            },
            Property {
                key: "Asd",
                value: "123",
            },
            Property {
                key: "Blah",
                value: "whatever",
            },
        ];

        assert_eq!("", input);
        assert_eq!(expected_properties, properties);
    }

    #[test]
    fn property_line_equals() {
        let (input, property) =
            property_line("HostName=dev").expect("Could not parse property line with equals");

        let expected_property = Property {
            key: "HostName",
            value: "dev",
        };

        assert_eq!("", input);
        assert_eq!(expected_property, property);
    }

    #[test]
    fn property_line_space() {
        let (input, property) =
            property_line("HostName dev").expect("Could not parse property line with space");

        let expected_property = Property {
            key: "HostName",
            value: "dev",
        };

        assert_eq!("", input);
        assert_eq!(expected_property, property);
    }

    #[test]
    fn host_line_equals() {
        let (input, host) = host_line("Host=dev").expect("Could not parse host with equals");
        assert_eq!("", input);
        assert_eq!("dev", host);
    }

    #[test]
    fn host_line_space() {
        let (input, host) = host_line("host dev").expect("Could not parse host with space");
        assert_eq!("", input);
        assert_eq!("dev", host);
    }

    #[test]
    fn single_host_block() {
        let (input, host) = host_block(
            "  \n\n   \n\n\
             \n       Host dev\n   \
             Asd      123     \n\n",
        )
        .expect("Could not parse single host block");

        let expected_host = Host {
            name: "dev",
            properties: vec![Property {
                key: "Asd",
                value: "123",
            }],
        };

        assert_eq!("\n\n", input);
        assert_eq!(expected_host, host);
    }

    #[test]
    fn single_host_block_no_properties() {
        let (input, host) = host_block(
            "  \n\n   \n\n\
             \n       Host dev\n   \
             \n\n",
        )
        .expect("Could not parse single host block");

        let expected_host = Host {
            name: "dev",
            properties: vec![],
        };

        assert_eq!("\n   \n\n", input);
        assert_eq!(expected_host, host);
    }

    #[test]
    fn two_host_blocks_no_properties() {
        let (input, host) = host_block(
            "  \n\n   \n\n\
             \n       Host dev\n   \
             \n\n\
             Host zzz",
        )
        .expect("Could not parse single host block pair");

        let expected_host = Host {
            name: "dev",
            properties: vec![],
        };

        assert_eq!("\n   \n\nHost zzz", input);
        assert_eq!(expected_host, host);
    }

    #[test]
    fn many_hosts_no_properties() {
        let (input, hosts) = hosts(
            "  \n\n   \n\n\
             \n       Host dev\n   \
             \n\n\
             Host zzz",
        )
        .expect("Could not parse multiple empty hosts");

        let expected_hosts = vec![
            Host {
                name: "dev",
                properties: vec![],
            },
            Host {
                name: "zzz",
                properties: vec![],
            },
        ];

        assert_eq!("", input);
        assert_eq!(expected_hosts, hosts);
    }

    #[test]
    fn many_hosts() {
        let (input, hosts) = hosts(
            "\n\n\n\n     Host old    \n\
            Asd    123\n\
            Test zz\
            \n\n\n\n\n\n\n\
            Host gregg\n
            #wut
            HostName hello\n\n\n\n
            Other thing\n\n\n",
        )
        .expect("Could not parse multple hosts with their properties");

        let expected_hosts = vec![
            Host {
                name: "old",
                properties: vec![
                    Property {
                        key: "Asd",
                        value: "123",
                    },
                    Property {
                        key: "Test",
                        value: "zz",
                    },
                ],
            },
            Host {
                name: "gregg",
                properties: vec![
                    Property {
                        key: "HostName",
                        value: "hello",
                    },
                    Property {
                        key: "Other",
                        value: "thing",
                    },
                ],
            },
        ];

        assert_eq!("", input);
        assert_eq!(expected_hosts, hosts);
    }

    #[test]
    fn no_hosts() {
        let empty_input = "       ";
        let (input, hosts) = hosts(empty_input).expect("Could not parse empty string");
        let expected_hosts: Vec<Host> = vec![];

        assert_eq!(empty_input, input);
        assert_eq!(expected_hosts, hosts);
    }

    #[test]
    fn property_as_host_line() {
        if property_line("       \n\nAsd 123\n\n\n").is_ok() {
            panic!("Property is not allowed to be a host line");
        }
    }

    #[test]
    fn proptery_as_host_block() {
        if host_block("       \n\nAsd 123\n\n\n").is_ok() {
            panic!("Property is not allowed to be a host block");
        }
    }

    #[test]
    fn proptery_as_hosts() {
        let bad_input = "       \n\nAsd 123\n\n\n";
        let (input, hosts) =
            hosts(bad_input).expect("Could not parse invalid string for host collection");

        let expected_hosts: Vec<Host> = vec![];

        assert_eq!(bad_input, input);
        assert_eq!(expected_hosts, hosts);
    }
}
