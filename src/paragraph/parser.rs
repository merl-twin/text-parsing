
use crate::{
    ParserResult,
    Source,
    Parser, Runtime, Breaker,
    SourceEvent,ParserEvent,
    PipeParser, SourceResult,
};

use super::{
    state::{ParaState,Paragraph},
};

/*

    Breaker::Line {Breaker::_}*X Breaker::Line = Breaker::Paragraph

*/


#[derive(Debug,Clone)]
pub struct Builder {

}
impl Builder {
    pub fn new() -> Builder {
        Builder{
          
        }
    }
    pub fn create(self) -> Paragraphs {
        Paragraphs(Runtime::new(()))
    }
}


pub struct Paragraphs(Runtime<ParaState,Paragraph,()>);

impl Parser for Paragraphs {
    type Data = Paragraph;
    
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Paragraph> {
        self.0.next_event(src)
    }
}

impl PipeParser for Paragraphs {
    fn next_char<S: Source>(&mut self, src: &mut S) -> SourceResult {
        Ok(match self.next_event(src)? {
            Some(local_pe) => {
                let (local,pe) = local_pe.into_inner();
                Some(local.local(match pe {
                    ParserEvent::Char(c) => SourceEvent::Char(c),
                    ParserEvent::Breaker(b) => SourceEvent::Breaker(b),
                    ParserEvent::Parsed(Paragraph) => SourceEvent::Breaker(Breaker::Paragraph),
                }))
            },
            None => None,
        })
    }
}



#[cfg(test)]
mod tests {
    use crate::*;
    use super::*;
    
    #[test]
    fn basic() {
        let mut src = "Hello, world!\n\nПривет, мир!".into_source();
        let mut parser = Builder::new().create();

        let mut res_iter = [
            ParserEvent::Char('H').localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 1, length: 1 },Snip { offset: 1, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 2, length: 1 },Snip { offset: 2, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char(',').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
            ParserEvent::Char('!').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
            ParserEvent::Parsed(Paragraph).localize(Snip { offset: 13, length: 2 },Snip { offset: 13, length: 2 }),
            ParserEvent::Char('П').localize(Snip { offset: 15, length: 1 },Snip { offset: 15, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 16, length: 1 },Snip { offset: 17, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 17, length: 1 },Snip { offset: 19, length: 2 }),
            ParserEvent::Char('в').localize(Snip { offset: 18, length: 1 },Snip { offset: 21, length: 2 }),
            ParserEvent::Char('е').localize(Snip { offset: 19, length: 1 },Snip { offset: 23, length: 2 }),
            ParserEvent::Char('т').localize(Snip { offset: 20, length: 1 },Snip { offset: 25, length: 2 }),
            ParserEvent::Char(',').localize(Snip { offset: 21, length: 1 },Snip { offset: 27, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 22, length: 1 },Snip { offset: 28, length: 1 }),
            ParserEvent::Char('м').localize(Snip { offset: 23, length: 1 },Snip { offset: 29, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 24, length: 1 },Snip { offset: 31, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 25, length: 1 },Snip { offset: 33, length: 2 }),
            ParserEvent::Char('!').localize(Snip { offset: 26, length: 1 },Snip { offset: 35, length: 1 }),
        ].into_iter();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
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
    fn basic_2() {
        let mut src = "Hello, world!  \n   \t  \n  Привет, мир!".into_source();
        let mut parser = Builder::new().create();

        let mut res_iter = [
            ParserEvent::Char('H').localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 1, length: 1 },Snip { offset: 1, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 2, length: 1 },Snip { offset: 2, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char(',').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
            ParserEvent::Char('!').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 13, length: 1 },Snip { offset: 13, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 14, length: 1 },Snip { offset: 14, length: 1 }),
            ParserEvent::Parsed(Paragraph).localize(Snip { offset: 15, length: 8 },Snip { offset: 15, length: 8 }),
            ParserEvent::Char(' ').localize(Snip { offset: 23, length: 1 },Snip { offset: 23, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 24, length: 1 },Snip { offset: 24, length: 1 }),
            ParserEvent::Char('П').localize(Snip { offset: 25, length: 1 },Snip { offset: 25, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 26, length: 1 },Snip { offset: 27, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 27, length: 1 },Snip { offset: 29, length: 2 }),
            ParserEvent::Char('в').localize(Snip { offset: 28, length: 1 },Snip { offset: 31, length: 2 }),
            ParserEvent::Char('е').localize(Snip { offset: 29, length: 1 },Snip { offset: 33, length: 2 }),
            ParserEvent::Char('т').localize(Snip { offset: 30, length: 1 },Snip { offset: 35, length: 2 }),
            ParserEvent::Char(',').localize(Snip { offset: 31, length: 1 },Snip { offset: 37, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 32, length: 1 },Snip { offset: 38, length: 1 }),
            ParserEvent::Char('м').localize(Snip { offset: 33, length: 1 },Snip { offset: 39, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 34, length: 1 },Snip { offset: 41, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 35, length: 1 },Snip { offset: 43, length: 2 }),
            ParserEvent::Char('!').localize(Snip { offset: 36, length: 1 },Snip { offset: 45, length: 1 }),
        ].into_iter();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
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
