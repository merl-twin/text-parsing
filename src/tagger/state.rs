use opt_struct::OptVec;

use super::{
    //entities::{Entity,ENTITIES},
    tags::{
        Tag, Closing, TagName, SpecTag,
    },
    parser::{     
        AttributeProperties, TaggerProperties, Unknown,
    },
};
use crate::{
    Error, Local, ParserEvent,
    NextState, InnerState, ParserState,
    SourceEvent, Breaker,
};


#[derive(Debug)]
pub(in super) enum TaggerState {
    Init,
    MayBeTag(Local<char>),
    SlashedTag {
        begin: Local<char>,
        current: Local<char>,
    },    
    TagName {
        begin: Local<char>,
        current: Local<char>,
        kind: Kind,
        name: String,
    },
    TagWaitAttrName(ReadTag),
    TagWaitAttrEq(ReadTag),
    TagWaitAttrValue(ReadTag),
    TagAttrName(ReadTag),
    TagAttrValue(ReadTag),
    TagAttrValueApos(ReadTag),
    TagAttrValueQuote(ReadTag),
    TagEnd(ReadTag),
}
impl Default for TaggerState {
    fn default() -> TaggerState {
        TaggerState::Init
    }
}

#[derive(Debug,Clone,Copy)]
pub(in super) enum Kind {
    Open,
    Close,
    Slash,
    Excl,
    Quest,
}

#[derive(Debug)]
pub(in super) struct ReadTag {
    begin: Local<char>,
    current: Local<char>,
    kind: Kind,
    name: TagName,

    tmp_buffer: Option<AttributeCollector>,
}
#[derive(Debug)]
struct AttributeCollector {
    need: OptVec<String>, // None means all, if no attrs neede there is no this struct (tmp_buffer = None)
    attributes: OptVec<(String,Option<String>)>,
    tmp_name: String,
    tmp_value: String,
}
impl AttributeCollector {
    fn new() -> AttributeCollector {
        AttributeCollector {
            need: OptVec::None,
            attributes: OptVec::None,
            tmp_name: String::new(),
            tmp_value: String::new(),
        }
    }
    fn do_need(&self, aname: &String) -> bool {
        for s in &self.need {
            if s == aname { return true; }
        }
        false
    }
}
fn tag_name_attrs(name: String, props: &TaggerProperties) -> (TagName, Option<AttributeCollector>) {
    let name = TagName::from(name);
    let attrs = match &props.attributes {
        AttributeProperties::All => Some(AttributeCollector::new()),
        AttributeProperties::None => None,
        AttributeProperties::Custom(v) => {
            let mut col = None;
            for (tag_name, attr_name) in v {
                if *tag_name == name {
                    match &mut col {
                        None => {
                            let mut c = AttributeCollector::new();
                            c.need.push(attr_name.clone());
                            col = Some(c);
                        },
                        Some(col) => col.need.push(attr_name.clone()),
                    }
                }
            }
            col
        },
    };
    (name,attrs)
}
impl ReadTag {
    fn attr_name_ascii_lowercase(&mut self, c: char) {
        if let Some(attr) = &mut self.tmp_buffer {
            attr.tmp_name.push(c.to_ascii_lowercase());
        }
    }
    fn attr_value_ascii_lowercase(&mut self, c: char) {
        if let Some(attr) = &mut self.tmp_buffer {
            attr.tmp_value.push(c.to_ascii_lowercase());
        }
    }
    fn attr_clear(&mut self) {
        if let Some(attr) = &mut self.tmp_buffer {
            attr.tmp_name.clear();
            attr.tmp_value.clear();
        }
    }
    fn attr_flush_no_value(&mut self) {
        if let Some(attr) = &mut self.tmp_buffer {
            if !attr.tmp_name.is_empty() {
                let name = std::mem::take(&mut attr.tmp_name);
                if attr.do_need(&name) {
                    attr.attributes.push((name,None));
                }
            }
            attr.tmp_value.clear();
        }        
    }
    fn attr_flush(&mut self) {
        if let Some(attr) = &mut self.tmp_buffer {
            if !attr.tmp_name.is_empty() {
                let name = std::mem::take(&mut attr.tmp_name);
                let value = std::mem::take(&mut attr.tmp_value);
                if attr.do_need(&name) {
                    attr.attributes.push((name,Some(value)));
                }
            } else {
                attr.tmp_value.clear();
            }
        }
    }
    fn attributes(&mut self) -> OptVec<(String,Option<String>)> {
        match &mut self.tmp_buffer {
            Some(attr) => {
                let mut tmp = OptVec::None;
                std::mem::swap(&mut attr.attributes,&mut tmp);
                tmp
            },
            None => OptVec::None,
        }
    }
}

