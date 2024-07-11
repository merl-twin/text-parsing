use crate::{
    Snip, Localize, Local,
    PipeParser,
    Error,
};



pub trait Source {
    fn next_char(&mut self) -> SourceResult;
}

pub trait IntoSource {
    type Source: Source;
    
    fn into_source(self) -> Self::Source;
}

pub trait Sourcefy {
    fn sourcefy(self) -> SourceEvent;
}

pub type SourceResult =  Result<Option<Local<SourceEvent>>,Error>;

#[derive(Debug,Clone,Copy,Eq,PartialEq)]
// Inclusive: Sentence = sentence breaker + word breaker, etc.
pub enum Breaker {
    None,
    Word,
    Sentence,
    Paragraph,
    Section,
}

#[derive(Debug,Eq,PartialEq)]
pub enum SourceEvent {
    Char(char),
    Breaker(Breaker),
}
impl Sourcefy for char {
    fn sourcefy(self) -> SourceEvent {
        SourceEvent::Char(self)
    }
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
            let chars = Snip { offset: char_index, length: 1 };
            let bytes = Snip { offset: byte_index, length: c.len_utf8() };
            c.sourcefy().localize(chars,bytes)
        }))
    }
}

impl<T: Source> SourceExt for T {}

pub trait SourceExt: Source + Sized {
    fn pipe<P>(self, parser: P) -> Pipe<Self,P>
    where P: PipeParser
    {
        Pipe {
            source: self,
            parser,
        }
    }
    fn filter_char<F>(self, filter: F) -> Filter<Self,F>
    where F: FnMut(char) -> Option<char>
    {
        Filter {
            source: self,
            filter,
        }
    }
    fn into_separator(self) -> IntoSeparator<Self> {
        IntoSeparator {
            source: self,
            buffer: None,
            current: None,
        }
    }
}



pub struct Pipe<S,P>
{
    source: S,
    parser: P,
}
impl<S,P> Source for Pipe<S,P>
where S: Source,
      P: PipeParser
{
    fn next_char(&mut self) -> SourceResult {
        self.parser.next_char(&mut self.source)
    }
}

pub struct Filter<S,F> {
    source: S,
    filter: F,
}
impl<S,F> Source for Filter<S,F>
where S: Source,
      F: FnMut(char) -> Option<char>
{
    fn next_char(&mut self) -> SourceResult {
        loop {
            match self.source.next_char()? {
                Some(local_se) => {
                    let (local,se) = local_se.into_inner();
                    match se {
                        SourceEvent::Char(c) => match (&mut self.filter)(c) {
                            Some(c) => break Ok(Some(local.with_inner(SourceEvent::Char(c)))),
                            None => continue,
                        },
                        SourceEvent::Breaker(b) => break Ok(Some(local.with_inner(SourceEvent::Breaker(b)))),
                    }
                },
                None => break Ok(None),
            }
        }
    }
}

use unicode_properties::{
    UnicodeGeneralCategory,
    GeneralCategory,
};

/*

   Mapping some chars into breakers:

      \n      => Breaker::Sentence
   
   Chars by unicode properties:
       
      Cc | Zs => Breaker::Word
      Zl      => Breaker::Sentence                    
      Zp      => Breaker::Paragraph

   Merging Breakers:
      
      1) Breaker::Sentence + Breaker::Sentence = Breaker::Paragraph
      2) Merge by "Inclusiveness": Sentence = sentence breaker + word breaker, etc.

*/

