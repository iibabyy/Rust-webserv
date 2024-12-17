extern crate nom;

use std::{collections::HashMap, path::PathBuf};

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{char, multispace0, space1},
    multi::many0,
    sequence::preceded,
    IResult,
};

use crate::LocationBlock;

use super::ServerBlock;

// Définition des symboles autorisés dans les identifiants
#[allow(unused)]
fn symbols(input: &str) -> IResult<&str, &str> {
    alt((tag("/"), tag("."), tag("-"), tag("_")))(input)
}

// Définition des symboles autorisés dans les identifiants
fn modifier(input: &str) -> IResult<&str, &str> {
    alt((tag("="), tag("~")))(input)
}

// Identifiant : chaîne alphanumérique avec symboles autorisés
fn identifier(input: &str) -> IResult<&str, &str> {
    take_while(|c: char| c.is_ascii_whitespace() == false && c != ';')(input)
}

// Espaces blancs optionnels
fn whitespace(input: &str) -> IResult<&str, &str> {
    multispace0(input)
}

// Espaces blancs optionnels
fn space(input: &str) -> IResult<&str, &str> {
    space1(input)
}

fn has_values(identifiant: &str) -> bool {
    return identifiant != "internal";
}

// Directive avec un identifiant et des espaces blancs entre l'identifiant et les valeurs
fn directive(mut input: &str) -> IResult<&str, (String, Vec<String>)> {
    input = skip_whitespaces(input);
    let (mut input, id) = identifier(input)?;
    let mut values = Vec::new();
    if has_values(id) {
        (input, values) = many0(preceded(space1, identifier))(input)?;
    }
    input = skip_spaces(input);
    (input, _) = char(';')(input)?;
    Ok((
        input,
        (
            id.to_owned(),
            values
                .iter()
                .filter(|str| str.is_empty() == false)
                .map(|str| str.to_string())
                .collect(),
        ),
    ))
}

fn skip_whitespaces(input: &str) -> &str {
    match whitespace(input) {
        Ok((input, _)) => input,
        Err(_) => input,
    }
}

fn skip_spaces(input: &str) -> &str {
    match space(input) {
        Ok((input, _)) => input,
        Err(_) => input,
    }
}

// Bloc de type "server" ou "location"
fn block(input: &str) -> IResult<&str, ServerBlock> {
    let mut directives: HashMap<String, Vec<String>> = HashMap::new();
    let mut locations: HashMap<String, LocationBlock> = HashMap::new();
    let mut cgi: HashMap<String, PathBuf> = HashMap::new();
    let input = skip_whitespaces(input);
    let (input, _) = tag("server")(input)?;
    let input = skip_whitespaces(input);
    let (input, _) = char('{')(input)?;
    let mut input = skip_whitespaces(input);

    let mut found = true;
    while found == true {
        found = false;
        input = match directive(input) {
            Ok((new_input, directive)) => {
                if directive.0 == "cgi" {
                    if directive.1.len() != 2 {
                        return Err(nom::Err::Error(nom::error::Error::new(
                            input,
                            nom::error::ErrorKind::Fail,
                        )));
                    }
                    cgi.insert(
                        directive.1[0].clone(),
                        PathBuf::from(directive.1[1].as_str()),
                    );
                } else {
                    directives.insert(directive.0, directive.1);
                }
                found = true;
                new_input
            }
            Err(_) => input,
        };
        input = match location_block(input) {
            Ok((new_input, location)) => {
                if locations.contains_key(location.path.as_str()) {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Fail,
                    )));
                }
                locations.insert(location.path.clone(), location);
                found = true;
                new_input
            }
            Err(_) => input,
        };
    }
    input = skip_whitespaces(input);
    let (input, _) = char('}')(input)?;
    Ok((
        input,
        ServerBlock {
            locations: locations,
            directives: directives,
            cgi: cgi,
        },
    ))
}

// Bloc de type "location" : Cette fonction retourne une directive avec un identifiant et une liste de valeurs
fn location_block(mut input: &str) -> IResult<&str, LocationBlock> {
    input = skip_whitespaces(input);
    (input, _) = tag("location")(input)?;
    input = skip_spaces(input);
    let (mut input, modifier) = match modifier(input) {
        Ok((input, modifier)) => (input, Some(modifier.to_owned())),
        _ => (input, None),
    };
    input = skip_spaces(input);
    let (mut input, path) = identifier(input)?;
    input = skip_whitespaces(input);
    (input, _) = char('{')(input)?;
    let (mut input, directives) = many0(directive)(input)?;
    input = skip_whitespaces(input);
    (input, _) = char('}')(input)?;

    let mut infos: HashMap<String, Vec<String>> = HashMap::new();
    let mut cgi: HashMap<String, PathBuf> = HashMap::new();

    for directive in directives {
        if directive.0 == "cgi" {
            if directive.1.len() != 2 {
                return Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Fail,
                )));
            }
            cgi.insert(
                directive.1[0].clone(),
                PathBuf::from(directive.1[1].as_str()),
            );
        } else {
            infos.insert(directive.0, directive.1);
        }
    }

    input = skip_whitespaces(input);
    // Crée un bloc de type "location" similaire à une directive
    Ok((
        input,
        LocationBlock {
            modifier: modifier,
            path: path.to_owned(),
            directives: infos,
            cgi: cgi,
        },
    ))
}

// Fichier de configuration : commence avec SOI (Start of Input) et finit avec EOI (End of Input)
pub fn config(input: &str) -> IResult<&str, Vec<ServerBlock>> {
    let (input, servs) = many0(block)(input)?;

    let input = skip_whitespaces(input);
    if input.is_empty() == false {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Fail,
        )))
    } else {
        Ok((input, servs))
    }
}
