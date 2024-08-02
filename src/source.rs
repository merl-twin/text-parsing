use crate::{
    Snip, Localize, Local,
    PipeParser,
    Error,
};

#[derive(Debug,Clone,Copy,Default,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub struct Processed {
    pub chars: usize,
    pub bytes: usize,
}

pub trait Source {
    fn next_char(&mut self) -> SourceResult;
    fn processed(&self) -> Processed;
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
    Space,
    Word,
    Line,
    Sentence,
    Paragraph,
    Section,
}

#[derive(Debug,Clone,Copy,Eq,PartialEq)]
pub enum SourceEvent {
    Char(char),
    Breaker(Breaker),
}
impl Sourcefy for char {
    fn sourcefy(self) -> SourceEvent {
        SourceEvent::Char(self)
    }
}
impl Sourcefy for Breaker {
    fn sourcefy(self) -> SourceEvent {
        SourceEvent::Breaker(self)
    }
}

impl Breaker {
    pub fn into_source_as(self, s: &str) -> OptSource {
        let blen = s.len();
        let clen = s.chars().count();
        OptSource::new(self.sourcefy().localize(Snip{ offset: 0, length: clen }, Snip{ offset: 0, length: blen }))
    }
}

impl IntoSource for char {
    type Source = OptSource;
    fn into_source(self) -> Self::Source {
        let blen = self.len_utf8();
        OptSource::new(self.sourcefy().localize(Snip{ offset: 0, length: 1 }, Snip{ offset: 0, length: blen }))
    }
}

pub struct EmptySource;
impl Source for EmptySource {
    fn next_char(&mut self) -> SourceResult {
        Ok(None)
    }
    fn processed(&self) -> Processed {
        Processed::default()
    }
}

impl<S> Source for Option<S>
where S: Source
{
    fn next_char(&mut self) -> SourceResult {
        match self {
            Some(source) => source.next_char(),
            None => Ok(None),
        }
    }
    fn processed(&self) -> Processed {
        match self {
            Some(source) => source.processed(),
            None => Processed::default(),
        }
    }
}

pub struct ParserSource<'p,'s,P,S> {
    parser: &'p mut P,
    source: &'s mut S,
}
impl<'p,'s,P,S> ParserSource<'p,'s,P,S> {
    pub fn new<'a,'b>(parser: &'a mut P, source: &'b mut S) -> ParserSource<'a,'b,P,S> {
        ParserSource { parser, source }
    }
}
impl<'p,'s,P,S> Source for ParserSource<'p,'s,P,S>
where P: PipeParser,
      S: Source
{
    fn next_char(&mut self) -> SourceResult {
        self.parser.next_char(self.source)
    }
    fn processed(&self) -> Processed {
        self.source.processed()
    }
}

pub struct OptSource {
    source: Option<Local<SourceEvent>>,
    done: Processed,
}
impl OptSource {
    pub fn new(local_se: Local<SourceEvent>) -> OptSource {
        OptSource {
            source: Some(local_se),
            done: Processed::default(),
        }
    }
}
impl Source for OptSource {
    fn next_char(&mut self) -> SourceResult {
        let r = self.source.take();
        if let Some(local_se) = &r {
            self.done.chars += local_se.chars().length;
            self.done.bytes += local_se.bytes().length;
        }
        Ok(r)
    }
    fn processed(&self) -> Processed {
        self.done
    }
}

impl<'s> IntoSource for &'s str {
    type Source = StrSource<'s>;
    fn into_source(self) -> Self::Source {
        StrSource::new(self)
    }
}

impl<'s> IntoSource for &'s String {
    type Source = StrSource<'s>;
    fn into_source(self) -> Self::Source {
        StrSource::new(self as &str)
    }
}