pub struct IntoSeparator<S> {
    source: S,
    buffer: Option<Local<SourceEvent>>,
    current: Option<(Local<()>,Breaker)>,
}
impl<S> Source for IntoSeparator<S>
where S: Source
{
    fn next_char(&mut self) -> SourceResult {
        fn merge_breakers(cur_loc: Local<()>, cur_b: Breaker, nxt_loc: Local<()>, nxt_b: Breaker) -> Result<(Local<()>,Breaker),Error> {
            Ok(match (cur_b,nxt_b) {
                (Breaker::Sentence,Breaker::Sentence) => {
                    let loc = Local::from_segment(cur_loc,nxt_loc)?;
                    (loc,Breaker::Paragraph)
                },
                (Breaker::None,_) => (nxt_loc,nxt_b),
                (_,Breaker::None) => (cur_loc,cur_b),
                (Breaker::Word,_) => (nxt_loc,nxt_b),
                (_,Breaker::Word) => (cur_loc,cur_b),
                (Breaker::Sentence,_) => (nxt_loc,nxt_b),
                (_,Breaker::Sentence) => (cur_loc,cur_b),
                (Breaker::Paragraph,_) => (nxt_loc,nxt_b),
                (_,Breaker::Paragraph) => (cur_loc,cur_b),
                (Breaker::Section,Breaker::Section) => (nxt_loc,nxt_b),
            })
        }

        
        loop {
            match self.buffer.take() {
                Some(lse) => break Ok(Some(lse)),
                None => {
                    match self.source.next_char()? {
                        Some(local_se) => {                            
                            let (local,se) = local_se.map(|se| {
                                match se {
                                    SourceEvent::Char(c) => {
                                        match c {
                                            '\n' => SourceEvent::Breaker(Breaker::Sentence),
                                            _ => match c.general_category() {
                                                GeneralCategory::Control |
                                                GeneralCategory::SpaceSeparator => SourceEvent::Breaker(Breaker::Word),
                                                GeneralCategory::LineSeparator => SourceEvent::Breaker(Breaker::Sentence),                    
                                                GeneralCategory::ParagraphSeparator => SourceEvent::Breaker(Breaker::Paragraph),
                                                _ => SourceEvent::Char(c),
                                            },
                                        }
                                    },
                                    b @ SourceEvent::Breaker(..) => b,
                                }
                            }).into_inner();
                            match se {
                                c @ SourceEvent::Char(..) => match self.current.take() {
                                    Some((local_br,br)) => {
                                        self.buffer = Some(local.with_inner(c));
                                        break Ok(Some(local_br.with_inner(SourceEvent::Breaker(br))));
                                    },
                                    None => break Ok(Some(local.with_inner(c))),
                                },
                                SourceEvent::Breaker(br) => match self.current.take() {
                                    Some((c_local,c_br)) => {
                                        self.current = Some(merge_breakers(c_local,c_br,local,br)?);
                                    },
                                    None => {
                                        self.current = Some((local,br));
                                    },
                                },
                            }
                        },
                        None => match self.current.take() {
                            Some((local,br)) => break Ok(Some(local.with_inner(SourceEvent::Breaker(br)))),
                            None => break Ok(None),
                        },
                    }
                },
            }
        }
    }
}



#[cfg(test)]
mod tests {
    use crate::*;
    use super::*;

    use unicode_properties::{
        UnicodeGeneralCategory,
        GeneralCategory,
    };
    