fn crate_tag_event(mut tag: ReadTag) -> Result<Local<ParserEvent<Tag>>,Error> {
    let attrs = tag.attributes();
    let t = match tag.kind {
        Kind::Open => Tag::new(tag.name,Closing::Open,attrs),
        Kind::Close => Tag::new(tag.name,Closing::Close,attrs),
        Kind::Slash |
        Kind::Excl |
        Kind::Quest => Tag::new(tag.name,Closing::Void,attrs),
    };
    Local::from_segment(tag.begin,tag.current).map(|local| local.with_inner(ParserEvent::Parsed(t)))
}


impl ParserState for TaggerState {
    type Context = TaggerProperties;
    type Data = Tag;
    
    fn eof(self, props: &TaggerProperties) -> NextState<TaggerState,Tag> {
        // unexpected EOF in TAG
        //fn push_tag_eof(&mut self, begin: Local<char>, current: Local<char>, name: TagName, kind: Kind) -> Result<(),Error> {
        fn push_tag_eof(props: &TaggerProperties)-> Result<(),Error> {
            match props.eof_in_tag {
                Unknown::Error => Err(Error::EofInTag),
                Unknown::Skip => Ok(()),
                //Unknown::Text => ,
            }
        }
        
        Ok(match self {
            TaggerState::Init => InnerState::empty(),
            TaggerState::MayBeTag(tag_char) => InnerState::empty().with_event(tag_char.map(|c| ParserEvent::Char(c))),
            TaggerState::SlashedTag{..} |
            TaggerState::TagEnd(..) |
            TaggerState::TagWaitAttrName(..) |
            TaggerState::TagWaitAttrEq(..) |
            TaggerState::TagWaitAttrValue(..) |
            TaggerState::TagAttrName(..) |
            TaggerState::TagAttrValue(..) |
            TaggerState::TagAttrValueApos(..) |
            TaggerState::TagAttrValueQuote(..) |
            TaggerState::TagName{..} => {
                push_tag_eof(props)?;
                InnerState::empty()
            },
        })
    }
    fn next_state(self, local_src: Local<SourceEvent>, props: &TaggerProperties) -> NextState<TaggerState,Tag> {
        match self {
            TaggerState::Init => init(local_src),
            TaggerState::MayBeTag(tag_char) => may_be_tag(tag_char,local_src),
            TaggerState::SlashedTag{ begin, current } => slashed_tag(begin,current,local_src),
            TaggerState::TagName{ begin, current, kind, name } => tag_name(begin, current, local_src, kind, name, props),
            TaggerState::TagWaitAttrName(tag) => tag_wait_attr_name(tag, local_src),
            TaggerState::TagWaitAttrEq(tag) => tag_wait_attr_eq(tag, local_src),
            TaggerState::TagWaitAttrValue(tag) => tag_wait_attr_value(tag, local_src),
            TaggerState::TagAttrName(tag) => tag_attr_name(tag, local_src),
            TaggerState::TagAttrValue(tag) => tag_attr_value(tag, local_src),
            TaggerState::TagAttrValueApos(tag) => tag_attr_value_apos(tag, local_src),
            TaggerState::TagAttrValueQuote(tag) => tag_attr_value_quote(tag, local_src),
            TaggerState::TagEnd(tag) => tag_end(tag,local_src),
        }
    }
}

