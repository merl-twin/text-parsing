use crate::{
    Local,
    SourceEvent,
    SourceResult, Source,
    Breaker, Error,
};

pub trait Parser {
    type Data;
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Self::Data>;
}

pub trait PipeParser {
    fn next_char<S: Source>(&mut self, src: &mut S) -> SourceResult;
}

pub trait IntoPipeParser {
    type Piped: PipeParser;
    fn into_piped(self) -> Self::Piped;
}

impl<T: Parser> ParserExt for T {}

pub trait ParserExt: Parser + Sized {
    fn pipe_with<I,F>(self, func: F) -> PipedWith<Self,I,F>
    where I: IntoIterator<Item = SourceEvent>,
          F: FnMut(<Self as Parser>::Data) -> I
    {
        PipedWith {
            parser: self,
            func,
            current_iter: None,
        }
    }
    /*fn partial_pipe<P>(self, parser: P) -> PartialPipe<Self,P>
    where P: PipeParser
    {
        PartialPipe {
            parser: self,
            pipe: parser,
        }
}*/
    fn filter<F: Filter<Self::Data>>(self, filter: F) -> Filtered<Self,F> {
        Filtered {
            parser: self,
            filter,
        }
    }
    fn try_filter<F: TryFilter<Self::Data>>(self, filter: F) -> TryFiltered<Self,F> {
        TryFiltered {
            parser: self,
            filter,
        }
    }

    
    fn try_into_breaker<B: IntoBreaker<Self::Data>>(self, into_breaker: B) -> TryIntoBreaker<Self,B> {
        TryIntoBreaker {
            parser: self,
            into_breaker,
        }
    }
    fn into_breaker(self) -> PipeBreaker<Self> {
        PipeBreaker {
            parser: self,
        }
    }
}

pub type ParserResult<D> =  Result<Option<Local<ParserEvent<D>>>,Error>;

#[derive(Debug,Eq,PartialEq)]
pub enum ParserEvent<D> {
    Char(char),
    Breaker(Breaker),
    Parsed(D),
}

pub trait Filter<D> {
    fn filter(&mut self, ev: ParserEvent<D>) -> Option<ParserEvent<D>>;
}
pub trait TryFilter<D> {
    fn filter(&mut self, ev: ParserEvent<D>) -> Result<Option<ParserEvent<D>>,Error>;
}
pub trait IntoBreaker<D> {
    fn into_breaker(&mut self, data: &D) -> Option<Breaker>;
}

pub struct TryIntoBreaker<P,B> {
    parser: P,
    into_breaker: B,
}
impl<P,B> Parser for TryIntoBreaker<P,B>
where P: Parser,
      B: IntoBreaker<<P as Parser>::Data>
{
    type Data = <P as Parser>::Data;
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Self::Data> {
        Ok(match self.parser.next_event(src)? {
            Some(local_pe) => {
                let (local,pe) = local_pe.into_inner();
                let pe = match pe {
                    p @ ParserEvent::Char(_) |
                    p @ ParserEvent::Breaker(_) => p,
                    ParserEvent::Parsed(d) => match self.into_breaker.into_breaker(&d) {
                        Some(b) => ParserEvent::Breaker(b),
                        None => ParserEvent::Parsed(d),
                    }
                };
                Some(local.local(pe))
            },
            None => None,
        })
    }
}

pub struct Filtered<P,F> {
    parser: P,
    filter: F,
}
impl<P,F> Parser for Filtered<P,F>
where P: Parser,
      F: Filter<<P as Parser>::Data>
{
    type Data = <P as Parser>::Data;
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Self::Data> {
        while let Some(local_pe) = self.parser.next_event(src)? {
            let (local,pe) = local_pe.into_inner();
            if let Some(pe) = self.filter.filter(pe) {
                return Ok(Some(local.local(pe)));
            }
        }
        Ok(None)
    }
}
pub struct TryFiltered<P,F> {
    parser: P,
    filter: F,
}
impl<P,F> Parser for TryFiltered<P,F>
where P: Parser,
      F: TryFilter<<P as Parser>::Data>
{
    type Data = <P as Parser>::Data;
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Self::Data> {
        while let Some(local_pe) = self.parser.next_event(src)? {
            let (local,pe) = local_pe.into_inner();
            if let Some(pe) = self.filter.filter(pe)? {
                return Ok(Some(local.local(pe)));
            }
        }
        Ok(None)
    }
}

pub struct PipeBreaker<P> {
    parser: P,
}
impl<P> PipeParser for PipeBreaker<P>
where P: Parser,
      P::Data: Into<Breaker>
{
    fn next_char<S: Source>(&mut self, src: &mut S) -> SourceResult {
        Ok(match self.parser.next_event(src)? {
            Some(local_pe) => {
                let (local,pe) = local_pe.into_inner();
                let se = match pe {
                    ParserEvent::Char(c) => SourceEvent::Char(c),
                    ParserEvent::Breaker(b) => SourceEvent::Breaker(b),
                    ParserEvent::Parsed(d) => SourceEvent::Breaker(d.into()),
                };
                Some(local.local(se))
            },
            None => None,
        }) 
    }
}


pub struct PipedWith<P,I,F>
where P: Parser,
      I: IntoIterator<Item = SourceEvent>,
      F: FnMut(<P as Parser>::Data) -> I
{
    parser: P,
    func: F,
    current_iter: Option<(Local<()>,<I as IntoIterator>::IntoIter)>, 
}
impl<P,I,F> PipeParser for PipedWith<P,I,F>
where P: Parser,
      I: IntoIterator<Item = SourceEvent>,
      F: FnMut(<P as Parser>::Data) -> I
{
    fn next_char<S: Source>(&mut self, src: &mut S) -> SourceResult {
        if let Some((local,iter)) = &mut self.current_iter {
            match iter.next() {
                Some(se) => return Ok(Some(local.local(se))),
                None => self.current_iter = None,
            }
        }
        while let Some(local_pe) = self.parser.next_event(src)? {
            let (local,pe) = local_pe.into_inner();
            match pe {
                ParserEvent::Char(c) => return Ok(Some(local.local(SourceEvent::Char(c)))),
                ParserEvent::Breaker(b) => return Ok(Some(local.local(SourceEvent::Breaker(b)))),
                ParserEvent::Parsed(d) => {
                    let mut iter = (&mut self.func)(d).into_iter();
                    if let Some(se) = iter.next() {
                        self.current_iter = Some((local,iter));
                        return Ok(Some(local.local(se)));
                    }
                },
            }
        }
        Ok(None)
    }
}

/*
pub struct PartialPipe<S,P> {
    parser: S,
    pipe: P,
}
impl<S,P> Parser for PartialPipe<S,P>
where S: Parser,
      P: PipeParser
{
    type Data = <S as Parser>::Data;
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Self::Data> {
        
    }
}

struct SourceFilter {
    
}
 */

