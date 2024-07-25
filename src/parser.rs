use crate::{
    Local,
    SourceEvent,
    SourceResult, Source,
    Breaker, Error,
    source::ParserSource,
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
    fn partial_pipe_with<I,F>(self, func: F) -> PartialPipedWith<Self,I,F>
    where I: IntoIterator<Item = Local<SourceEvent>>,
          F: FnMut(<Self as Parser>::Data) -> Result<I,<Self as Parser>::Data>
    {
        PartialPipedWith {
            parser: self,
            func,
            current_iter: None,
        }
    }
    fn filter<F: Filter<Self::Data>>(self, filter: F) -> Filtered<Self,F> {
        Filtered {
            parser: self,
            filter,
        }
    }
    /*fn try_filter<F: TryFilter<Self::Data>>(self, filter: F) -> TryFiltered<Self,F> {
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
    }*/
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

/*pub trait TryFilter<D> {
    fn filter(&mut self, ev: ParserEvent<D>) -> Result<Option<ParserEvent<D>>,Error>;
}
pub trait IntoBreaker<D> {
    fn into_breaker(&mut self, data: &D) -> Option<Breaker>;
}
pub trait Flat {
    fn flatten(&mut self, ) -> 
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
}*/

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


pub struct PartialPipedWith<P,I,F>
where P: Parser,
      I: IntoIterator<Item = Local<SourceEvent>>,
      F: FnMut(<P as Parser>::Data) -> Result<I,<P as Parser>::Data>
{
    parser: P,
    func: F,
    current_iter: Option<<I as IntoIterator>::IntoIter>, 
}
impl<P,I,F> Parser for PartialPipedWith<P,I,F>
where P: Parser,
      I: IntoIterator<Item = Local<SourceEvent>>,
      F: FnMut(<P as Parser>::Data) -> Result<I,<P as Parser>::Data>
{
    type Data = <P as Parser>::Data;
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Self::Data> {
        if let Some(iter) = &mut self.current_iter {
            match iter.next() {
                Some(local_se) => return Ok(Some(local_se.map(|se|match se {
                    SourceEvent::Char(c) => ParserEvent::Char(c),
                    SourceEvent::Breaker(b) => ParserEvent::Breaker(b),
                }))),
                None => self.current_iter = None,
            }
        }
        while let Some(local_pe) = self.parser.next_event(src)? {
            let (local,pe) = local_pe.into_inner();
            match pe {
                p @ ParserEvent::Char(..) |
                p @ ParserEvent::Breaker(..) => return Ok(Some(local.local(p))),
                ParserEvent::Parsed(d) => match (&mut self.func)(d) {
                    Ok(into_iter) => {
                        let mut iter = into_iter.into_iter();
                        if let Some(local_se) = iter.next() {
                            self.current_iter = Some(iter);
                            return Ok(Some(local_se.map(|se| match se {
                                SourceEvent::Char(c) => ParserEvent::Char(c),
                                SourceEvent::Breaker(b) => ParserEvent::Breaker(b),
                            })));
                        }
                    },
                    Err(d) => return Ok(Some(local.local(ParserEvent::Parsed(d)))),
                },
            }
        }
        Ok(None)
    }
}


impl<T: PipeParser> PipeParserExt for T {}

pub trait PipeParserExt: PipeParser + Sized {
    fn pipe<P>(self, pipe: P) -> Pipe<Self,P>
    where P: PipeParser
    {
        Pipe {
            parser: self,
            pipe,
        }
    }
    fn option(self, use_it: bool) -> Option<Self> {
        match use_it {
            true => Some(self),
            false => None,
        }
    }

    fn as_source<'p,'s,S: Source>(&'p mut self, src: &'s mut S) -> ParserSource<'p,'s,Self,S> {
        ParserSource::new(self,src)
    }
}


pub struct Pipe<P1,P2> {
    parser: P1,
    pipe: P2,
}
impl<P1,P2> PipeParser for Pipe<P1,P2>
where P1: PipeParser,
      P2: PipeParser
{
    fn next_char<S: Source>(&mut self, src: &mut S) -> SourceResult {
        let mut src = self.parser.as_source(src);
        self.pipe.next_char(&mut src)       
    }
}




impl<P> Parser for Option<P>
where P: Parser
{
    type Data = <P as Parser>::Data;
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Self::Data> {
        match self {
            Some(parser) => parser.next_event(src),
            None => Ok(src.next_char()?.map(|local_se| local_se.map(|se| match se {
                SourceEvent::Char(c) => ParserEvent::Char(c),
                SourceEvent::Breaker(b) => ParserEvent::Breaker(b),
            }))),
        }
    }
}

impl<P> PipeParser for Option<P>
where P: PipeParser
{
    fn next_char<S: Source>(&mut self, src: &mut S) -> SourceResult {
        match self {
            Some(parser) => parser.next_char(src),
            None => src.next_char(),
        }
    }
}
