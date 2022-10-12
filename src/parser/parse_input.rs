use nom::branch::alt;
use nom::bytes::complete::{tag, take_while, take_while1};
use nom::character::complete::{alpha1, line_ending, multispace0, not_line_ending};
use nom::character::streaming::char;
use nom::combinator::{peek, value};
use nom::error::ParseError;
use nom::multi::{many0, many0_count, many1, many_m_n, separated_list0, separated_list1};
use nom::sequence::{delimited, tuple};
use nom::IResult;

type Weight = u8;

#[derive(Debug)]
pub struct RawParserInput {
    pub transitions: Vec<RawParserTransition>,
    pub modules: Vec<RawParserModule>,
}

#[derive(Debug)]
pub struct RawParserTransition {
    pub name: String,
    pub input_places: Vec<RawParserPlace>,
    pub output_places: Vec<RawParserPlace>,
}

#[derive(Debug)]
pub struct RawParserModule {
    pub name: String,
    pub places: Vec<RawParserPlace>,
}

#[derive(Debug)]
pub struct RawParserPlace {
    pub name: String,
    pub weight: Weight,
}

pub fn parse(input: &str) -> IResult<&str, RawParserInput> {
    let (input, _) = many0_count(comment)(input)?;
    let (input, transitions) = transitions(input)?;
    let (input, modules) = many1(module)(input)?;

    let raw_parser_input = RawParserInput {
        transitions,
        modules,
    };

    Ok((input, raw_parser_input))
}

/// Parse Petrinet
/// { <transitionline> \n ... }
fn transitions(input: &str) -> IResult<&str, Vec<RawParserTransition>> {
    let opening = char('{');
    let closing = char('}');
    let transitionlines = many1(transitionline);

    let (input, (_, _, transitions, _, _)) = tuple((
        opening,
        line_ending,
        transitionlines,
        many0(line_ending),
        closing,
    ))(input)?;

    Ok((input, transitions))
}

/// Parse Module
/// <Name> { <place1>, <place2>(<count>), ... }
fn module(input: &str) -> IResult<&str, RawParserModule> {
    let opening = |i| char('{')(i);
    let closing = |i| ws(tag("}"))(i);

    let places_parser = |i| {
        separated_list1(
            alt((tag(","), tag("\n"), tag("\r\n"))),
            place_with_optional_number,
        )(i)
    };

    let (input, (_, name, _, places, _, _)) = tuple((
        many0(line_ending),
        ws(name),
        opening,
        places_parser,
        many0(line_ending),
        closing,
    ))(input)?;

    let raw_parser_module = RawParserModule {
        name: name.to_string(),
        places: set_default_value_for_vec(places, 0),
    };

    Ok((input, raw_parser_module))
}

/// Parse a transitionline: transitions with arcs, pre- and postplaces
/// <transition>: <place1>, <place2>(<count>), ... -> <place1>, ...
fn transitionline<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, RawParserTransition, E> {
    let name_parser = name;
    let colon = char(':');
    let places_parser = |i| separated_list0(char(','), place_with_optional_number)(i);
    let arrow = ws(tag("->"));

    let (input, (transition_name, _, input_places, _, output_places)) =
        tuple((name_parser, colon, places_parser, arrow, places_parser))(input)?;

    let proto_transition = RawParserTransition {
        name: transition_name.to_string(),
        input_places: set_default_value_for_vec(input_places, 1),
        output_places: set_default_value_for_vec(output_places, 1),
    };

    Ok((input, proto_transition))
}

/// Parse a place with optional Number
fn place_with_optional_number<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (String, Option<Weight>), E> {
    let (input, name) = name(input)?;
    let weight_parser = tuple((char('('), take_while1(char::is_numeric), char(')')));
    let (input, number) = many_m_n(0, 1, weight_parser)(input)?;

    let weight: Option<Weight>;
    if number.len() > 0 {
        weight = Some(number[0].1.parse().unwrap());
    } else {
        weight = None;
    }
    let raw_parser_place = (name.to_string(), weight);
    Ok((input, raw_parser_place))
}

/// Parse a name
/// with isalphanumerical or underscore
fn name<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &str, E> {
    let (i, (_, _, name, _)) = tuple((
        multispace0,
        peek(alpha1),
        take_while(|i: char| i.is_alphanumeric() || i == '_'),
        multispace0,
    ))(i)?;
    Ok((i, name))
}

/// Parse a comment
/// #This is a comment until a linebreak
fn comment<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, (), E> {
    value((), tuple((char('#'), not_line_ending, line_ending)))(i)
}

/// Trim, ignore whitespaces before and after
fn ws<'a, F, O, E: ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

fn set_default_value_for_vec(
    input: Vec<(String, Option<Weight>)>,
    value: Weight,
) -> Vec<RawParserPlace> {
    input
        .into_iter()
        .map(|p| set_default_value(p, value))
        .collect::<Vec<RawParserPlace>>()
}

fn set_default_value(input: (String, Option<Weight>), value: Weight) -> RawParserPlace {
    RawParserPlace {
        name: input.0.to_string(),
        weight: match input.1 {
            Some(x) => x,
            None => value,
        },
    }
}
