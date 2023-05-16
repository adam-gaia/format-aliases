use anyhow::bail;
use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use colorized::Colors;
use colorized::*;
use indoc::printdoc;
use nom::bytes::complete::take_till1;
use nom::bytes::complete::take_until;
use nom::bytes::complete::take_while;
use nom::bytes::complete::take_while1;
use nom::character::complete::anychar;
use nom::character::complete::multispace1;
use nom::combinator::map;
use nom::multi::separated_list1;
use nom::sequence::delimited;
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take},
    character::complete::{alpha1, alphanumeric1, one_of},
    combinator::opt,
    error::{context, ErrorKind, VerboseError},
    multi::{count, many0, many1, many_m_n},
    sequence::{preceded, separated_pair, terminated, tuple},
    AsChar, Err as NomErr, IResult, InputTakeAtPosition,
};
use std::collections::HashMap;
use std::io;
use std::io::BufRead;
use std::io::{Read, Write};
use std::str::FromStr;

#[derive(Debug, Clone)]
enum Shell {
    Bourne,
}

impl Shell {
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "sh" | "bash" | "zsh" => Ok(Self::Bourne),
            _ => bail!("Unsupported shell"),
        }
    }
}

fn print_function(shell: &str) -> Result<()> {
    let shell = Shell::from_str(shell)?;
    match shell {
        Shell::Bourne => {
            printdoc! {"
                alias() {{
                    if [ $# -eq 0 ]; then
                        # Pipe the output of the builtin alias command to be formatted
                        builtin alias | format-aliases
                    else
                        # Pass the arguments to the builtin alias command
                        builtin alias ${{@}}
                    fi
                }}
            "}
        }
    }
    Ok(())
}

#[derive(Debug, Subcommand)]
enum Command {
    Init { shell: String },
    Format,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Debug)]
struct AliasedCommand {
    name: String,
    args: Vec<String>,
}

impl AliasedCommand {
    fn new(name: String, args: Vec<String>) -> Self {
        Self { name, args }
    }
}

#[derive(Debug, Eq, PartialEq, PartialOrd)]
struct Alias {
    name: String,
    value: Vec<String>,
}
impl Alias {
    fn new(name: &str, value: Vec<&str>) -> Self {
        Self {
            name: name.to_string(),
            value: value.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn display(&self, in_color: bool) -> String {
        if in_color {
            format!(
                "{}='{}'",
                self.name.color(Colors::GreenFg),
                self.value.join(" ")
            )
        } else {
            format!("{}='{}'", self.name, self.value.join(" "))
        }
    }
}

impl Ord for Alias {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

fn parse_name(input: &str) -> IResult<&str, &str> {
    context("name", take_till1(|c: char| c == '='))(input)
}

fn parse_token(input: &str) -> IResult<&str, &str> {
    context(
        "token",
        take_while1(|c: char| c != '\'' && c != '"' && !c.is_whitespace()),
    )(input)
}

fn parse_whitespace_separated(input: &str) -> IResult<&str, Vec<&str>> {
    context(
        "whitespace separated",
        separated_list1(multispace1, parse_token),
    )(input)
}

fn parse_between_quotes(input: &str) -> IResult<&str, Vec<&str>> {
    context(
        "between quotes",
        alt((
            delimited(tag("\""), parse_whitespace_separated, tag("\"")),
            delimited(tag("'"), parse_whitespace_separated, tag("'")),
        )),
    )(input)
}

fn parse_value(input: &str) -> IResult<&str, Vec<&str>> {
    context(
        "value",
        alt((parse_between_quotes, parse_whitespace_separated)),
    )(input)
}

fn parse_alias(input: &str) -> IResult<&str, Alias> {
    context("alias", separated_pair(parse_name, tag("="), parse_value))(input)
        .map(|(input, (name, value))| (input, Alias::new(name, value)))
}

#[derive(Debug)]
struct Aliases {
    aliases: HashMap<String, Vec<Alias>>,
}

impl Aliases {
    fn new() -> Self {
        Self {
            aliases: HashMap::new(),
        }
    }

    fn push(&mut self, alias: Alias) {
        let aliased_command_name = alias.value[0].clone();
        let aliases = self
            .aliases
            .entry(aliased_command_name)
            .or_insert_with(Vec::new);
        aliases.push(alias);
    }
}

fn print_header(title: &str, color: Option<Colors>) {
    match color {
        Some(color) => println!("[{}]", title.color(color)),
        None => println!("[{}]", title),
    }
}

fn format_aliases(in_color: bool) -> Result<()> {
    let mut aliases: Aliases = Aliases::new();
    let mut error_aliases: Vec<String> = Vec::new();
    let mut line = String::new();
    while std::io::stdin().read_line(&mut line)? != 0 {
        let line = std::mem::take(&mut line);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match parse_alias(&line) {
            Ok((_, alias)) => {
                aliases.push(alias);
            }
            Err(_) => {
                error_aliases.push(line.to_string());
            }
        }
    }

    let mut general_aliases = Vec::new();
    for (aliased_command, aliases) in aliases.aliases.iter_mut() {
        if aliases.len() == 1 {
            general_aliases.push(&aliases[0]);
            continue;
        }

        let header_color = if in_color {
            Some(Colors::YellowFg)
        } else {
            None
        };
        print_header(aliased_command, header_color);
        aliases.sort();
        for alias in aliases {
            println!("  {}", alias.display(in_color));
        }
        println!("")
    }

    if !general_aliases.is_empty() {
        let header_color = if in_color {
            Some(Colors::YellowFg)
        } else {
            None
        };
        print_header("general", header_color);
        general_aliases.sort();
        for alias in general_aliases {
            println!("  {}", alias.display(in_color));
        }
        println!("");
    }

    if !error_aliases.is_empty() {
        let header_color = if in_color {
            Some(Colors::BrightBlackFg)
        } else {
            None
        };
        print_header("unparsable", header_color);
        for alias in error_aliases {
            eprintln!("  {}", alias);
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let in_color = !no_color::is_no_color();
    let args = Args::parse();
    match args.command {
        Some(command) => match command {
            Command::Init { shell } => {
                print_function(&shell)?;
            }
            Command::Format => {
                format_aliases(in_color)?;
            }
        },
        None => {
            format_aliases(in_color)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_name() {
        assert_eq!(parse_name("alias"), Ok(("", "alias")));
        assert_eq!(parse_name("alias="), Ok(("=", "alias")));
        assert_eq!(parse_name("alias foo"), Ok(("", "alias foo")));
        assert_eq!(parse_name("alias foo="), Ok(("=", "alias foo")));
    }

    #[test]
    fn test_parse_whitespace_separated() {
        assert_eq!(parse_whitespace_separated("foo"), Ok(("", vec!["foo"])));
        assert_eq!(
            parse_whitespace_separated("foo bar"),
            Ok(("", vec!["foo", "bar"]))
        );
        assert_eq!(
            parse_whitespace_separated("foo bar baz"),
            Ok(("", vec!["foo", "bar", "baz"]))
        );
    }

    #[test]
    fn test_parse_between_quotes() {
        assert_eq!(
            parse_between_quotes("\"foo bar\""),
            Ok(("", vec!["foo", "bar"]))
        );
        assert_eq!(
            parse_between_quotes("'foo bar'"),
            Ok(("", vec!["foo", "bar"]))
        );
        assert_eq!(
            parse_between_quotes("\"foo bar\" baz"),
            Ok((" baz", vec!["foo", "bar"]))
        );
        assert_eq!(
            parse_between_quotes("'foo bar' baz"),
            Ok((" baz", vec!["foo", "bar"]))
        );
    }

    #[test]
    fn test_parse_value() {
        assert_eq!(parse_value("foo"), Ok(("", vec!["foo"])));
        assert_eq!(parse_value("foo bar"), Ok(("", vec!["foo", "bar"])));
        assert_eq!(
            parse_value("foo bar baz"),
            Ok(("", vec!["foo", "bar", "baz"]))
        );
        assert_eq!(parse_value("\"foo bar\""), Ok(("", vec!["foo", "bar"])));
        assert_eq!(parse_value("'foo bar'"), Ok(("", vec!["foo", "bar"])));
        assert_eq!(
            parse_value("\"foo bar\" baz"),
            Ok((" baz", vec!["foo", "bar"]))
        );
        assert_eq!(
            parse_value("'foo bar' baz"),
            Ok((" baz", vec!["foo", "bar"]))
        );
    }

    #[test]
    fn test_parse_alias() {
        assert_eq!(
            parse_alias("foo='bar=baz'"),
            Ok(("", Alias::new("foo", vec!["bar=baz"])))
        );
        assert_eq!(
            parse_alias("foo=bar=baz"),
            Ok(("", Alias::new("foo", vec!["bar=baz"])))
        );
        assert_eq!(
            parse_alias("foo='bar baz qux'"),
            Ok(("", Alias::new("foo", vec!["bar", "baz", "qux"])))
        );
        assert_eq!(
            parse_alias("foo='bar; baz'"),
            Ok(("", Alias::new("foo", vec!["bar;", "baz"])))
        );
        assert_eq!(
            parse_alias("foo='bar baz qux quux corge'"),
            Ok((
                "",
                Alias::new("foo", vec!["bar", "baz", "qux", "quux", "corge"])
            ))
        );
    }

    #[test]
    fn test_trailing_whitespace() {
        assert_eq!(
            parse_alias("foo=bar   "),
            Ok(("   ", Alias::new("foo", vec!["bar"])))
        );
    }

    #[test]
    fn test_parse_real_aliases() {
        assert_eq!(
            parse_alias("vim=nvim"),
            Ok(("", Alias::new("vim", vec!["nvim"])))
        );
        assert_eq!(
            parse_alias("tpath='path --tree'"),
            Ok(("", Alias::new("tpath", vec!["path", "--tree"])))
        );
        assert_eq!(
            parse_alias("tpath=\"path --tree\""),
            Ok(("", Alias::new("tpath", vec!["path", "--tree"])))
        );
        assert_eq!(
            parse_alias("make='colorify make --warn-undefined-variables'"),
            Ok((
                "",
                Alias::new(
                    "make",
                    vec!["colorify", "make", "--warn-undefined-variables"]
                )
            ))
        );
        assert_eq!(
            parse_alias("..='cd ..'"),
            Ok(("", Alias::new("..", vec!["cd", ".."])))
        );
        assert_eq!(
            parse_alias("..2='cd ../..'"),
            Ok(("", Alias::new("..2", vec!["cd", "../.."])))
        );
        assert_eq!(
            parse_alias(":q=exit"),
            Ok(("", Alias::new(":q", vec!["exit"])))
        );
    }

    /*
    * TODO: get tests to pass
    #[test]
    fn parse_escaped_quote() {
        assert_eq!(parse_token("\""), Ok(("", "\"")));
    }

    #[test]
    fn test_really_hard_alias() {
        assert_eq!(
            parse_whitespace_separated("echo \"use \\\"trash\\\" instead\""),
            Ok(("", vec!["rm", "echo", "use", "\"trash\"", "instead"]))
        );

        assert_eq!(
            parse_alias("rm='echo \"use \\\"trash\\\" instead\"'"),
            Ok((
                "",
                Alias::new("rm", vec!["rm", "echo", "use", "\"trash\"", "instead"])
            ))
        );
    }
    */
}
