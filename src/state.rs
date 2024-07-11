use std::collections::VecDeque;

use crate::{
    Error,
    Local, ParserEvent,
    Parser, 
    ParserResult,
    Source, SourceEvent,
};

use opt_struct::OptVec;

pub trait StateMachine: Default {
    type Context;
    type Data;
    
    fn eof(self, context: &Self::Context) -> NextResult<Self,Self::Data>;
    fn next_state(self, local_char: Local<SourceEvent>, context: &Self::Context) -> NextResult<Self,Self::Data>;
}

#[derive(Debug)]
enum State<S> {
    SourceDone,
    SourceInvalid,
    Inner(S),
}

pub type NextResult<S, D> = Result<Next<S,D>,Error>;

pub struct Next<S: Default, D> {
    next_state: S,
    events: OptVec<Local<ParserEvent<D>>>,
}
impl<S: Default, D> Next<S,D> {
    pub fn empty() -> Next<S,D> {
        Next {
            next_state: S::default(),
            events: OptVec::None,
        }
    }
    pub fn with_state(mut self, st: S) -> Next<S,D> {
        self.next_state = st;
        self
    }
    pub fn with_event(mut self, ev: Local<ParserEvent<D>>) -> Next<S,D> {
        self.events.push(ev);
        self
    }
    pub fn push_event(&mut self, ev: Local<ParserEvent<D>>) {
        self.events.push(ev);
    }
}


pub struct Runtime<S,D,C> // State, Data, Context
where S: StateMachine<Data = D, Context = C>
{ 
    state: State<S>,
    buffer: VecDeque<Local<ParserEvent<D>>>,
    context: C,
}

impl<S,D,C> Runtime<S,D,C>
where S: StateMachine<Data = D, Context = C>
{
    pub fn new(context: C) -> Runtime<S,D,C> {
        Runtime {
            state: State::Inner(S::default()),
            buffer: VecDeque::new(),
            context,
        }
    }
    
    fn process_eof(&mut self, next: NextResult<S,D>) -> ParserResult<D> {
        let r = self.process(next)?;
        self.state = State::SourceDone;
        Ok(r)
    }
    fn process_err(&mut self, e: Error) -> ParserResult<D> {
        self.state = State::SourceInvalid;
        Err(e)
    }
    fn process(&mut self, next: NextResult<S,D>) -> ParserResult<D> {
        match next {
            Ok(Next { next_state, events }) => {
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

impl<S,D,C> Parser for Runtime<S,D,C>
where S: StateMachine<Data = D, Context = C>
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
