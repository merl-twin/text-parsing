use crate::{
    Local,
    Parser, SourceEvent, ParserEvent,
    SourceResult, Source,
};

pub trait PipeParser {
    fn next_char<S: Source>(&mut self, src: &mut S) -> SourceResult;
}

pub trait IntoPipeParser {
    type Piped: PipeParser;
    fn into_piped(self) -> Self::Piped;
}

impl<T: Parser> ParserExt for T {}

pub trait ParserExt: Parser + Sized {
    fn piped<I,F>(self, func: F) -> Piped<Self,I,F>
    where I: IntoIterator<Item = SourceEvent>,
          F: FnMut(<Self as Parser>::Data) -> I
    {
        Piped {
            parser: self,
            func,
            current_iter: None,
        }
    }
}

pub struct Piped<P,I,F>
where P: Parser,
      I: IntoIterator<Item = SourceEvent>,
      F: FnMut(<P as Parser>::Data) -> I
{
    parser: P,
    func: F,
    current_iter: Option<(Local<()>,<I as IntoIterator>::IntoIter)>, 
}
impl<P,I,F> PipeParser for Piped<P,I,F>
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