fn init(local_src: Local<SourceEvent>) -> NextState<TaggerState,Tag> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            match lc {
                '<' => InnerState::empty()
                    .with_state(TaggerState::MayBeTag(local_char)),
                _ => InnerState::empty()
                    .with_event(local_char.map(|c| ParserEvent::Char(c))),
            }
        },
        SourceEvent::Breaker(b) => match b {
            Breaker::None => InnerState::empty(),
            _ => InnerState::empty()
                .with_event(local_src.local(ParserEvent::Breaker(b))),
        },
    })
}


const TAB: char = '\u{09}';
const LF: char = '\u{0A}';
const FF: char = '\u{0C}';
const CR: char = '\u{0D}';


fn tag_attr_name(mut tag: ReadTag, local_src: Local<SourceEvent>) -> NextState<TaggerState,Tag> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            tag.current = local_char;
            match lc {
                '=' => InnerState::empty().with_state(TaggerState::TagWaitAttrValue(tag)),
                TAB | LF | FF | CR | ' ' => InnerState::empty().with_state(TaggerState::TagWaitAttrEq(tag)),
                '/' => {
                    tag.attr_flush_no_value();
                    InnerState::empty().with_state(TaggerState::TagWaitAttrName(tag))
                },
                '>' => {
                    tag.attr_flush_no_value();
                    InnerState::empty().with_event(crate_tag_event(tag)?)
                },
                c @ _ => {
                    tag.attr_name_ascii_lowercase(c);
                    InnerState::empty().with_state(TaggerState::TagAttrName(tag))
                },
            }
        },
        SourceEvent::Breaker(b) => match b {
            Breaker::None => InnerState::empty().with_state(TaggerState::TagAttrName(tag)),
            _ => InnerState::empty().with_state(TaggerState::TagWaitAttrEq(tag)),
        },
    })
}

fn tag_attr_value_apos(mut tag: ReadTag, local_src: Local<SourceEvent>) -> NextState<TaggerState,Tag> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            tag.current = local_char;
            match lc {
                '\'' => {
                    tag.attr_flush();
                    InnerState::empty().with_state(TaggerState::TagWaitAttrName(tag))
                },
                c @ _ => {
                    tag.attr_value_ascii_lowercase(c);
                    InnerState::empty().with_state(TaggerState::TagAttrValueApos(tag))
                },
            }
        },
        SourceEvent::Breaker(_) => InnerState::empty().with_state(TaggerState::TagAttrValueApos(tag)),
    })
}

fn tag_attr_value_quote(mut tag: ReadTag, local_src: Local<SourceEvent>) -> NextState<TaggerState,Tag> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            tag.current = local_char;
            match lc {
                '"' => {
                    tag.attr_flush();
                    InnerState::empty().with_state(TaggerState::TagWaitAttrName(tag))
                },
                c @ _ => {
                    tag.attr_value_ascii_lowercase(c);
                    InnerState::empty().with_state(TaggerState::TagAttrValueQuote(tag))
                },
            }
        },
        SourceEvent::Breaker(_) => InnerState::empty().with_state(TaggerState::TagAttrValueQuote(tag)),
    })
}

fn tag_attr_value(mut tag: ReadTag, local_src: Local<SourceEvent>) -> NextState<TaggerState,Tag> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            tag.current = local_char;
            match lc {
                TAB | LF | FF | CR | ' ' => {
                    tag.attr_flush();
                    InnerState::empty().with_state(TaggerState::TagWaitAttrName(tag))
                },
                '>' => {
                    tag.attr_flush();
                    InnerState::empty().with_event(crate_tag_event(tag)?)
                },
                c @ _ => {
                    tag.attr_value_ascii_lowercase(c);
                    InnerState::empty().with_state(TaggerState::TagAttrValue(tag))
                },
            }
        },
        SourceEvent::Breaker(b) => match b {
            Breaker::None => InnerState::empty().with_state(TaggerState::TagAttrValue(tag)),
            _ => {
                tag.attr_flush();
                InnerState::empty().with_state(TaggerState::TagWaitAttrName(tag))
            },
        },
    })
}

