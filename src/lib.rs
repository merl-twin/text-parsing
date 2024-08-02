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


#[derive(Debug)]
pub enum Error {
    EofInTag,
    EndBeforeBegin,
    NoBegin,
}