    #[test]
    fn basic() {
        let mut src = " &GreaterGreater; &#x09; &#128175; &#xFEFF; &#x200d; &#x200c; &#xf8e6; &#x2764;"
            .into_source()
            .pipe(crate::entities::Builder::new().create().into_piped())
            .filter_char(|c| {
                match c.general_category() {
                    GeneralCategory::Format if c != '\u{200d}' => None,
                    GeneralCategory::Unassigned => None,
                    _ if c == '\u{f8e6}' => None,
                    _ => Some(c),
                }
            });

        let mut res_iter = [
            SourceEvent::Char(' ').localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            SourceEvent::Char('⪢').localize(Snip { offset: 1, length: 16 },Snip { offset: 1, length: 16 }),
            SourceEvent::Char(' ').localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
            SourceEvent::Char('\t').localize(Snip { offset: 18, length: 6 },Snip { offset: 18, length: 6 }),
            SourceEvent::Char(' ').localize(Snip { offset: 24, length: 1 },Snip { offset: 24, length: 1 }),
            SourceEvent::Char('💯').localize(Snip { offset: 25, length: 9 },Snip { offset: 25, length: 9 }),
            SourceEvent::Char(' ').localize(Snip { offset: 34, length: 1 },Snip { offset: 34, length: 1 }),
            SourceEvent::Char(' ').localize(Snip { offset: 43, length: 1 },Snip { offset: 43, length: 1 }),
            SourceEvent::Char('\u{200d}').localize(Snip { offset: 44, length: 8 },Snip { offset: 44, length: 8 }),
            SourceEvent::Char(' ').localize(Snip { offset: 52, length: 1 },Snip { offset: 52, length: 1 }),
            SourceEvent::Char(' ').localize(Snip { offset: 61, length: 1 },Snip { offset: 61, length: 1 }),
            SourceEvent::Char(' ').localize(Snip { offset: 70, length: 1 },Snip { offset: 70, length: 1 }),
            SourceEvent::Char('❤').localize(Snip { offset: 71, length: 8 },Snip { offset: 71, length: 8 }),
        ].into_iter();        

        while let Some(local_event) = src.next_char().unwrap() {
            //let (local,event) = local_event.into_inner();
            //println!("SourceEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Source: {:?}",local_event);
                    println!("Result: {:?}",ev);
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

    #[test]
    fn basic_breaker() {
        let mut src = " &GreaterGreater; &#x09; &#128175; &#xFEFF; &#x200d; &#x200c; &#xf8e6; &#x2764; "
            .into_source()
            .pipe(crate::entities::Builder::new().create().into_piped())
            .filter_char(|c| {
                match c.general_category() {
                    GeneralCategory::Format if c != '\u{200d}' => None,
                    GeneralCategory::Unassigned => None,                    
                    _ if c == '\u{f8e6}' => None,
                    _ => Some(c),
                }
            })
            .into_separator();

        let mut res_iter = [
            SourceEvent::Breaker(Breaker::Word).localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            SourceEvent::Char('⪢').localize(Snip { offset: 1, length: 16 },Snip { offset: 1, length: 16 }),
            SourceEvent::Breaker(Breaker::Word).localize(Snip { offset: 24, length: 1 },Snip { offset: 24, length: 1 }),
            SourceEvent::Char('💯').localize(Snip { offset: 25, length: 9 },Snip { offset: 25, length: 9 }),
            SourceEvent::Breaker(Breaker::Word).localize(Snip { offset: 43, length: 1 },Snip { offset: 43, length: 1 }),
            SourceEvent::Char('\u{200d}').localize(Snip { offset: 44, length: 8 },Snip { offset: 44, length: 8 }),
            SourceEvent::Breaker(Breaker::Word).localize(Snip { offset: 70, length: 1 },Snip { offset: 70, length: 1 }),
            SourceEvent::Char('❤').localize(Snip { offset: 71, length: 8 },Snip { offset: 71, length: 8 }),
            SourceEvent::Breaker(Breaker::Word).localize(Snip { offset: 79, length: 1 },Snip { offset: 79, length: 1 }),
        ].into_iter();        

        while let Some(local_event) = src.next_char().unwrap() {
            //let (local,event) = local_event.into_inner();
            //println!("SourceEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Source: {:?}",local_event);
                    println!("Result: {:?}",ev);
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

    #[test]
    fn basic_breaker_2() {
        let mut src = " &GreaterGreater; &#x09;\n &#128175; &#xFEFF; &#x200d; \n &#x200c; \n &#xf8e6; &#x2764; "
            .into_source()
            .pipe(crate::entities::Builder::new().create().into_piped())
            .filter_char(|c| {
                match c.general_category() {
                    GeneralCategory::Format if c != '\u{200d}' => None,
                    GeneralCategory::Unassigned => None,                    
                    _ if c == '\u{f8e6}' => None,
                    _ => Some(c),
                }
            })
            .into_separator();

        let mut res_iter = [
            SourceEvent::Breaker(Breaker::Word).localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            SourceEvent::Char('⪢').localize(Snip { offset: 1, length: 16 },Snip { offset: 1, length: 16 }),
            SourceEvent::Breaker(Breaker::Sentence).localize(Snip { offset: 24, length: 1 },Snip { offset: 24, length: 1 }),
            SourceEvent::Char('💯').localize(Snip { offset: 26, length: 9 },Snip { offset: 26, length: 9 }),
            SourceEvent::Breaker(Breaker::Word).localize(Snip { offset: 44, length: 1 },Snip { offset: 44, length: 1 }),
            SourceEvent::Char('\u{200d}').localize(Snip { offset: 45, length: 8 },Snip { offset: 45, length: 8 }),
            SourceEvent::Breaker(Breaker::Paragraph).localize(Snip { offset: 54, length: 12 },Snip { offset: 54, length: 12 }),
            SourceEvent::Char('❤').localize(Snip { offset: 76, length: 8 },Snip { offset: 76, length: 8 }),
            SourceEvent::Breaker(Breaker::Word).localize(Snip { offset: 84, length: 1 },Snip { offset: 84, length: 1 }),
        ].into_iter();        

        while let Some(local_event) = src.next_char().unwrap() {
            //let (local,event) = local_event.into_inner();
            //println!("SourceEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Source: {:?}",local_event);
                    println!("Result: {:?}",ev);
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

}