fn tag_wait_attr_value(mut tag: ReadTag, local_src: Local<SourceEvent>) -> NextState<TaggerState,Tag> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            tag.current = local_char;
            match lc {
                TAB | LF | FF | CR | ' ' => InnerState::empty().with_state(TaggerState::TagWaitAttrValue(tag)),
                '>' => {
                    tag.attr_flush();
                    InnerState::empty().with_event(crate_tag_event(tag)?)
                },
                '\'' => InnerState::empty().with_state(TaggerState::TagAttrValueApos(tag)),
                '"' => InnerState::empty().with_state(TaggerState::TagAttrValueQuote(tag)),
                c @ _ => {
                    tag.attr_value_ascii_lowercase(c);
                    InnerState::empty().with_state(TaggerState::TagAttrValue(tag))
                },
            }
        },
        SourceEvent::Breaker(_) => InnerState::empty().with_state(TaggerState::TagWaitAttrValue(tag)),
    })
}

fn tag_wait_attr_eq(mut tag: ReadTag, local_src: Local<SourceEvent>) -> NextState<TaggerState,Tag> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            tag.current = local_char;
            match lc {
                '=' => InnerState::empty().with_state(TaggerState::TagWaitAttrValue(tag)),
                TAB | LF | FF | CR | ' ' => InnerState::empty().with_state(TaggerState::TagWaitAttrEq(tag)),
                '/' => {
                    tag.attr_flush_no_value();
                    InnerState::empty().with_state(TaggerState::TagWaitAttrName(tag))
                },
                '>' => {
                    tag.attr_flush_no_value();
                    InnerState::empty().with_event(crate_tag_event(tag)?)
                },            
                c @ _ => {
                    tag.attr_flush_no_value();
                    tag.attr_name_ascii_lowercase(c);
                    InnerState::empty().with_state(TaggerState::TagAttrName(tag))
                },
            }
        },
        SourceEvent::Breaker(_) => InnerState::empty().with_state(TaggerState::TagWaitAttrEq(tag)),
    })
}

fn tag_wait_attr_name(mut tag: ReadTag, local_src: Local<SourceEvent>) -> NextState<TaggerState,Tag> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            tag.current = local_char;
            match lc {
                TAB | LF | FF | CR | ' ' | '/' => InnerState::empty().with_state(TaggerState::TagWaitAttrName(tag)),
                '>' => InnerState::empty().with_event(crate_tag_event(tag)?),
                c @ _ => {
                    tag.attr_clear();
                    tag.attr_name_ascii_lowercase(c);
                    InnerState::empty().with_state(TaggerState::TagAttrName(tag))
                },
            }
        },
        SourceEvent::Breaker(_) => InnerState::empty().with_state(TaggerState::TagWaitAttrName(tag)),
    })
}

fn tag_end(mut tag: ReadTag, local_src: Local<SourceEvent>) -> NextState<TaggerState,Tag> {    
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            tag.current = local_char;
            match lc {
                '>' => InnerState::empty().with_event(crate_tag_event(tag)?),
                _ => InnerState::empty().with_state(TaggerState::TagEnd(tag)),
            }
        },
        SourceEvent::Breaker(_) => InnerState::empty().with_state(TaggerState::TagEnd(tag)),
    })
}

fn tag_name(begin: Local<char>, current: Local<char>, local_src: Local<SourceEvent>, kind: Kind, mut name: String, props: &TaggerProperties) -> NextState<TaggerState,Tag> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            match lc {
                TAB | LF | FF | CR | ' ' => {
                    let (name,attrs) = tag_name_attrs(name,props);                    
                    InnerState::empty()
                        .with_state(TaggerState::TagWaitAttrName(ReadTag{ begin, kind, name, current: local_char, tmp_buffer: attrs }))
                },
                '>' => {
                    let tag = ReadTag{ begin, current: local_char, name: TagName::from(name), kind, tmp_buffer: None };
                    InnerState::empty().with_event(crate_tag_event(tag)?)
                },
                c @ _ => {           
                    for cc in c.to_lowercase() { name.push(cc); }
                    InnerState::empty()
                        .with_state(TaggerState::TagName {
                            begin, kind, name,
                            current: local_char,
                        })
                },
            }
        },
        SourceEvent::Breaker(b) => match b {
            Breaker::None => InnerState::empty()
                .with_state(TaggerState::TagName { begin, kind, name, current }),
            _ => {
                let (name,attrs) = tag_name_attrs(name,props);
                InnerState::empty()
                    .with_state(TaggerState::TagWaitAttrName(ReadTag{ begin, kind, name, current, tmp_buffer: attrs }))
            },
        },
    })
}


