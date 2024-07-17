//mod aho_corasick;

mod locality;
pub use locality::{
    Snip, Local, Localize,
};

mod source;
pub use source::{
    Breaker,
    Source, IntoSource,
    StrSource,
    OptSource,
    EmptySource,
    SourceExt,
    SourceResult,
    SourceEvent,
};

mod parser;
pub use parser::{
    Parser,
    ParserExt,
    PipeParser,
    IntoPipeParser,
    ParserResult,
    ParserEvent,
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