pub struct StrSource<'s> {
    source: std::iter::Enumerate<std::str::CharIndices<'s>>,
    done: Processed,
}
impl<'s> StrSource<'s> {
    pub fn new(s: &str) -> StrSource {
        StrSource {
            source: s.char_indices().enumerate(),
            done: Processed::default(),
        }
    }
}
impl<'s> Source for StrSource<'s> {
    fn next_char(&mut self) -> SourceResult {
        Ok(self.source.next().map(|(char_index,(byte_index,c))| {
            let chars = Snip { offset: char_index, length: 1 };
            let bytes = Snip { offset: byte_index, length: c.len_utf8() };
            let r = c.sourcefy().localize(chars,bytes);
            self.done.chars += r.chars().length;
            self.done.bytes += r.bytes().length;
            r
        }))
    }
    fn processed(&self) -> Processed {
        self.done
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
    fn filter_char<F>(self, filter: F) -> Filtered<Self,F>
    where F: FnMut(char) -> Option<char>
    {
        Filtered {
            source: self,
            filter,
        }
    }
    fn map_char<M>(self, mapper: M) -> MapChar<Self,M> {
        MapChar {
            source: self,
            mapper,
        }
    }
    fn into_separator(self) -> IntoSeparator<Self> {
        IntoSeparator {
            source: self,
        }
    }
    fn merge_separators(self) -> MergeSeparator<Self> {
        MergeSeparator {
            source: self,
            buffer: None,
            current: None,
        }
    }
    /*fn shift(self, char_offset: usize, byte_offset: usize) -> Shift<Self> {
        Shift {
            source: self,
            char_offset,
            byte_offset,
        }
    }*/
    fn chain<S: Source>(self, chained: S) -> Chain<Self,S> {
        Chain {
            inner: InnerChain::First(self),
            second: Some(chained),           
        }
    }
    fn try_map<M>(self, mapper: M) -> Map<Self,M> {
        Map {
            source: self,
            mapper,
        }
    }
}

pub trait CharMapper {
    fn map(&mut self, c: char) -> char;
}

pub trait Mapper {
    fn map(&mut self, se: &SourceEvent) -> Option<SourceEvent>;
}

pub struct MapChar<S,M>
{
    source: S,
    mapper: M,
}
impl<S,M> Source for MapChar<S,M>
where S: Source,
      M: CharMapper
{
    fn next_char(&mut self) -> SourceResult {
        self.source.next_char().map(|ole| ole.map(|local_se| local_se.map(|se| match se {
            SourceEvent::Char(c) => SourceEvent::Char(self.mapper.map(c)),
            b @ SourceEvent::Breaker(_) => b,
        })))  
    }
    fn processed(&self) -> Processed {
        self.source.processed()
    }
}
        

pub struct Map<S,M>
{
    source: S,
    mapper: M,
}
impl<S,M> Source for Map<S,M>
where S: Source,
      M: Mapper
{
    fn next_char(&mut self) -> SourceResult {
        Ok(match self.source.next_char()? {
            Some(local_se) => {
                let (local,se) = local_se.into_inner();
                Some(match self.mapper.map(&se) {
                    Some(se) => local.local(se),
                    None => local.local(se),
                })
            },
            None => None,
        })
    }
    fn processed(&self) -> Processed {
        self.source.processed()
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
    fn processed(&self) -> Processed {
        self.source.processed()
    }
}

pub struct Filtered<S,F> {
    source: S,
    filter: F,
}
impl<S,F> Source for Filtered<S,F>
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
    fn processed(&self) -> Processed {
        self.source.processed()
    }
}

struct Shift<S> {
    source: S,
    char_offset: usize,
    byte_offset: usize,
}
impl<S> Shift<S> {
    fn new(source: S, shift: Processed) -> Shift<S> {
        Shift {
            source,
            char_offset: shift.chars,
            byte_offset: shift.bytes,
        }
    }
}
impl<S> Source for  Shift<S>
where S: Source
{
    fn next_char(&mut self) -> SourceResult {
        Ok(match self.source.next_char()? {
            Some(ev) => Some(ev.with_shift(self.char_offset,self.byte_offset)),
            None => None,
        })
    }
    fn processed(&self) -> Processed {
        let mut p = self.source.processed();
        p.chars += self.char_offset;
        p.bytes += self.byte_offset;
        p
    }
}

enum InnerChain<S1,S2> {
    First(S1),
    Second(Shift<S2>),
    Done(Processed)
}

pub struct Chain<S1,S2> {
    inner: InnerChain<S1,S2>,
    second: Option<S2>,
}
impl<S1,S2> Source for Chain<S1,S2>
where S1: Source,
      S2: Source
{
    fn next_char(&mut self) -> SourceResult {
        loop {
            match &mut self.inner {
                InnerChain::First(first) => match first.next_char()? {
                    Some(ev) => break Ok(Some(ev)),
                    None => match self.second.take() {
                        Some(second) => self.inner = InnerChain::Second(Shift::new(second,first.processed())),
                        None => self.inner = InnerChain::Done(first.processed()),
                    }
                },
                InnerChain::Second(second) => match second.next_char()? {
                    Some(ev) => break Ok(Some(ev)),
                    None => self.inner = InnerChain::Done(second.processed()),
                },
                InnerChain::Done(_) => break Ok(None),
            }
        }
    }
    fn processed(&self) -> Processed {
        match &self.inner {
            InnerChain::First(first) => first.processed(),
            InnerChain::Second(second) => second.processed(),
            InnerChain::Done(p) => *p,
        }
    }
}

use unicode_properties::{
    UnicodeGeneralCategory,
    GeneralCategory,
};

/*

   Mapping some chars into breakers:

      \n      => Breaker::Line
   
   Chars by unicode properties:
       
      Cc | Zs => Breaker::Word
      Zl      => Breaker::Line                   
      Zp      => Breaker::Paragraph

   Merging Breakers:
      
      1) Breaker::Line + Breaker::Line = Breaker::Paragraph
      2) Merge by "Inclusiveness": Sentence = sentence breaker + word breaker, etc.

*/

pub struct IntoSeparator<S> {
    source: S,
}
impl<S> Source for IntoSeparator<S>
where S: Source
{
    fn next_char(&mut self) -> SourceResult {
        self.source.next_char().map(|opt_lse| {
            opt_lse.map(|local_se| {
                local_se.map(|se| {
                    match se {
                        SourceEvent::Char(c) => {
                            match c {
                                '\n' => SourceEvent::Breaker(Breaker::Line),
                                _ => match c.general_category() {
                                    GeneralCategory::Control |
                                    GeneralCategory::SpaceSeparator => SourceEvent::Breaker(Breaker::Space),
                                    GeneralCategory::LineSeparator => SourceEvent::Breaker(Breaker::Line),                    
                                    GeneralCategory::ParagraphSeparator => SourceEvent::Breaker(Breaker::Paragraph),
                                    _ => SourceEvent::Char(c),
                                },
                            }
                        },
                        b @ SourceEvent::Breaker(..) => b,
                    }
                })
            })
        })
    }
    fn processed(&self) -> Processed {
        self.source.processed()
    }
}


pub struct MergeSeparator<S> {
    source: S,
    buffer: Option<Local<SourceEvent>>,
    current: Option<(Local<()>,Breaker)>,
}
impl<S> Source for MergeSeparator<S>
where S: Source
{
    fn next_char(&mut self) -> SourceResult {
        fn merge_breakers(cur_loc: Local<()>, cur_b: Breaker, nxt_loc: Local<()>, nxt_b: Breaker) -> Result<(Local<()>,Breaker),Error> {
            let loc = Local::from_segment(cur_loc,nxt_loc)?;
            Ok((loc,match (cur_b,nxt_b) {
                (Breaker::None,_) => nxt_b,
                (_,Breaker::None) => cur_b,
                (Breaker::Space,_) => nxt_b,
                (_,Breaker::Space) => cur_b,
                (Breaker::Word,_) => nxt_b,
                (_,Breaker::Word) => cur_b,
                (Breaker::Line,_) => nxt_b,
                (_,Breaker::Line) => cur_b,
                (Breaker::Sentence,_) => nxt_b,
                (_,Breaker::Sentence) => cur_b,                
                (Breaker::Paragraph,_) => nxt_b,
                (_,Breaker::Paragraph) => cur_b,
                (Breaker::Section,Breaker::Section) => nxt_b,
            }))
        }

        
        loop {
            match self.buffer.take() {
                Some(lse) => break Ok(Some(lse)),
                None => {
                    match self.source.next_char()? {
                        Some(local_se) => {                            
                            let (local,se) = local_se.into_inner();
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
    fn processed(&self) -> Processed {
        self.source.processed()
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
            .into_separator()
            .pipe(crate::paragraph::Builder::new().create())
            .merge_separators();

        let mut res_iter = [
            SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            SourceEvent::Char('⪢').localize(Snip { offset: 1, length: 16 },Snip { offset: 1, length: 16 }),
            SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 17, length: 8 },Snip { offset: 17, length: 8 }),
            SourceEvent::Char('💯').localize(Snip { offset: 25, length: 9 },Snip { offset: 25, length: 9 }),
            SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 34, length: 10 },Snip { offset: 34, length: 10 }),
            SourceEvent::Char('\u{200d}').localize(Snip { offset: 44, length: 8 },Snip { offset: 44, length: 8 }),
            SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 52, length: 19 },Snip { offset: 52, length: 19 }),
            SourceEvent::Char('❤').localize(Snip { offset: 71, length: 8 },Snip { offset: 71, length: 8 }),
            SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 79, length: 1 },Snip { offset: 79, length: 1 }),
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
            .into_separator()
            .pipe(crate::paragraph::Builder::new().create())
            .merge_separators();

        let mut res_iter = [
            SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            SourceEvent::Char('⪢').localize(Snip { offset: 1, length: 16 },Snip { offset: 1, length: 16 }),
            SourceEvent::Breaker(Breaker::Line).localize(Snip { offset: 17, length: 9 },Snip { offset: 17, length: 9 }),
            SourceEvent::Char('💯').localize(Snip { offset: 26, length: 9 },Snip { offset: 26, length: 9 }),
            SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 35, length: 10 },Snip { offset: 35, length: 10 }),
            SourceEvent::Char('\u{200d}').localize(Snip { offset: 45, length: 8 },Snip { offset: 45, length: 8 }),
            SourceEvent::Breaker(Breaker::Paragraph).localize(Snip { offset: 53, length: 23 },Snip { offset: 53, length: 23 }),
            SourceEvent::Char('❤').localize(Snip { offset: 76, length: 8 },Snip { offset: 76, length: 8 }),
            SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 84, length: 1 },Snip { offset: 84, length: 1 }),
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
    fn chain_1() {
        let src = " &GreaterGreater; &#x09;\n &#128175; &#xFEFF;";
        let mut src = src.into_source()
            .chain(Breaker::Word.into_source_as(" "))
            .chain("&#x200d; \n &#x200c; \n &#xf8e6; &#x2764; ".into_source())
            .pipe(crate::entities::Builder::new().create().into_piped())
            .filter_char(|c| {
                match c.general_category() {
                    GeneralCategory::Format if c != '\u{200d}' => None,
                    GeneralCategory::Unassigned => None,                    
                    _ if c == '\u{f8e6}' => None,
                    _ => Some(c),
                }
            })
            .into_separator()
            .pipe(crate::paragraph::Builder::new().create())
            .merge_separators();

        let mut res_iter = [
            SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            SourceEvent::Char('⪢').localize(Snip { offset: 1, length: 16 },Snip { offset: 1, length: 16 }),
            SourceEvent::Breaker(Breaker::Line).localize(Snip { offset: 17, length: 9 },Snip { offset: 17, length: 9 }),
            SourceEvent::Char('💯').localize(Snip { offset: 26, length: 9 },Snip { offset: 26, length: 9 }),
            SourceEvent::Breaker(Breaker::Word).localize(Snip { offset: 35, length: 10 },Snip { offset: 35, length: 10 }),
            SourceEvent::Char('\u{200d}').localize(Snip { offset: 45, length: 8 },Snip { offset: 45, length: 8 }),
            SourceEvent::Breaker(Breaker::Paragraph).localize(Snip { offset: 53, length: 23 },Snip { offset: 53, length: 23 }),
            SourceEvent::Char('❤').localize(Snip { offset: 76, length: 8 },Snip { offset: 76, length: 8 }),
            SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 84, length: 1 },Snip { offset: 84, length: 1 }),
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
