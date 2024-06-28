use crate::{
    Pos, Local, Localize,
    Error,
};

pub trait Source {
    fn next_char(&mut self) -> Result<Option<Local<char>>,Error>;
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
    fn next_char(&mut self) -> Result<Option<Local<char>>,Error> {
        Ok(self.0.next().map(|(char_index,(byte_index,c))| {
            let chars = Pos { offset: char_index, length: 1 };
            let bytes = Pos { offset: byte_index, length: c.len_utf8() };
            c.localize(chars,bytes)
        }))
    }
}
