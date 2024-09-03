//mod aho_corasick;

mod locality;
pub use locality::{
    Snip, Local, Localize,
};

pub mod source;
pub use source::{
    Breaker,
    Source, IntoSource,
    StrSource,
    OptSource,
    Processed,
    EmptySource,
    ParserSource,
    SourceExt,
    SourceResult,
    SourceEvent,

    //Pipe, Filtered, IntoSeparator, Chain,
    //Shift,
};

pub mod parser;
pub use parser::{
    Parser,
    ParserExt,
    PipeParser,
    PipeParserExt,
    IntoPipeParser,
    ParserResult,
    ParserEvent,

    //Filter, Filtered, TryFilter, TryFiltered, IntoBreaker, TryIntoBreaker, PipeBreaker
    // Pipe
};

mod state;
pub use state::{
    NextResult, Next,
    StateMachine, Runtime,
};

pub mod entities {
    mod entities;
    mod parser;
    mod state;

    pub use parser::{Builder,EntityParser,PipedEntityParser};
}

pub mod tagger {
    mod tags;
    mod state;
    mod parser;

    pub use parser::{Builder,TagParser};
    pub use tags::{Tag,TagName,Closing,SpecTag};
}

pub mod paragraph {
    mod parser;
    mod state;

    pub use parser::{Builder,Paragraphs};
}


pub enum Error {
    EofInTag(Vec<Local<SourceEvent>>),
    EndBeforeBegin,
    NoBegin,
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::EndBeforeBegin => f.debug_struct("EndBeforeBegin"),
            Error::NoBegin => f.debug_struct("NoBegin"),
            Error::EofInTag(v) => {
                let mut dbg = f.debug_struct("EofInTag");
                let mut iter = v.into_iter();
                if let Some(lse) = iter.next() {
                    let (local,se) = lse.into_inner();
                    let first = local;
                    let mut last = local;
                    let mut s = String::new();
                    push_s(se,&mut s);
                    for lse in iter {
                        let (local,se) = lse.into_inner();
                        last = local;
                        push_s(se,&mut s);
                    }                    
                    if let Ok(lc) = Local::from_segment(first,last) {
                        dbg.field("chars", &lc.chars())
                            .field("bytes", &lc.bytes());
                    }
                    dbg.field("data", &s);                    
                }
                dbg
            }
        }.finish()
    }
}

fn push_s(se: SourceEvent, s: &mut String) {
    match se {
        SourceEvent::Char(c) => s.push(c),
        SourceEvent::Breaker(Breaker::None) => {},
        SourceEvent::Breaker(_) => s.push(' '),
    }
}
