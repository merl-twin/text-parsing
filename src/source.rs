use crate::{
    Pos, Localize,
    SourceResult, Sourcefy,
    PipeParser,
};

pub trait Source {
    fn next_char(&mut self) -> SourceResult;
}

pub trait IntoSource {
    type Source: Source;
    
    fn into_source(self) -> Self::Source;
}

impl<'s> IntoSource for &'s str {
    type Source = StrSource<'s>;
    fn into_source(self) -> Self::Source {
        StrSource(self.char_indices().enumerate())
    }
}

impl<'s> IntoSource for &'s String {
    type Source = StrSource<'s>;
    fn into_source(self) -> Self::Source {
        StrSource(self.char_indices().enumerate())
    }
}

pub struct StrSource<'s> (std::iter::Enumerate<std::str::CharIndices<'s>>);
impl<'s> Source for StrSource<'s> {
    fn next_char(&mut self) -> SourceResult {
        Ok(self.0.next().map(|(char_index,(byte_index,c))| {
            let chars = Pos { offset: char_index, length: 1 };
            let bytes = Pos { offset: byte_index, length: c.len_utf8() };
            c.sourcefy().localize(chars,bytes)
        }))
    }
}

impl<T: Source> SourceExt for T {}

pub trait SourceExt: Source + Sized {
    fn map<P>(self, parser: P) -> Map<Self,P>
    where P: PipeParser
    {
        Map {
            source: self,
            parser,
        }
    }
}

pub struct Map<S,P>
{
    source: S,
    parser: P,
}
impl<S,P> Source for Map<S,P>
where S: Source,
      P: PipeParser
{
    fn next_char(&mut self) -> SourceResult {
        self.parser.next_char(&mut self.source)
    }
}

