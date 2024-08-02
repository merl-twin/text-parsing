use opt_struct::OptVec;
use unicode_properties::GeneralCategory;
use unicode_properties::UnicodeGeneralCategory;

use crate::{
    Error, Local, ParserEvent,
    NextResult, Next, StateMachine,
    SourceEvent, Breaker,
};

#[derive(Debug,PartialEq)]
pub struct Paragraph;

#[derive(Debug)]
pub(in super) enum ParaState {
    Init,
    First(OptVec<Local<ParserEvent<Paragraph>>>),
}
impl Default for ParaState {
    fn default() -> ParaState {
        ParaState::Init
    }
}

impl StateMachine for ParaState {
    type Context = ();
    type Data = Paragraph;
    
    fn eof(self, _props: &()) -> NextResult<ParaState,Paragraph> {
        // unexpected EOF
        Ok(match self {
            ParaState::Init => Next::empty(),
            ParaState::First(v) => {
                let mut next = Next::empty();
                for lpe in v {
                    next = next.with_event(lpe);
                }
                next
            },
        })
    }
    fn next_state(self, local_src: Local<SourceEvent>, _props: &()) -> NextResult<ParaState,Paragraph> {
        match self {
            ParaState::Init => init(local_src),
            ParaState::First(v) => first(v,local_src),
        }
    }
}

fn src_prs(local_src: Local<SourceEvent>) -> (SourceEvent,Local<ParserEvent<Paragraph>>) {
    let src = *local_src.data();
    let prs = local_src.local(match src {
        SourceEvent::Char(c) => ParserEvent::Char(c),
        SourceEvent::Breaker(b) => ParserEvent::Breaker(b),
    });
    (src,prs)
}


fn init(local_src: Local<SourceEvent>) -> NextResult<ParaState,Paragraph> {
    let (src,prs) = src_prs(local_src);
    Ok(match src {
        SourceEvent::Char(lc) => match lc {            
            '\n' => Next::empty()
                .with_state(ParaState::First(OptVec::One(prs))),
            c @ _ => match c.general_category() {
                GeneralCategory::LineSeparator => Next::empty()
                    .with_state(ParaState::First(OptVec::One(prs))),
                _ => Next::empty().with_event(prs),
            },
        },
        SourceEvent::Breaker(b) => match b {
            Breaker::Line => Next::empty()
                .with_state(ParaState::First(OptVec::One(prs))),
            _ => Next::empty().with_event(prs),
        },
    })
}

fn create_para(current: OptVec<Local<ParserEvent<Paragraph>>>, end: Local<ParserEvent<Paragraph>>) -> Result<Local<ParserEvent<Paragraph>>,Error> {
    match current.into_iter().next() {
        Some(begin) => Local::from_segment(begin,end).map(|lc| lc.local(ParserEvent::Parsed(Paragraph))),
        None => Err(Error::NoBegin),
    }
}

fn first(mut current: OptVec<Local<ParserEvent<Paragraph>>>, local_src: Local<SourceEvent>) -> NextResult<ParaState,Paragraph> {
    let (src,prs) = src_prs(local_src);
    Ok(match src {
        SourceEvent::Char(lc) => match lc {            
            '\n' => Next::empty().with_event(create_para(current,prs)?),          
            c @ _ => match c.general_category() {
                GeneralCategory::LineSeparator => Next::empty().with_event(create_para(current,prs)?),
                GeneralCategory::Control |
                GeneralCategory::SpaceSeparator => {
                    current.push(prs);
                    Next::empty()
                        .with_state(ParaState::First(current))
                },
                _ => {
                    current.push(prs);
                    let mut next = Next::empty()
                        .with_state(ParaState::Init);
                    for lpe in current {
                        next = next.with_event(lpe);
                    }
                    next
                },
            },
        },
        SourceEvent::Breaker(b) => match b {
            Breaker::None |
            Breaker::Space |
            Breaker::Word |
            Breaker::Sentence => {
                current.push(prs);
                Next::empty()
                    .with_state(ParaState::First(current))
            },
            Breaker::Line => Next::empty().with_event(create_para(current,prs)?),
            Breaker::Paragraph |
            Breaker::Section => {
                current.push(prs);
                let mut next = Next::empty()
                    .with_state(ParaState::Init);
                for lpe in current {
                    next = next.with_event(lpe);
                }
                next
            },
        },
    })
}