fn slashed_tag(begin: Local<char>, current: Local<char>, local_src: Local<SourceEvent>) -> NextState<TaggerState,Tag> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            match lc {
                '>' =>  {
                    let tag = ReadTag{ begin, current: local_char, name: TagName::x_from(SpecTag::Slash), kind: Kind::Slash, tmp_buffer: None };
                    InnerState::empty().with_event(crate_tag_event(tag)?)
                }
                c @ _ if c.is_ascii_alphabetic() => {
                    // TODO tag_name: add name coo info
                    InnerState::empty().with_state(TaggerState::TagName {
                        begin,
                        current: local_char,
                        kind: Kind::Close,
                        name: {
                            let mut s = String::new();
                            for cc in c.to_lowercase() { s.push(cc); }
                            s
                        },
                    })
                },
                _ => InnerState::empty().with_state(TaggerState::TagEnd(ReadTag {
                    begin,
                    current: local_char,
                    kind: Kind::Slash,
                    name: TagName::x_from(SpecTag::Slash),
                    tmp_buffer: None,
                })),
            }
        },
        SourceEvent::Breaker(_) => InnerState::empty().with_state(TaggerState::TagEnd(ReadTag {
            begin,
            current,
            kind: Kind::Slash,
            name: TagName::x_from(SpecTag::Slash),
            tmp_buffer: None,
        })),
    })
}

fn may_be_tag(tag_char: Local<char>, local_src: Local<SourceEvent>) -> NextState<TaggerState,Tag> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            match lc {
                '<' => {
                    InnerState::empty()
                        .with_state(TaggerState::MayBeTag(local_char))
                        .with_event(tag_char.map(|c| ParserEvent::Char(c)))
                },
                '/' => InnerState::empty().with_state(TaggerState::SlashedTag{ begin: tag_char, current: local_char }),
                '!' => InnerState::empty().with_state(TaggerState::TagEnd(ReadTag {
                    begin: tag_char,
                    current: local_char,
                    kind: Kind::Excl,
                    name: TagName::x_from(SpecTag::Excl),
                    tmp_buffer: None,
                })),
                '?' => InnerState::empty().with_state(TaggerState::TagEnd(ReadTag {
                    begin: tag_char,
                    current: local_char,
                    kind: Kind::Quest,
                    name: TagName::x_from(SpecTag::Quest),
                    tmp_buffer: None,
                })),
                c @ _ if c.is_ascii_alphabetic() => {
                    // TODO tag_name: add name coo info
                    InnerState::empty().with_state(TaggerState::TagName {
                        begin: tag_char,
                        current: local_char,
                        kind: Kind::Open,
                        name: {
                            let mut s = String::new();
                            for cc in c.to_lowercase() { s.push(cc); }
                            s
                        },
                    })
                },
                _ => InnerState::empty()
                    .with_event(tag_char.map(|c| ParserEvent::Char(c)))
                    .with_event(local_char.map(|c| ParserEvent::Char(c))),
            }
        },
        SourceEvent::Breaker(b) => match b {
            Breaker::None => InnerState::empty()
                .with_state(TaggerState::MayBeTag(tag_char)),
            _ => InnerState::empty()
                .with_event(tag_char.map(|c| ParserEvent::Char(c)))
                .with_event(local_src.local(ParserEvent::Breaker(b))),
        },
    })
}
