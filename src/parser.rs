pub type ParseResult<'i, T> = Result<(&'i str, T), ParseError<'i>>;

#[derive(Debug)]
pub enum Combinator {
    Custom(String),
    Character(char),
    Alpha,
    Num,
    Tag(String),
    Whitespace,
}

#[derive(Debug)]
pub struct ParseError<'i> {
    pub input: &'i str,
    pub combinator: Combinator,
}

pub fn character<'i>(c: char) -> impl FnOnce(&'i str) -> ParseResult<char> {
    move |input: &'i str| {
        if let Some(input) = input.strip_prefix(c) {
            Ok((input, c))
        } else {
            Err(ParseError {
                input,
                combinator: Combinator::Character(c),
            })
        }
    }
}

pub fn alpha<'i>() -> impl FnOnce(&'i str) -> ParseResult<char> {
    |input: &'i str| {
        let res = (['a'..='z', 'A'..='Z'])
            .map(|range| range.map(|c| character(c)(input)).find(Result::is_ok));

        match res {
            [Some(ok), None] | [None, Some(ok)] => ok,
            [None, None] => Err(ParseError {
                input,
                combinator: Combinator::Alpha,
            }),
            [Some(_), Some(_)] => unreachable!(),
        }
    }
}

pub fn num<'i>() -> impl FnOnce(&'i str) -> ParseResult<char> {
    |input: &'i str| {
        let res = ('0'..='9')
            .map(|c| character(c)(input))
            .find(Result::is_ok)
            .ok_or(ParseError {
                input,
                combinator: Combinator::Num,
            });

        match res {
            Ok(res) => res,
            Err(e) => Err(e),
        }
    }
}

pub fn alphanum<'i>() -> impl FnOnce(&'i str) -> ParseResult<char> {
    |input| either(alpha(), num())(input)
}

pub fn tag<'i, 't>(tag: &'t str) -> impl FnOnce(&'i str) -> ParseResult<&'t str>
where
    'i: 't,
{
    move |input: &'i str| {
        if let Some(input) = input.strip_prefix(tag) {
            Ok((input, tag))
        } else {
            Err(ParseError {
                input,
                combinator: Combinator::Tag(tag.to_owned()),
            })
        }
    }
}

pub fn whitespace(input: &str) -> Result<(&str, ()), ParseError> {
    if let Some(input) = input.strip_prefix(' ') {
        Ok((input, ()))
    } else {
        Err(ParseError {
            input,
            combinator: Combinator::Whitespace,
        })
    }
}

pub fn either<'i, L, R, T>(lp: L, rp: R) -> impl FnOnce(&'i str) -> ParseResult<T>
where
    L: FnOnce(&'i str) -> ParseResult<T>,
    R: FnOnce(&'i str) -> ParseResult<T>,
{
    move |input: &'i str| match lp(input) {
        Ok(res) => Ok(res),
        Err(_) => rp(input),
    }
}
