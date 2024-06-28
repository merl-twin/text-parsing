use opt_struct::OptVec;
use std::{
    collections::VecDeque,
};

//mod aho_corasick;

mod locality;
pub use locality::{
    Pos, Local, Localize,
};

mod source;
pub use source::{
    Source, IntoSource,
    StrSource,
    SourceExt,
};

mod parser;
pub use parser::{
    ParserExt,
    PipeParser,
    IntoPipeParser,
};

pub mod entities {
    mod entities;
    mod parser;
    mod state;

    pub use parser::{Builder,EntityParser};
}

pub mod tagger {
    mod tags;
    mod state;
    mod parser;

    pub use parser::{Builder,TagParser};
}


#[derive(Debug)]
pub enum Error {
    EofInTag,
    EndBeforeBegin,
}

#[derive(Debug)]
pub enum State<S> {
    SourceDone,
    SourceInvalid,
    Inner(S),
}

#[derive(Debug,Clone,Copy)]
// Inclusive: Sentence = sentence breaker + word breaker, etc.
pub enum Breaker {
    None,
    Word,
    Sentence,
    Paragraph,
    Section,
}

#[derive(Debug)]
pub enum ParserEvent<D> {
    Char(char),
    Breaker(Breaker),
    Parsed(D),
}


#[derive(Debug)]
pub enum SourceEvent {
    Char(char),
    Breaker(Breaker),
}
pub trait Sourcefy {
    fn sourcefy(self) -> SourceEvent;
}
impl Sourcefy for char {
    fn sourcefy(self) -> SourceEvent {
        SourceEvent::Char(self)
    }
}


pub type LocalEvent<D> = Local<ParserEvent<D>>;

pub type SourceResult =  Result<Option<Local<SourceEvent>>,Error>;
pub type ParserResult<D> =  Result<Option<LocalEvent<D>>,Error>;
pub type NextState<S, D> = Result<InnerState<S,D>,Error>;

pub struct InnerState<S: Default, D> {
    next_state: S,
    events: OptVec<LocalEvent<D>>,
}
impl<S: Default, D> InnerState<S,D> {
    pub fn empty() -> InnerState<S,D> {
        InnerState {
            next_state: S::default(),
            events: OptVec::None,
        }
    }
    pub fn with_state(mut self, st: S) -> InnerState<S,D> {
        self.next_state = st;
        self
    }
    pub fn with_event(mut self, ev: LocalEvent<D>) -> InnerState<S,D> {
        self.events.push(ev);
        self
    }
    pub fn push_event(&mut self, ev: LocalEvent<D>) {
        self.events.push(ev);
    }
}

pub trait ParserState: Default {
    type Context;
    type Data;
    
    fn eof(self, context: &Self::Context) -> NextState<Self,Self::Data>;
    fn next_state(self, local_char: Local<SourceEvent>, context: &Self::Context) -> NextState<Self,Self::Data>;
}

pub trait Parser {
    type Data;
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Self::Data>;
}

struct InnerParser<S,D,C> // State, Data, Context
where S: ParserState<Data = D, Context = C>
{ 
    state: State<S>,
    buffer: VecDeque<LocalEvent<D>>,
    context: C,
}

impl<S,D,C> InnerParser<S,D,C>
where S: ParserState<Data = D, Context = C>
{
    fn new(context: C) -> InnerParser<S,D,C> {
        InnerParser {
            state: State::Inner(S::default()),
            buffer: VecDeque::new(),
            context,
        }
    }
    
    fn process_eof(&mut self, next: NextState<S,D>) -> ParserResult<D> {
        let r = self.process(next)?;
        self.state = State::SourceDone;
        Ok(r)
    }
    fn process_err(&mut self, e: Error) -> ParserResult<D> {
        self.state = State::SourceInvalid;
        Err(e)
    }
    fn process(&mut self, next: NextState<S,D>) -> ParserResult<D> {
        match next {
            Ok(InnerState { next_state, events }) => {
                self.state = State::Inner(next_state);
                let mut iter = events.into_iter();
                Ok(match iter.next() {
                    None => None,
                    Some(ev) => {
                        self.buffer.extend(iter);
                        Some(ev)
                    }
                })
            },
            Err(e) => self.process_err(e),
        }
    }
}

impl<S,D,C> Parser for InnerParser<S,D,C>
where S: ParserState<Data = D, Context = C>
{
    type Data = D;
    
    fn next_event<SRC: Source>(&mut self, src: &mut SRC) -> ParserResult<D> {
        match self.buffer.pop_front() {
            Some(le) => Ok(Some(le)),
            None => loop {
                match &mut self.state {                    
                    State::SourceDone => break Ok(self.buffer.pop_front()),
                    State::SourceInvalid => break Ok(None),
                    State::Inner(ts) => {
                        let inner_state = std::mem::take(ts);                        
                        if let Some(local) = match src.next_char() {
                            Ok(None) => self.process_eof(inner_state.eof(&self.context)),                            
                            Ok(Some(local_char)) => self.process(inner_state.next_state(local_char,&self.context)),
                            Err(e) => self.process_err(e),
                        }? {
                            break Ok(Some(local))
                        }
                    },
                }
            }
        }
    }
}


