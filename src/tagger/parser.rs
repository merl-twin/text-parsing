use super::{
    tags::{
        Tag, TagName,
    },
    state::{
        TaggerState,
    }
};
use crate::{
    Error,
    SourceEvent,
    ParserEvent,
    Local,
    ParserResult,
    Source,
    Parser, Runtime,
};

/*

  Algorithm: https://dev.w3.org/html5/spec-LC/parsing.html

*/


#[derive(Debug,Clone)]
pub struct Builder {
    auto_detect: bool,
    eof_in_tag: Unknown,
    
    properties: TaggerProperties,
}
impl Builder {
    pub fn new() -> Builder {
        Builder{
            auto_detect: false,
            eof_in_tag: Unknown::Error,
            properties: TaggerProperties::default(),
        }
    }
    pub fn auto_detect() -> Builder {
        Builder{
            auto_detect: true,
            eof_in_tag: Unknown::Error,
            properties: TaggerProperties::default(),
        }
    }
    pub fn with_all_attributes(mut self) -> Builder {
        self.properties.attributes = AttributeProperties::All;
        self
    }
    pub fn with_attribute<S: ToString>(mut self, name: TagName, attr: S) -> Builder {
        match &mut self.properties.attributes {
            AttributeProperties::None |
            AttributeProperties::All => self.properties.attributes = AttributeProperties::Custom(vec![(name,attr.to_string())]),
            AttributeProperties::Custom(v) => v.push((name,attr.to_string())),
        };
        self
    }
    pub fn skip_eof_in_tag(mut self) -> Builder {
        self.eof_in_tag = Unknown::Skip;
        self
    }
    pub fn text_eof_in_tag(mut self) -> Builder {
        self.eof_in_tag = Unknown::Text;
        self
    }
    pub fn create(self) -> TagParser {
        match self.auto_detect {
            false => TagParser(InnerTagParser::Xhtml(XhtmlParser {
                done: false,
                eof_in_tag: self.eof_in_tag,
                sbuffer: None,
                pbuffer: None,
                runtime: Runtime::new(self.properties),
                final_error: None,
            })),
            true => TagParser(InnerTagParser::Detector(Detector{
                eof_in_tag: self.eof_in_tag,
                runtime: Runtime::new(self.properties),
            })),
        }
    }
}

#[derive(Debug,Clone,Copy)]
enum Unknown {
    Error,
    Skip,
    Text
}

#[derive(Debug,Clone)]
pub(in super) enum AttributeProperties {
    None,
    Custom(Vec<(TagName,String)>),
    All,
}

#[derive(Debug,Clone)]
pub(in super) struct TaggerProperties {
    pub attributes: AttributeProperties,
}
impl Default for TaggerProperties {
    fn default() -> TaggerProperties {
        TaggerProperties {
            attributes: AttributeProperties::None,
        }
    }
}

pub struct TagParser(InnerTagParser);
impl Parser for TagParser {
    type Data = Tag;
    
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Tag> {
        self.0.next_event(src)
    }
}

enum InnerTagParser {
    None,
    Detector(Detector),
    Xhtml(XhtmlParser),
    Plain(PlainParser),
}
impl Parser for InnerTagParser {
    type Data = Tag;
    
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Tag> {
        match self {            
            InnerTagParser::Detector(..) => {
                let mut tmp = InnerTagParser::None;
                std::mem::swap(&mut tmp, self);
                let detector = match tmp {
                    InnerTagParser::Detector(d) => d,
                    _ => unreachable!(),
                };
                
                let mut r = None;
                *self = match detector.try_next(src) {
                    DetectorResult::Next(slf,res) => {
                        r = Some(res);
                        InnerTagParser::Detector(slf)
                    },
                    DetectorResult::Xhtml(xhtml) => InnerTagParser::Xhtml(xhtml),
                    DetectorResult::Plain(plain) => InnerTagParser::Plain(plain),
                };
                match r {
                    Some(r) => r,
                    None => self.next_event(src),
                }
            },
            InnerTagParser::Xhtml(parser) => parser.next_event(src),
            InnerTagParser::Plain(parser) => parser.next_event(src),
            InnerTagParser::None => Ok(None),
        }
    }
}

#[derive(Default)]
struct Counter {
    common: usize,
    named: usize,
    service: usize,
    unknown: usize,
}
impl Counter {
    fn push(&mut self, tag: &Tag) {
        if tag.name.is_common() { self.common += 1; } else {
            if tag.name.is_named() { self.named += 1; } else {
                if tag.name.is_service() { self.service += 1; }
                else { self.unknown += 1; }
            }
        }
    }
    fn tags(&self) -> usize {
        self.common + self.named + self.service
    }
    fn check(&self) -> bool {
        ( self.common >= 2 ) || ( self.named >= 5 )
    }
}

struct Detector {
    eof_in_tag: Unknown,
    runtime: Runtime<TaggerState,Tag,TaggerProperties>,
}
enum DetectorResult {
    Next(Detector,ParserResult<Tag>),
    Xhtml(XhtmlParser),
    Plain(PlainParser),
}
impl Detector {
    fn try_next<S: Source>(mut self, src: &mut S) -> DetectorResult {
        match self.runtime.next_event(src) {
            Ok(Some(lpe)) => match lpe.data() {
                ParserEvent::Parsed(tag) => {
                    let mut counter = Counter::default();
                    counter.push(tag);                    
                    let mut pbuffer = vec![lpe];
                    let mut eof_buffer = None;
                    let mut eof_error = None;
                    let done = loop {
                        match self.runtime.next_event(src) {
                            Ok(Some(lpe)) => match lpe.data() {
                                ParserEvent::Parsed(tag) => {
                                    counter.push(tag);
                                    pbuffer.push(lpe);
                                },
                                _ => pbuffer.push(lpe),
                            },
                            Ok(None) => break true,
                            Err(Error::EofInTag(raw)) => {
                                eof_buffer = Some(raw);
                                break true;
                            },
                            Err(e) => {
                                eof_error = Some(e);
                                break true;
                            }, 
                        }

                        // check detector
                        if counter.check() { break false; }                        
                    };
                    // make decision, return parser
                    match counter.tags() > 0 {
                        true => {
                            match (self.eof_in_tag, eof_buffer) {
                                (_,None) |
                                (Unknown::Skip,_) => {},
                                (Unknown::Error,Some(raw)) => eof_error = Some(Error::EofInTag(raw)),
                                (Unknown::Text,Some(raw)) => {
                                    for lse in raw {
                                        pbuffer.push(lse.map(|se| se.into()));
                                    }
                                }
                            }
                            DetectorResult::Xhtml(XhtmlParser {
                                done,
                                eof_in_tag: self.eof_in_tag,
                                sbuffer: None,
                                pbuffer: Some(pbuffer.into_iter()),
                                runtime: self.runtime,
                                final_error: eof_error,
                            })
                        },
                        false => {
                            if let Some(raw) = eof_buffer {
                                for lse in raw {
                                    pbuffer.push(lse.map(|se| se.into()));
                                }
                            }
                            DetectorResult::Plain(PlainParser {
                                done,
                                sbuffer: None,
                                pbuffer: Some(pbuffer.into_iter()),
                                runtime: self.runtime,
                                final_error: eof_error,
                            })
                        },
                    }  
                },
                _ => DetectorResult::Next(self,Ok(Some(lpe))),
            },
            Ok(None) => DetectorResult::Next(self,Ok(None)),                
            Err(Error::EofInTag(raw)) => DetectorResult::Plain(PlainParser { // means no tags were found before
                done: true,
                sbuffer: Some(raw.into_iter()),
                pbuffer: None,
                runtime: self.runtime,
                final_error: None
            }),
            Err(e) => DetectorResult::Next(self,Err(e)),             
        }
    }
}


struct PlainParser {
    done: bool,
    sbuffer: Option<std::vec::IntoIter<Local<SourceEvent>>>,
    pbuffer: Option<std::vec::IntoIter<Local<ParserEvent<Tag>>>>,
    runtime: Runtime<TaggerState,Tag,TaggerProperties>,
    final_error: Option<Error>,
}
impl PlainParser {
    fn final_err(&mut self) -> ParserResult<Tag> {
        match self.final_error.take() {
            None => Ok(None),
            Some(e) => Err(e),
        }
    }
}
impl Parser for PlainParser {
    type Data = Tag;
    
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Tag> {
        // check sbuffer
        if let Some(sbuf) = &mut self.sbuffer {
            match sbuf.next() {
                Some(lse) => return Ok(Some(lse.map(|se| se.into()))),
                None => self.sbuffer = None,
            }
        }

        // check pbuffer
        if let Some(pbuf) = &mut self.pbuffer {
            match pbuf.next() {
                Some(lpe) => return match lpe.data() {
                    ParserEvent::Parsed(..) => {
                        let (_,pe) = lpe.into_inner();
                        let tag = match pe {
                            ParserEvent::Parsed(tag) => tag,
                            _ => unreachable!(),
                        };
                        let mut iter = tag.raw.into_iter();
                        match iter.next() {
                            Some(lse) => {
                                self.sbuffer = Some(iter);
                                Ok(Some(lse.map(|se| se.into())))
                            },
                            None => self.final_err(),
                        } 
                    },
                    _ => Ok(Some(lpe)),
                },
                None => self.pbuffer = None,
            }
        } 

        match self.done {
            false => match self.runtime.next_event(src) {
                Ok(Some(lpe)) => match lpe.data() {
                    ParserEvent::Parsed(..) => {
                        let (_,pe) = lpe.into_inner();
                        let tag = match pe {
                            ParserEvent::Parsed(tag) => tag,
                            _ => unreachable!(),
                        };
                        let mut iter = tag.raw.into_iter();
                        match iter.next() {
                            Some(lse) => {
                                self.sbuffer = Some(iter);
                                Ok(Some(lse.map(|se| se.into())))
                            },
                            None => self.final_err(),
                        } 
                    },
                    _ => Ok(Some(lpe)),
                },
                Ok(None) => self.final_err(),
                Err(Error::EofInTag(raw)) => {
                    self.done = true;
                    let mut iter = raw.into_iter();
                    match iter.next() {
                        Some(lse) => {
                            self.sbuffer = Some(iter);
                            Ok(Some(lse.map(|se| se.into())))
                        },
                        None => self.final_err(),
                    }
                },
                Err(e) => Err(e),             
            },
            true => self.final_err(),
        }
    }
}


struct XhtmlParser {
    done: bool,
    eof_in_tag: Unknown,
    sbuffer: Option<std::vec::IntoIter<Local<SourceEvent>>>,
    pbuffer: Option<std::vec::IntoIter<Local<ParserEvent<Tag>>>>,
    runtime: Runtime<TaggerState,Tag,TaggerProperties>,
    final_error: Option<Error>,
}
impl XhtmlParser {
    fn final_err(&mut self) -> ParserResult<Tag> {
        match self.final_error.take() {
            None => Ok(None),
            Some(e) => Err(e),
        }
    }
}
impl Parser for XhtmlParser {
    type Data = Tag;
    
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Tag> {
        // check sbuffer
        if let Some(sbuf) = &mut self.sbuffer {
            match sbuf.next() {
                Some(lse) => return Ok(Some(lse.map(|se| se.into()))),
                None => self.sbuffer = None,
            }
        }

        // check pbuffer
        if let Some(pbuf) = &mut self.pbuffer {
            match pbuf.next() {
                Some(lpe) => return Ok(Some(lpe)),
                None => self.pbuffer = None,
            }
        } 

        match self.done {
            false => loop {
                match self.runtime.next_event(src) {
                    Ok(Some(lpe)) => break Ok(Some(lpe)),
                    Ok(None) => break self.final_err(),
                    Err(Error::EofInTag(raw)) => {
                        self.done = true;
                        match self.eof_in_tag {
                            Unknown::Error => break Err(Error::EofInTag(raw)),
                            Unknown::Skip => {},
                            Unknown::Text => {
                                let mut iter = raw.into_iter();
                                if let Some(lse) = iter.next() {                                                
                                    self.sbuffer = Some(iter);
                                    break Ok(Some(lse.map(|se| se.into())));
                                }
                            },
                        }
                    },
                    Err(e) => break Err(e),             
                }
            },
            true => self.final_err(),
        }
    }
}


/*impl PipeParser for TagParser {
    fn next_char<S: Source>(&mut self, src: &mut S) -> SourceResult {
        Ok(match self.next_event(src)? {
            Some(local_pe) => {
                let (local,pe) = local_pe.into_inner();
                Some(local.local(match pe {
                    ParserEvent::Char(c) => SourceEvent::Char(c),
                    ParserEvent::Breaker(b) => SourceEvent::Breaker(b),
                    ParserEvent::Parsed(tag) => SourceEvent::Breaker(tag.breaker),
                }))
            },
            None => None,
        })
    }
}*/



#[cfg(test)]
mod tests {
    use crate::*;
    use super::*;

    use crate::tagger::tags::*;
    use opt_struct::OptVec;
    
    #[test]
    fn basic() {
        let mut src = "<h1>Hello, world!</h1>Привет, мир!".into_source();
        let mut parser = Builder::new().create();

        let mut res_iter = [
            ParserEvent::Parsed(Tag {
                name: TagName::H1, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
                end: ().localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 1, length: 1 },Snip { offset: 1, length: 1 }),
                    SourceEvent::Char('1').localize(Snip { offset: 2, length: 1 },Snip { offset: 2, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
                ],
            }).localize(Snip { offset: 0, length: 4 },Snip { offset: 0, length: 4 }),
            ParserEvent::Char('H').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
            ParserEvent::Char(',').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 13, length: 1 },Snip { offset: 13, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 14, length: 1 },Snip { offset: 14, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 15, length: 1 },Snip { offset: 15, length: 1 }),
            ParserEvent::Char('!').localize(Snip { offset: 16, length: 1 },Snip { offset: 16, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::H1, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
                end: ().localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 18, length: 1 },Snip { offset: 18, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 19, length: 1 },Snip { offset: 19, length: 1 }),
                    SourceEvent::Char('1').localize(Snip { offset: 20, length: 1 },Snip { offset: 20, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
                ],
            }).localize(Snip { offset: 17, length: 5 },Snip { offset: 17, length: 5 }),
            ParserEvent::Char('П').localize(Snip { offset: 22, length: 1 },Snip { offset: 22, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 23, length: 1 },Snip { offset: 24, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 24, length: 1 },Snip { offset: 26, length: 2 }),
            ParserEvent::Char('в').localize(Snip { offset: 25, length: 1 },Snip { offset: 28, length: 2 }),
            ParserEvent::Char('е').localize(Snip { offset: 26, length: 1 },Snip { offset: 30, length: 2 }),
            ParserEvent::Char('т').localize(Snip { offset: 27, length: 1 },Snip { offset: 32, length: 2 }),
            ParserEvent::Char(',').localize(Snip { offset: 28, length: 1 },Snip { offset: 34, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 29, length: 1 },Snip { offset: 35, length: 1 }),
            ParserEvent::Char('м').localize(Snip { offset: 30, length: 1 },Snip { offset: 36, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 31, length: 1 },Snip { offset: 38, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 32, length: 1 },Snip { offset: 40, length: 2 }),
            ParserEvent::Char('!').localize(Snip { offset: 33, length: 1 },Snip { offset: 42, length: 1 }),
        ].into_iter();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {            
            /*if let ParserEvent::Parsed(tag) = local_event.data() {
                for lse in &tag.raw {
                    let (l,e) = lse.into_inner();
                    println!("SourceEvent::{:?}.localize({:?},{:?}),",e,l.chars(),l.bytes());
                }
                println!("");
            }*/
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
                    println!("Result: {:?}",ev);
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

    #[test]
    fn basic_void() {
        let mut src = "<h1>Hello, world!</h1>Привет, <tag />мир!".into_source();
        let mut parser = Builder::new().create();

        let mut res_iter = [
            ParserEvent::Parsed(Tag {
                name: TagName::H1, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
                end: ().localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 1, length: 1 },Snip { offset: 1, length: 1 }),
                    SourceEvent::Char('1').localize(Snip { offset: 2, length: 1 },Snip { offset: 2, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
                ],
            }).localize(Snip { offset: 0, length: 4 },Snip { offset: 0, length: 4 }),
            ParserEvent::Char('H').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
            ParserEvent::Char(',').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 13, length: 1 },Snip { offset: 13, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 14, length: 1 },Snip { offset: 14, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 15, length: 1 },Snip { offset: 15, length: 1 }),
            ParserEvent::Char('!').localize(Snip { offset: 16, length: 1 },Snip { offset: 16, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::H1, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
                end: ().localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 18, length: 1 },Snip { offset: 18, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 19, length: 1 },Snip { offset: 19, length: 1 }),
                    SourceEvent::Char('1').localize(Snip { offset: 20, length: 1 },Snip { offset: 20, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
                ],
            }).localize(Snip { offset: 17, length: 5 },Snip { offset: 17, length: 5 }),
            ParserEvent::Char('П').localize(Snip { offset: 22, length: 1 },Snip { offset: 22, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 23, length: 1 },Snip { offset: 24, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 24, length: 1 },Snip { offset: 26, length: 2 }),
            ParserEvent::Char('в').localize(Snip { offset: 25, length: 1 },Snip { offset: 28, length: 2 }),
            ParserEvent::Char('е').localize(Snip { offset: 26, length: 1 },Snip { offset: 30, length: 2 }),
            ParserEvent::Char('т').localize(Snip { offset: 27, length: 1 },Snip { offset: 32, length: 2 }),
            ParserEvent::Char(',').localize(Snip { offset: 28, length: 1 },Snip { offset: 34, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 29, length: 1 },Snip { offset: 35, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Other("tag".to_string()), closing: Closing::Void, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 30, length: 1 },Snip { offset: 36, length: 1 }),
                end: ().localize(Snip { offset: 36, length: 1 },Snip { offset: 42, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 30, length: 1 },Snip { offset: 36, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 31, length: 1 },Snip { offset: 37, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 32, length: 1 },Snip { offset: 38, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 33, length: 1 },Snip { offset: 39, length: 1 }),
                    SourceEvent::Char(' ').localize(Snip { offset: 34, length: 1 },Snip { offset: 40, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 35, length: 1 },Snip { offset: 41, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 36, length: 1 },Snip { offset: 42, length: 1 }),
                ],
            }).localize(Snip { offset: 30, length: 7 },Snip { offset: 36, length: 7 }),
            ParserEvent::Char('м').localize(Snip { offset: 37, length: 1 },Snip { offset: 43, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 38, length: 1 },Snip { offset: 45, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 39, length: 1 },Snip { offset: 47, length: 2 }),
            ParserEvent::Char('!').localize(Snip { offset: 40, length: 1 },Snip { offset: 49, length: 1 }),
        ].into_iter();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            /*if let ParserEvent::Parsed(tag) = local_event.data() {
                for lse in &tag.raw {
                    let (l,e) = lse.into_inner();
                    println!("SourceEvent::{:?}.localize({:?},{:?}),",e,l.chars(),l.bytes());
                }
                println!("");
            }*/
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
                    println!("Result: {:?}",ev);
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

    #[test]
    fn basic_void_2() {
        let mut src = "<h1>Hello, world!</h1>Привет, <tags/>мир!".into_source();
        let mut parser = Builder::new().create();

        let mut res_iter = [
            ParserEvent::Parsed(Tag {
                name: TagName::H1, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
                end: ().localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 1, length: 1 },Snip { offset: 1, length: 1 }),
                    SourceEvent::Char('1').localize(Snip { offset: 2, length: 1 },Snip { offset: 2, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
                ],
            }).localize(Snip { offset: 0, length: 4 },Snip { offset: 0, length: 4 }),
            ParserEvent::Char('H').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
            ParserEvent::Char(',').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 13, length: 1 },Snip { offset: 13, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 14, length: 1 },Snip { offset: 14, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 15, length: 1 },Snip { offset: 15, length: 1 }),
            ParserEvent::Char('!').localize(Snip { offset: 16, length: 1 },Snip { offset: 16, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::H1, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
                end: ().localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 18, length: 1 },Snip { offset: 18, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 19, length: 1 },Snip { offset: 19, length: 1 }),
                    SourceEvent::Char('1').localize(Snip { offset: 20, length: 1 },Snip { offset: 20, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
                ],
            }).localize(Snip { offset: 17, length: 5 },Snip { offset: 17, length: 5 }),
            ParserEvent::Char('П').localize(Snip { offset: 22, length: 1 },Snip { offset: 22, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 23, length: 1 },Snip { offset: 24, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 24, length: 1 },Snip { offset: 26, length: 2 }),
            ParserEvent::Char('в').localize(Snip { offset: 25, length: 1 },Snip { offset: 28, length: 2 }),
            ParserEvent::Char('е').localize(Snip { offset: 26, length: 1 },Snip { offset: 30, length: 2 }),
            ParserEvent::Char('т').localize(Snip { offset: 27, length: 1 },Snip { offset: 32, length: 2 }),
            ParserEvent::Char(',').localize(Snip { offset: 28, length: 1 },Snip { offset: 34, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 29, length: 1 },Snip { offset: 35, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Other("tags".to_string()), closing: Closing::Void, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 30, length: 1 },Snip { offset: 36, length: 1 }),
                end: ().localize(Snip { offset: 36, length: 1 },Snip { offset: 42, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 30, length: 1 },Snip { offset: 36, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 31, length: 1 },Snip { offset: 37, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 32, length: 1 },Snip { offset: 38, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 33, length: 1 },Snip { offset: 39, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 34, length: 1 },Snip { offset: 40, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 35, length: 1 },Snip { offset: 41, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 36, length: 1 },Snip { offset: 42, length: 1 }),
                ],
            }).localize(Snip { offset: 30, length: 7 },Snip { offset: 36, length: 7 }),
            ParserEvent::Char('м').localize(Snip { offset: 37, length: 1 },Snip { offset: 43, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 38, length: 1 },Snip { offset: 45, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 39, length: 1 },Snip { offset: 47, length: 2 }),
            ParserEvent::Char('!').localize(Snip { offset: 40, length: 1 },Snip { offset: 49, length: 1 }),
        ].into_iter();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            /*if let ParserEvent::Parsed(tag) = local_event.data() {
                for lse in &tag.raw {
                    let (l,e) = lse.into_inner();
                    println!("SourceEvent::{:?}.localize({:?},{:?}),",e,l.chars(),l.bytes());
                }
                println!("");
            }*/
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
                    println!("Result: {:?}",ev);
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

    
    #[test]
    fn a_img() {        
        let mut src = "
<p>In the common case, <a href=\"apis-in-html-documents.html#dynamic-markup-insertion\" title=\"dynamic markup
  insertion\">, e.g. using the <code title=\"dom-document-write\"><a href=\"apis-in-html-documents.html#dom-document-write\">document.write()</a></code> API.</p>
  <p><img alt=\"\" height=\"554\" src=\"https://dev.w3.org/html5/spec/images/parsing-model-overview.png\" width=\"427\"></p>
  <p id=\"nestedParsing\">There is only one set of states for the
  tokenizer stage and the tree construction stage...</p>".into_source();
        let mut parser = Builder::new()
            .with_attribute(TagName::A,"href")
            .with_attribute(TagName::Img,"alt")
            .create();

        let mut res_iter = [
            ParserEvent::Char('\n').localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 1, length: 1 },Snip { offset: 1, length: 1 }),
                end: ().localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 1, length: 1 },Snip { offset: 1, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 2, length: 1 },Snip { offset: 2, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
                ],
            }).localize(Snip { offset: 1, length: 3 },Snip { offset: 1, length: 3 }),
            ParserEvent::Char('I').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 13, length: 1 },Snip { offset: 13, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 14, length: 1 },Snip { offset: 14, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 15, length: 1 },Snip { offset: 15, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 16, length: 1 },Snip { offset: 16, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 18, length: 1 },Snip { offset: 18, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 19, length: 1 },Snip { offset: 19, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 20, length: 1 },Snip { offset: 20, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
            ParserEvent::Char(',').localize(Snip { offset: 22, length: 1 },Snip { offset: 22, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 23, length: 1 },Snip { offset: 23, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::A, closing: Closing::Open,
                attributes: OptVec::One(("href".to_string(), Some(Snip{ offset: 9, length: 51 }))),
                begin: ().localize(Snip { offset: 24, length: 1 },Snip { offset: 24, length: 1 }),
                end: ().localize(Snip { offset: 121, length: 1 },Snip { offset: 121, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 24, length: 1 },Snip { offset: 24, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 25, length: 1 },Snip { offset: 25, length: 1 }),
                    SourceEvent::Char(' ').localize(Snip { offset: 26, length: 1 },Snip { offset: 26, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 27, length: 1 },Snip { offset: 27, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 28, length: 1 },Snip { offset: 28, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 29, length: 1 },Snip { offset: 29, length: 1 }),
                    SourceEvent::Char('f').localize(Snip { offset: 30, length: 1 },Snip { offset: 30, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 31, length: 1 },Snip { offset: 31, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 32, length: 1 },Snip { offset: 32, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 33, length: 1 },Snip { offset: 33, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 34, length: 1 },Snip { offset: 34, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 35, length: 1 },Snip { offset: 35, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 36, length: 1 },Snip { offset: 36, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 37, length: 1 },Snip { offset: 37, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 38, length: 1 },Snip { offset: 38, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 39, length: 1 },Snip { offset: 39, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 40, length: 1 },Snip { offset: 40, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 41, length: 1 },Snip { offset: 41, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 42, length: 1 },Snip { offset: 42, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 43, length: 1 },Snip { offset: 43, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 44, length: 1 },Snip { offset: 44, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 45, length: 1 },Snip { offset: 45, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 46, length: 1 },Snip { offset: 46, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 47, length: 1 },Snip { offset: 47, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 48, length: 1 },Snip { offset: 48, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 49, length: 1 },Snip { offset: 49, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 50, length: 1 },Snip { offset: 50, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 51, length: 1 },Snip { offset: 51, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 52, length: 1 },Snip { offset: 52, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 53, length: 1 },Snip { offset: 53, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 54, length: 1 },Snip { offset: 54, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 55, length: 1 },Snip { offset: 55, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 56, length: 1 },Snip { offset: 56, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 57, length: 1 },Snip { offset: 57, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 58, length: 1 },Snip { offset: 58, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 59, length: 1 },Snip { offset: 59, length: 1 }),
                    SourceEvent::Char('#').localize(Snip { offset: 60, length: 1 },Snip { offset: 60, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 61, length: 1 },Snip { offset: 61, length: 1 }),
                    SourceEvent::Char('y').localize(Snip { offset: 62, length: 1 },Snip { offset: 62, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 63, length: 1 },Snip { offset: 63, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 64, length: 1 },Snip { offset: 64, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 65, length: 1 },Snip { offset: 65, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 66, length: 1 },Snip { offset: 66, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 67, length: 1 },Snip { offset: 67, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 68, length: 1 },Snip { offset: 68, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 69, length: 1 },Snip { offset: 69, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 70, length: 1 },Snip { offset: 70, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 71, length: 1 },Snip { offset: 71, length: 1 }),
                    SourceEvent::Char('k').localize(Snip { offset: 72, length: 1 },Snip { offset: 72, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 73, length: 1 },Snip { offset: 73, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 74, length: 1 },Snip { offset: 74, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 75, length: 1 },Snip { offset: 75, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 76, length: 1 },Snip { offset: 76, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 77, length: 1 },Snip { offset: 77, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 78, length: 1 },Snip { offset: 78, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 79, length: 1 },Snip { offset: 79, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 80, length: 1 },Snip { offset: 80, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 81, length: 1 },Snip { offset: 81, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 82, length: 1 },Snip { offset: 82, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 83, length: 1 },Snip { offset: 83, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 84, length: 1 },Snip { offset: 84, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 85, length: 1 },Snip { offset: 85, length: 1 }),
                    SourceEvent::Char(' ').localize(Snip { offset: 86, length: 1 },Snip { offset: 86, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 87, length: 1 },Snip { offset: 87, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 88, length: 1 },Snip { offset: 88, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 89, length: 1 },Snip { offset: 89, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 90, length: 1 },Snip { offset: 90, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 91, length: 1 },Snip { offset: 91, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 92, length: 1 },Snip { offset: 92, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 93, length: 1 },Snip { offset: 93, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 94, length: 1 },Snip { offset: 94, length: 1 }),
                    SourceEvent::Char('y').localize(Snip { offset: 95, length: 1 },Snip { offset: 95, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 96, length: 1 },Snip { offset: 96, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 97, length: 1 },Snip { offset: 97, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 98, length: 1 },Snip { offset: 98, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 99, length: 1 },Snip { offset: 99, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 100, length: 1 },Snip { offset: 100, length: 1 }),
                    SourceEvent::Char(' ').localize(Snip { offset: 101, length: 1 },Snip { offset: 101, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 102, length: 1 },Snip { offset: 102, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 103, length: 1 },Snip { offset: 103, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 104, length: 1 },Snip { offset: 104, length: 1 }),
                    SourceEvent::Char('k').localize(Snip { offset: 105, length: 1 },Snip { offset: 105, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 106, length: 1 },Snip { offset: 106, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 107, length: 1 },Snip { offset: 107, length: 1 }),
                    SourceEvent::Char('\n').localize(Snip { offset: 108, length: 1 },Snip { offset: 108, length: 1 }),
                    SourceEvent::Char(' ').localize(Snip { offset: 109, length: 1 },Snip { offset: 109, length: 1 }),
                    SourceEvent::Char(' ').localize(Snip { offset: 110, length: 1 },Snip { offset: 110, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 111, length: 1 },Snip { offset: 111, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 112, length: 1 },Snip { offset: 112, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 113, length: 1 },Snip { offset: 113, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 114, length: 1 },Snip { offset: 114, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 115, length: 1 },Snip { offset: 115, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 116, length: 1 },Snip { offset: 116, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 117, length: 1 },Snip { offset: 117, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 118, length: 1 },Snip { offset: 118, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 119, length: 1 },Snip { offset: 119, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 120, length: 1 },Snip { offset: 120, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 121, length: 1 },Snip { offset: 121, length: 1 }),
                ],
            }).localize(Snip { offset: 24, length: 98 },Snip { offset: 24, length: 98 }),                
            ParserEvent::Char(',').localize(Snip { offset: 122, length: 1 },Snip { offset: 122, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 123, length: 1 },Snip { offset: 123, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 124, length: 1 },Snip { offset: 124, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 125, length: 1 },Snip { offset: 125, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 126, length: 1 },Snip { offset: 126, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 127, length: 1 },Snip { offset: 127, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 128, length: 1 },Snip { offset: 128, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 129, length: 1 },Snip { offset: 129, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 130, length: 1 },Snip { offset: 130, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 131, length: 1 },Snip { offset: 131, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 132, length: 1 },Snip { offset: 132, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 133, length: 1 },Snip { offset: 133, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 134, length: 1 },Snip { offset: 134, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 135, length: 1 },Snip { offset: 135, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 136, length: 1 },Snip { offset: 136, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 137, length: 1 },Snip { offset: 137, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 138, length: 1 },Snip { offset: 138, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Code, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 139, length: 1 },Snip { offset: 139, length: 1 }),
                end: ().localize(Snip { offset: 171, length: 1 },Snip { offset: 171, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 139, length: 1 },Snip { offset: 139, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 140, length: 1 },Snip { offset: 140, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 141, length: 1 },Snip { offset: 141, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 142, length: 1 },Snip { offset: 142, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 143, length: 1 },Snip { offset: 143, length: 1 }),
                    SourceEvent::Char(' ').localize(Snip { offset: 144, length: 1 },Snip { offset: 144, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 145, length: 1 },Snip { offset: 145, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 146, length: 1 },Snip { offset: 146, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 147, length: 1 },Snip { offset: 147, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 148, length: 1 },Snip { offset: 148, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 149, length: 1 },Snip { offset: 149, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 150, length: 1 },Snip { offset: 150, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 151, length: 1 },Snip { offset: 151, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 152, length: 1 },Snip { offset: 152, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 153, length: 1 },Snip { offset: 153, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 154, length: 1 },Snip { offset: 154, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 155, length: 1 },Snip { offset: 155, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 156, length: 1 },Snip { offset: 156, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 157, length: 1 },Snip { offset: 157, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 158, length: 1 },Snip { offset: 158, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 159, length: 1 },Snip { offset: 159, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 160, length: 1 },Snip { offset: 160, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 161, length: 1 },Snip { offset: 161, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 162, length: 1 },Snip { offset: 162, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 163, length: 1 },Snip { offset: 163, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 164, length: 1 },Snip { offset: 164, length: 1 }),
                    SourceEvent::Char('w').localize(Snip { offset: 165, length: 1 },Snip { offset: 165, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 166, length: 1 },Snip { offset: 166, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 167, length: 1 },Snip { offset: 167, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 168, length: 1 },Snip { offset: 168, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 169, length: 1 },Snip { offset: 169, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 170, length: 1 },Snip { offset: 170, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 171, length: 1 },Snip { offset: 171, length: 1 }),
                ],
            }).localize(Snip { offset: 139, length: 33 },Snip { offset: 139, length: 33 }),
            ParserEvent::Parsed(Tag {
                name: TagName::A, closing: Closing::Open,
                attributes: OptVec::One(("href".to_string(), Some(Snip{ offset: 9, length: 45 }))),
                begin: ().localize(Snip { offset: 172, length: 1 },Snip { offset: 172, length: 1 }),
                end: ().localize(Snip { offset: 228, length: 1 },Snip { offset: 228, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 172, length: 1 },Snip { offset: 172, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 173, length: 1 },Snip { offset: 173, length: 1 }),
                    SourceEvent::Char(' ').localize(Snip { offset: 174, length: 1 },Snip { offset: 174, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 175, length: 1 },Snip { offset: 175, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 176, length: 1 },Snip { offset: 176, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 177, length: 1 },Snip { offset: 177, length: 1 }),
                    SourceEvent::Char('f').localize(Snip { offset: 178, length: 1 },Snip { offset: 178, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 179, length: 1 },Snip { offset: 179, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 180, length: 1 },Snip { offset: 180, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 181, length: 1 },Snip { offset: 181, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 182, length: 1 },Snip { offset: 182, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 183, length: 1 },Snip { offset: 183, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 184, length: 1 },Snip { offset: 184, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 185, length: 1 },Snip { offset: 185, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 186, length: 1 },Snip { offset: 186, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 187, length: 1 },Snip { offset: 187, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 188, length: 1 },Snip { offset: 188, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 189, length: 1 },Snip { offset: 189, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 190, length: 1 },Snip { offset: 190, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 191, length: 1 },Snip { offset: 191, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 192, length: 1 },Snip { offset: 192, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 193, length: 1 },Snip { offset: 193, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 194, length: 1 },Snip { offset: 194, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 195, length: 1 },Snip { offset: 195, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 196, length: 1 },Snip { offset: 196, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 197, length: 1 },Snip { offset: 197, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 198, length: 1 },Snip { offset: 198, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 199, length: 1 },Snip { offset: 199, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 200, length: 1 },Snip { offset: 200, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 201, length: 1 },Snip { offset: 201, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 202, length: 1 },Snip { offset: 202, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 203, length: 1 },Snip { offset: 203, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 204, length: 1 },Snip { offset: 204, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 205, length: 1 },Snip { offset: 205, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 206, length: 1 },Snip { offset: 206, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 207, length: 1 },Snip { offset: 207, length: 1 }),
                    SourceEvent::Char('#').localize(Snip { offset: 208, length: 1 },Snip { offset: 208, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 209, length: 1 },Snip { offset: 209, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 210, length: 1 },Snip { offset: 210, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 211, length: 1 },Snip { offset: 211, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 212, length: 1 },Snip { offset: 212, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 213, length: 1 },Snip { offset: 213, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 214, length: 1 },Snip { offset: 214, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 215, length: 1 },Snip { offset: 215, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 216, length: 1 },Snip { offset: 216, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 217, length: 1 },Snip { offset: 217, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 218, length: 1 },Snip { offset: 218, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 219, length: 1 },Snip { offset: 219, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 220, length: 1 },Snip { offset: 220, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 221, length: 1 },Snip { offset: 221, length: 1 }),
                    SourceEvent::Char('w').localize(Snip { offset: 222, length: 1 },Snip { offset: 222, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 223, length: 1 },Snip { offset: 223, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 224, length: 1 },Snip { offset: 224, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 225, length: 1 },Snip { offset: 225, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 226, length: 1 },Snip { offset: 226, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 227, length: 1 },Snip { offset: 227, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 228, length: 1 },Snip { offset: 228, length: 1 }),
                ],
            }).localize(Snip { offset: 172, length: 57 },Snip { offset: 172, length: 57 }), 
            ParserEvent::Char('d').localize(Snip { offset: 229, length: 1 },Snip { offset: 229, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 230, length: 1 },Snip { offset: 230, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 231, length: 1 },Snip { offset: 231, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 232, length: 1 },Snip { offset: 232, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 233, length: 1 },Snip { offset: 233, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 234, length: 1 },Snip { offset: 234, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 235, length: 1 },Snip { offset: 235, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 236, length: 1 },Snip { offset: 236, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 237, length: 1 },Snip { offset: 237, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 238, length: 1 },Snip { offset: 238, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 239, length: 1 },Snip { offset: 239, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 240, length: 1 },Snip { offset: 240, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 241, length: 1 },Snip { offset: 241, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 242, length: 1 },Snip { offset: 242, length: 1 }),
            ParserEvent::Char('(').localize(Snip { offset: 243, length: 1 },Snip { offset: 243, length: 1 }),
            ParserEvent::Char(')').localize(Snip { offset: 244, length: 1 },Snip { offset: 244, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::A, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 245, length: 1 },Snip { offset: 245, length: 1 }),
                end: ().localize(Snip { offset: 248, length: 1 },Snip { offset: 248, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 245, length: 1 },Snip { offset: 245, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 246, length: 1 },Snip { offset: 246, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 247, length: 1 },Snip { offset: 247, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 248, length: 1 },Snip { offset: 248, length: 1 }),
                ],
            }).localize(Snip { offset: 245, length: 4 },Snip { offset: 245, length: 4 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Code, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 249, length: 1 },Snip { offset: 249, length: 1 }),
                end: ().localize(Snip { offset: 255, length: 1 },Snip { offset: 255, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 249, length: 1 },Snip { offset: 249, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 250, length: 1 },Snip { offset: 250, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 251, length: 1 },Snip { offset: 251, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 252, length: 1 },Snip { offset: 252, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 253, length: 1 },Snip { offset: 253, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 254, length: 1 },Snip { offset: 254, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 255, length: 1 },Snip { offset: 255, length: 1 }),
                ],
            }).localize(Snip { offset: 249, length: 7 },Snip { offset: 249, length: 7 }),
            ParserEvent::Char(' ').localize(Snip { offset: 256, length: 1 },Snip { offset: 256, length: 1 }),
            ParserEvent::Char('A').localize(Snip { offset: 257, length: 1 },Snip { offset: 257, length: 1 }),
            ParserEvent::Char('P').localize(Snip { offset: 258, length: 1 },Snip { offset: 258, length: 1 }),
            ParserEvent::Char('I').localize(Snip { offset: 259, length: 1 },Snip { offset: 259, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 260, length: 1 },Snip { offset: 260, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 261, length: 1 },Snip { offset: 261, length: 1 }),
                end: ().localize(Snip { offset: 264, length: 1 },Snip { offset: 264, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 261, length: 1 },Snip { offset: 261, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 262, length: 1 },Snip { offset: 262, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 263, length: 1 },Snip { offset: 263, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 264, length: 1 },Snip { offset: 264, length: 1 }),
                ],
            }).localize(Snip { offset: 261, length: 4 },Snip { offset: 261, length: 4 }),
            ParserEvent::Char('\n').localize(Snip { offset: 265, length: 1 },Snip { offset: 265, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 266, length: 1 },Snip { offset: 266, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 267, length: 1 },Snip { offset: 267, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 268, length: 1 },Snip { offset: 268, length: 1 }),
                end: ().localize(Snip { offset: 270, length: 1 },Snip { offset: 270, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 268, length: 1 },Snip { offset: 268, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 269, length: 1 },Snip { offset: 269, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 270, length: 1 },Snip { offset: 270, length: 1 }),
                ],
            }).localize(Snip { offset: 268, length: 3 },Snip { offset: 268, length: 3 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Img, closing: Closing::Void,
                attributes: OptVec::One(("alt".to_string(), None)),
                begin: ().localize(Snip { offset: 271, length: 1 },Snip { offset: 271, length: 1 }),
                end: ().localize(Snip { offset: 377, length: 1 },Snip { offset: 377, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 271, length: 1 },Snip { offset: 271, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 272, length: 1 },Snip { offset: 272, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 273, length: 1 },Snip { offset: 273, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 274, length: 1 },Snip { offset: 274, length: 1 }),
                    SourceEvent::Char(' ').localize(Snip { offset: 275, length: 1 },Snip { offset: 275, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 276, length: 1 },Snip { offset: 276, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 277, length: 1 },Snip { offset: 277, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 278, length: 1 },Snip { offset: 278, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 279, length: 1 },Snip { offset: 279, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 280, length: 1 },Snip { offset: 280, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 281, length: 1 },Snip { offset: 281, length: 1 }),
                    SourceEvent::Char(' ').localize(Snip { offset: 282, length: 1 },Snip { offset: 282, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 283, length: 1 },Snip { offset: 283, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 284, length: 1 },Snip { offset: 284, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 285, length: 1 },Snip { offset: 285, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 286, length: 1 },Snip { offset: 286, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 287, length: 1 },Snip { offset: 287, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 288, length: 1 },Snip { offset: 288, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 289, length: 1 },Snip { offset: 289, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 290, length: 1 },Snip { offset: 290, length: 1 }),
                    SourceEvent::Char('5').localize(Snip { offset: 291, length: 1 },Snip { offset: 291, length: 1 }),
                    SourceEvent::Char('5').localize(Snip { offset: 292, length: 1 },Snip { offset: 292, length: 1 }),
                    SourceEvent::Char('4').localize(Snip { offset: 293, length: 1 },Snip { offset: 293, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 294, length: 1 },Snip { offset: 294, length: 1 }),
                    SourceEvent::Char(' ').localize(Snip { offset: 295, length: 1 },Snip { offset: 295, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 296, length: 1 },Snip { offset: 296, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 297, length: 1 },Snip { offset: 297, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 298, length: 1 },Snip { offset: 298, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 299, length: 1 },Snip { offset: 299, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 300, length: 1 },Snip { offset: 300, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 301, length: 1 },Snip { offset: 301, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 302, length: 1 },Snip { offset: 302, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 303, length: 1 },Snip { offset: 303, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 304, length: 1 },Snip { offset: 304, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 305, length: 1 },Snip { offset: 305, length: 1 }),
                    SourceEvent::Char(':').localize(Snip { offset: 306, length: 1 },Snip { offset: 306, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 307, length: 1 },Snip { offset: 307, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 308, length: 1 },Snip { offset: 308, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 309, length: 1 },Snip { offset: 309, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 310, length: 1 },Snip { offset: 310, length: 1 }),
                    SourceEvent::Char('v').localize(Snip { offset: 311, length: 1 },Snip { offset: 311, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 312, length: 1 },Snip { offset: 312, length: 1 }),
                    SourceEvent::Char('w').localize(Snip { offset: 313, length: 1 },Snip { offset: 313, length: 1 }),
                    SourceEvent::Char('3').localize(Snip { offset: 314, length: 1 },Snip { offset: 314, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 315, length: 1 },Snip { offset: 315, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 316, length: 1 },Snip { offset: 316, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 317, length: 1 },Snip { offset: 317, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 318, length: 1 },Snip { offset: 318, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 319, length: 1 },Snip { offset: 319, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 320, length: 1 },Snip { offset: 320, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 321, length: 1 },Snip { offset: 321, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 322, length: 1 },Snip { offset: 322, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 323, length: 1 },Snip { offset: 323, length: 1 }),
                    SourceEvent::Char('5').localize(Snip { offset: 324, length: 1 },Snip { offset: 324, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 325, length: 1 },Snip { offset: 325, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 326, length: 1 },Snip { offset: 326, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 327, length: 1 },Snip { offset: 327, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 328, length: 1 },Snip { offset: 328, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 329, length: 1 },Snip { offset: 329, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 330, length: 1 },Snip { offset: 330, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 331, length: 1 },Snip { offset: 331, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 332, length: 1 },Snip { offset: 332, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 333, length: 1 },Snip { offset: 333, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 334, length: 1 },Snip { offset: 334, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 335, length: 1 },Snip { offset: 335, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 336, length: 1 },Snip { offset: 336, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 337, length: 1 },Snip { offset: 337, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 338, length: 1 },Snip { offset: 338, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 339, length: 1 },Snip { offset: 339, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 340, length: 1 },Snip { offset: 340, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 341, length: 1 },Snip { offset: 341, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 342, length: 1 },Snip { offset: 342, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 343, length: 1 },Snip { offset: 343, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 344, length: 1 },Snip { offset: 344, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 345, length: 1 },Snip { offset: 345, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 346, length: 1 },Snip { offset: 346, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 347, length: 1 },Snip { offset: 347, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 348, length: 1 },Snip { offset: 348, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 349, length: 1 },Snip { offset: 349, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 350, length: 1 },Snip { offset: 350, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 351, length: 1 },Snip { offset: 351, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 352, length: 1 },Snip { offset: 352, length: 1 }),
                    SourceEvent::Char('v').localize(Snip { offset: 353, length: 1 },Snip { offset: 353, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 354, length: 1 },Snip { offset: 354, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 355, length: 1 },Snip { offset: 355, length: 1 }),
                    SourceEvent::Char('v').localize(Snip { offset: 356, length: 1 },Snip { offset: 356, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 357, length: 1 },Snip { offset: 357, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 358, length: 1 },Snip { offset: 358, length: 1 }),
                    SourceEvent::Char('w').localize(Snip { offset: 359, length: 1 },Snip { offset: 359, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 360, length: 1 },Snip { offset: 360, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 361, length: 1 },Snip { offset: 361, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 362, length: 1 },Snip { offset: 362, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 363, length: 1 },Snip { offset: 363, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 364, length: 1 },Snip { offset: 364, length: 1 }),
                    SourceEvent::Char(' ').localize(Snip { offset: 365, length: 1 },Snip { offset: 365, length: 1 }),
                    SourceEvent::Char('w').localize(Snip { offset: 366, length: 1 },Snip { offset: 366, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 367, length: 1 },Snip { offset: 367, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 368, length: 1 },Snip { offset: 368, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 369, length: 1 },Snip { offset: 369, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 370, length: 1 },Snip { offset: 370, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 371, length: 1 },Snip { offset: 371, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 372, length: 1 },Snip { offset: 372, length: 1 }),
                    SourceEvent::Char('4').localize(Snip { offset: 373, length: 1 },Snip { offset: 373, length: 1 }),
                    SourceEvent::Char('2').localize(Snip { offset: 374, length: 1 },Snip { offset: 374, length: 1 }),
                    SourceEvent::Char('7').localize(Snip { offset: 375, length: 1 },Snip { offset: 375, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 376, length: 1 },Snip { offset: 376, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 377, length: 1 },Snip { offset: 377, length: 1 }),
                ],
            }).localize(Snip { offset: 271, length: 107 },Snip { offset: 271, length: 107 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 378, length: 1 },Snip { offset: 378, length: 1 }),
                end: ().localize(Snip { offset: 381, length: 1 },Snip { offset: 381, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 378, length: 1 },Snip { offset: 378, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 379, length: 1 },Snip { offset: 379, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 380, length: 1 },Snip { offset: 380, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 381, length: 1 },Snip { offset: 381, length: 1 }),
                ],
            }).localize(Snip { offset: 378, length: 4 },Snip { offset: 378, length: 4 }),
            ParserEvent::Char('\n').localize(Snip { offset: 382, length: 1 },Snip { offset: 382, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 383, length: 1 },Snip { offset: 383, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 384, length: 1 },Snip { offset: 384, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 385, length: 1 },Snip { offset: 385, length: 1 }),
                end: ().localize(Snip { offset: 406, length: 1 },Snip { offset: 406, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 385, length: 1 },Snip { offset: 385, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 386, length: 1 },Snip { offset: 386, length: 1 }),
                    SourceEvent::Char(' ').localize(Snip { offset: 387, length: 1 },Snip { offset: 387, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 388, length: 1 },Snip { offset: 388, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 389, length: 1 },Snip { offset: 389, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 390, length: 1 },Snip { offset: 390, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 391, length: 1 },Snip { offset: 391, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 392, length: 1 },Snip { offset: 392, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 393, length: 1 },Snip { offset: 393, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 394, length: 1 },Snip { offset: 394, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 395, length: 1 },Snip { offset: 395, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 396, length: 1 },Snip { offset: 396, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 397, length: 1 },Snip { offset: 397, length: 1 }),
                    SourceEvent::Char('P').localize(Snip { offset: 398, length: 1 },Snip { offset: 398, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 399, length: 1 },Snip { offset: 399, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 400, length: 1 },Snip { offset: 400, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 401, length: 1 },Snip { offset: 401, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 402, length: 1 },Snip { offset: 402, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 403, length: 1 },Snip { offset: 403, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 404, length: 1 },Snip { offset: 404, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 405, length: 1 },Snip { offset: 405, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 406, length: 1 },Snip { offset: 406, length: 1 }),
                ],
            }).localize(Snip { offset: 385, length: 22 },Snip { offset: 385, length: 22 }),
            ParserEvent::Char('T').localize(Snip { offset: 407, length: 1 },Snip { offset: 407, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 408, length: 1 },Snip { offset: 408, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 409, length: 1 },Snip { offset: 409, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 410, length: 1 },Snip { offset: 410, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 411, length: 1 },Snip { offset: 411, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 412, length: 1 },Snip { offset: 412, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 413, length: 1 },Snip { offset: 413, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 414, length: 1 },Snip { offset: 414, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 415, length: 1 },Snip { offset: 415, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 416, length: 1 },Snip { offset: 416, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 417, length: 1 },Snip { offset: 417, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 418, length: 1 },Snip { offset: 418, length: 1 }),
            ParserEvent::Char('y').localize(Snip { offset: 419, length: 1 },Snip { offset: 419, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 420, length: 1 },Snip { offset: 420, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 421, length: 1 },Snip { offset: 421, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 422, length: 1 },Snip { offset: 422, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 423, length: 1 },Snip { offset: 423, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 424, length: 1 },Snip { offset: 424, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 425, length: 1 },Snip { offset: 425, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 426, length: 1 },Snip { offset: 426, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 427, length: 1 },Snip { offset: 427, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 428, length: 1 },Snip { offset: 428, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 429, length: 1 },Snip { offset: 429, length: 1 }),
            ParserEvent::Char('f').localize(Snip { offset: 430, length: 1 },Snip { offset: 430, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 431, length: 1 },Snip { offset: 431, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 432, length: 1 },Snip { offset: 432, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 433, length: 1 },Snip { offset: 433, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 434, length: 1 },Snip { offset: 434, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 435, length: 1 },Snip { offset: 435, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 436, length: 1 },Snip { offset: 436, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 437, length: 1 },Snip { offset: 437, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 438, length: 1 },Snip { offset: 438, length: 1 }),
            ParserEvent::Char('f').localize(Snip { offset: 439, length: 1 },Snip { offset: 439, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 440, length: 1 },Snip { offset: 440, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 441, length: 1 },Snip { offset: 441, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 442, length: 1 },Snip { offset: 442, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 443, length: 1 },Snip { offset: 443, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 444, length: 1 },Snip { offset: 444, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 445, length: 1 },Snip { offset: 445, length: 1 }),
            ParserEvent::Char('\n').localize(Snip { offset: 446, length: 1 },Snip { offset: 446, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 447, length: 1 },Snip { offset: 447, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 448, length: 1 },Snip { offset: 448, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 449, length: 1 },Snip { offset: 449, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 450, length: 1 },Snip { offset: 450, length: 1 }),
            ParserEvent::Char('k').localize(Snip { offset: 451, length: 1 },Snip { offset: 451, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 452, length: 1 },Snip { offset: 452, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 453, length: 1 },Snip { offset: 453, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 454, length: 1 },Snip { offset: 454, length: 1 }),
            ParserEvent::Char('z').localize(Snip { offset: 455, length: 1 },Snip { offset: 455, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 456, length: 1 },Snip { offset: 456, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 457, length: 1 },Snip { offset: 457, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 458, length: 1 },Snip { offset: 458, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 459, length: 1 },Snip { offset: 459, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 460, length: 1 },Snip { offset: 460, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 461, length: 1 },Snip { offset: 461, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 462, length: 1 },Snip { offset: 462, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 463, length: 1 },Snip { offset: 463, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 464, length: 1 },Snip { offset: 464, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 465, length: 1 },Snip { offset: 465, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 466, length: 1 },Snip { offset: 466, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 467, length: 1 },Snip { offset: 467, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 468, length: 1 },Snip { offset: 468, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 469, length: 1 },Snip { offset: 469, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 470, length: 1 },Snip { offset: 470, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 471, length: 1 },Snip { offset: 471, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 472, length: 1 },Snip { offset: 472, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 473, length: 1 },Snip { offset: 473, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 474, length: 1 },Snip { offset: 474, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 475, length: 1 },Snip { offset: 475, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 476, length: 1 },Snip { offset: 476, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 477, length: 1 },Snip { offset: 477, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 478, length: 1 },Snip { offset: 478, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 479, length: 1 },Snip { offset: 479, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 480, length: 1 },Snip { offset: 480, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 481, length: 1 },Snip { offset: 481, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 482, length: 1 },Snip { offset: 482, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 483, length: 1 },Snip { offset: 483, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 484, length: 1 },Snip { offset: 484, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 485, length: 1 },Snip { offset: 485, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 486, length: 1 },Snip { offset: 486, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 487, length: 1 },Snip { offset: 487, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 488, length: 1 },Snip { offset: 488, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 489, length: 1 },Snip { offset: 489, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 490, length: 1 },Snip { offset: 490, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 491, length: 1 },Snip { offset: 491, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 492, length: 1 },Snip { offset: 492, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 493, length: 1 },Snip { offset: 493, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 494, length: 1 },Snip { offset: 494, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 495, length: 1 },Snip { offset: 495, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 496, length: 1 },Snip { offset: 496, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 497, length: 1 },Snip { offset: 497, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 498, length: 1 },Snip { offset: 498, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 499, length: 1 },Snip { offset: 499, length: 1 }),
                end: ().localize(Snip { offset: 502, length: 1 },Snip { offset: 502, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 499, length: 1 },Snip { offset: 499, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 500, length: 1 },Snip { offset: 500, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 501, length: 1 },Snip { offset: 501, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 502, length: 1 },Snip { offset: 502, length: 1 }),
                ],
            }).localize(Snip { offset: 499, length: 4 },Snip { offset: 499, length: 4 }),
        ].into_iter();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            /*if let ParserEvent::Parsed(tag) = local_event.data() {
                for lse in &tag.raw {
                    let (l,e) = lse.into_inner();
                    println!("SourceEvent::{:?}.localize({:?},{:?}),",e,l.chars(),l.bytes());
                }
                println!("");
            }*/
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
                    println!("Result: {:?}",ev);                    
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

    
    #[test]
    fn a_img_2() {        
        let mut src = "
<p>In the common case, <a href=\"apis-in-html-documents.html#dynamic-markup-insertion\" title=\"dynamic markup
  insertion\">, e.g. using the <code title=\"dom-document-write\"><a href=\"apis-in-html-documents.html#dom-document-write\">document.write()</a></code> API.</p>
  <p><img alt=\"\" height=\"554\" src=\"https://dev.w3.org/html5/spec/images/parsing-model-overview.png\" width=\"427\"></p>
  <p id=\"nestedParsing\">There is only one set of states for the
  tokenizer stage and the tree construction stage...</p>".into_source().into_separator().merge_separators();
        let mut parser = Builder::new()
            .with_attribute(TagName::A,"href")
            .with_attribute(TagName::Img,"alt")
            .create();

        let mut res_iter = [
            ParserEvent::Breaker(Breaker::Line).localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 1, length: 1 },Snip { offset: 1, length: 1 }),
                end: ().localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 1, length: 1 },Snip { offset: 1, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 2, length: 1 },Snip { offset: 2, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
                ],
            }).localize(Snip { offset: 1, length: 3 },Snip { offset: 1, length: 3 }),
            ParserEvent::Char('I').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 13, length: 1 },Snip { offset: 13, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 14, length: 1 },Snip { offset: 14, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 15, length: 1 },Snip { offset: 15, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 16, length: 1 },Snip { offset: 16, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 18, length: 1 },Snip { offset: 18, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 19, length: 1 },Snip { offset: 19, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 20, length: 1 },Snip { offset: 20, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
            ParserEvent::Char(',').localize(Snip { offset: 22, length: 1 },Snip { offset: 22, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 23, length: 1 },Snip { offset: 23, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::A, closing: Closing::Open,
                attributes: OptVec::One(("href".to_string(), Some(Snip{ offset: 9, length: 51 }))),                
                begin: ().localize(Snip { offset: 24, length: 1 },Snip { offset: 24, length: 1 }),
                end: ().localize(Snip { offset: 121, length: 1 },Snip { offset: 121, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 24, length: 1 },Snip { offset: 24, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 25, length: 1 },Snip { offset: 25, length: 1 }),
                    SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 26, length: 1 },Snip { offset: 26, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 27, length: 1 },Snip { offset: 27, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 28, length: 1 },Snip { offset: 28, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 29, length: 1 },Snip { offset: 29, length: 1 }),
                    SourceEvent::Char('f').localize(Snip { offset: 30, length: 1 },Snip { offset: 30, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 31, length: 1 },Snip { offset: 31, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 32, length: 1 },Snip { offset: 32, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 33, length: 1 },Snip { offset: 33, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 34, length: 1 },Snip { offset: 34, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 35, length: 1 },Snip { offset: 35, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 36, length: 1 },Snip { offset: 36, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 37, length: 1 },Snip { offset: 37, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 38, length: 1 },Snip { offset: 38, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 39, length: 1 },Snip { offset: 39, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 40, length: 1 },Snip { offset: 40, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 41, length: 1 },Snip { offset: 41, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 42, length: 1 },Snip { offset: 42, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 43, length: 1 },Snip { offset: 43, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 44, length: 1 },Snip { offset: 44, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 45, length: 1 },Snip { offset: 45, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 46, length: 1 },Snip { offset: 46, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 47, length: 1 },Snip { offset: 47, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 48, length: 1 },Snip { offset: 48, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 49, length: 1 },Snip { offset: 49, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 50, length: 1 },Snip { offset: 50, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 51, length: 1 },Snip { offset: 51, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 52, length: 1 },Snip { offset: 52, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 53, length: 1 },Snip { offset: 53, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 54, length: 1 },Snip { offset: 54, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 55, length: 1 },Snip { offset: 55, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 56, length: 1 },Snip { offset: 56, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 57, length: 1 },Snip { offset: 57, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 58, length: 1 },Snip { offset: 58, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 59, length: 1 },Snip { offset: 59, length: 1 }),
                    SourceEvent::Char('#').localize(Snip { offset: 60, length: 1 },Snip { offset: 60, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 61, length: 1 },Snip { offset: 61, length: 1 }),
                    SourceEvent::Char('y').localize(Snip { offset: 62, length: 1 },Snip { offset: 62, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 63, length: 1 },Snip { offset: 63, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 64, length: 1 },Snip { offset: 64, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 65, length: 1 },Snip { offset: 65, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 66, length: 1 },Snip { offset: 66, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 67, length: 1 },Snip { offset: 67, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 68, length: 1 },Snip { offset: 68, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 69, length: 1 },Snip { offset: 69, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 70, length: 1 },Snip { offset: 70, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 71, length: 1 },Snip { offset: 71, length: 1 }),
                    SourceEvent::Char('k').localize(Snip { offset: 72, length: 1 },Snip { offset: 72, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 73, length: 1 },Snip { offset: 73, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 74, length: 1 },Snip { offset: 74, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 75, length: 1 },Snip { offset: 75, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 76, length: 1 },Snip { offset: 76, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 77, length: 1 },Snip { offset: 77, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 78, length: 1 },Snip { offset: 78, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 79, length: 1 },Snip { offset: 79, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 80, length: 1 },Snip { offset: 80, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 81, length: 1 },Snip { offset: 81, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 82, length: 1 },Snip { offset: 82, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 83, length: 1 },Snip { offset: 83, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 84, length: 1 },Snip { offset: 84, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 85, length: 1 },Snip { offset: 85, length: 1 }),
                    SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 86, length: 1 },Snip { offset: 86, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 87, length: 1 },Snip { offset: 87, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 88, length: 1 },Snip { offset: 88, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 89, length: 1 },Snip { offset: 89, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 90, length: 1 },Snip { offset: 90, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 91, length: 1 },Snip { offset: 91, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 92, length: 1 },Snip { offset: 92, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 93, length: 1 },Snip { offset: 93, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 94, length: 1 },Snip { offset: 94, length: 1 }),
                    SourceEvent::Char('y').localize(Snip { offset: 95, length: 1 },Snip { offset: 95, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 96, length: 1 },Snip { offset: 96, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 97, length: 1 },Snip { offset: 97, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 98, length: 1 },Snip { offset: 98, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 99, length: 1 },Snip { offset: 99, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 100, length: 1 },Snip { offset: 100, length: 1 }),
                    SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 101, length: 1 },Snip { offset: 101, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 102, length: 1 },Snip { offset: 102, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 103, length: 1 },Snip { offset: 103, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 104, length: 1 },Snip { offset: 104, length: 1 }),
                    SourceEvent::Char('k').localize(Snip { offset: 105, length: 1 },Snip { offset: 105, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 106, length: 1 },Snip { offset: 106, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 107, length: 1 },Snip { offset: 107, length: 1 }),
                    SourceEvent::Breaker(Breaker::Line).localize(Snip { offset: 108, length: 3 },Snip { offset: 108, length: 3 }),
                    SourceEvent::Char('i').localize(Snip { offset: 111, length: 1 },Snip { offset: 111, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 112, length: 1 },Snip { offset: 112, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 113, length: 1 },Snip { offset: 113, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 114, length: 1 },Snip { offset: 114, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 115, length: 1 },Snip { offset: 115, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 116, length: 1 },Snip { offset: 116, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 117, length: 1 },Snip { offset: 117, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 118, length: 1 },Snip { offset: 118, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 119, length: 1 },Snip { offset: 119, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 120, length: 1 },Snip { offset: 120, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 121, length: 1 },Snip { offset: 121, length: 1 }),
                ],
            }).localize(Snip { offset: 24, length: 98 },Snip { offset: 24, length: 98 }),
            ParserEvent::Char(',').localize(Snip { offset: 122, length: 1 },Snip { offset: 122, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 123, length: 1 },Snip { offset: 123, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 124, length: 1 },Snip { offset: 124, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 125, length: 1 },Snip { offset: 125, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 126, length: 1 },Snip { offset: 126, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 127, length: 1 },Snip { offset: 127, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 128, length: 1 },Snip { offset: 128, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 129, length: 1 },Snip { offset: 129, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 130, length: 1 },Snip { offset: 130, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 131, length: 1 },Snip { offset: 131, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 132, length: 1 },Snip { offset: 132, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 133, length: 1 },Snip { offset: 133, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 134, length: 1 },Snip { offset: 134, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 135, length: 1 },Snip { offset: 135, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 136, length: 1 },Snip { offset: 136, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 137, length: 1 },Snip { offset: 137, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 138, length: 1 },Snip { offset: 138, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Code, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 139, length: 1 },Snip { offset: 139, length: 1 }),
                end: ().localize(Snip { offset: 171, length: 1 },Snip { offset: 171, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 139, length: 1 },Snip { offset: 139, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 140, length: 1 },Snip { offset: 140, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 141, length: 1 },Snip { offset: 141, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 142, length: 1 },Snip { offset: 142, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 143, length: 1 },Snip { offset: 143, length: 1 }),
                    SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 144, length: 1 },Snip { offset: 144, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 145, length: 1 },Snip { offset: 145, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 146, length: 1 },Snip { offset: 146, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 147, length: 1 },Snip { offset: 147, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 148, length: 1 },Snip { offset: 148, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 149, length: 1 },Snip { offset: 149, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 150, length: 1 },Snip { offset: 150, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 151, length: 1 },Snip { offset: 151, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 152, length: 1 },Snip { offset: 152, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 153, length: 1 },Snip { offset: 153, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 154, length: 1 },Snip { offset: 154, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 155, length: 1 },Snip { offset: 155, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 156, length: 1 },Snip { offset: 156, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 157, length: 1 },Snip { offset: 157, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 158, length: 1 },Snip { offset: 158, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 159, length: 1 },Snip { offset: 159, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 160, length: 1 },Snip { offset: 160, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 161, length: 1 },Snip { offset: 161, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 162, length: 1 },Snip { offset: 162, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 163, length: 1 },Snip { offset: 163, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 164, length: 1 },Snip { offset: 164, length: 1 }),
                    SourceEvent::Char('w').localize(Snip { offset: 165, length: 1 },Snip { offset: 165, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 166, length: 1 },Snip { offset: 166, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 167, length: 1 },Snip { offset: 167, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 168, length: 1 },Snip { offset: 168, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 169, length: 1 },Snip { offset: 169, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 170, length: 1 },Snip { offset: 170, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 171, length: 1 },Snip { offset: 171, length: 1 }),
                ],
            }).localize(Snip { offset: 139, length: 33 },Snip { offset: 139, length: 33 }),
            ParserEvent::Parsed(Tag {
                name: TagName::A, closing: Closing::Open,
                attributes: OptVec::One(("href".to_string(), Some(Snip{ offset: 9, length: 45 }))),                
                begin: ().localize(Snip { offset: 172, length: 1 },Snip { offset: 172, length: 1 }),
                end: ().localize(Snip { offset: 228, length: 1 },Snip { offset: 228, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 172, length: 1 },Snip { offset: 172, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 173, length: 1 },Snip { offset: 173, length: 1 }),
                    SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 174, length: 1 },Snip { offset: 174, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 175, length: 1 },Snip { offset: 175, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 176, length: 1 },Snip { offset: 176, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 177, length: 1 },Snip { offset: 177, length: 1 }),
                    SourceEvent::Char('f').localize(Snip { offset: 178, length: 1 },Snip { offset: 178, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 179, length: 1 },Snip { offset: 179, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 180, length: 1 },Snip { offset: 180, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 181, length: 1 },Snip { offset: 181, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 182, length: 1 },Snip { offset: 182, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 183, length: 1 },Snip { offset: 183, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 184, length: 1 },Snip { offset: 184, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 185, length: 1 },Snip { offset: 185, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 186, length: 1 },Snip { offset: 186, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 187, length: 1 },Snip { offset: 187, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 188, length: 1 },Snip { offset: 188, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 189, length: 1 },Snip { offset: 189, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 190, length: 1 },Snip { offset: 190, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 191, length: 1 },Snip { offset: 191, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 192, length: 1 },Snip { offset: 192, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 193, length: 1 },Snip { offset: 193, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 194, length: 1 },Snip { offset: 194, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 195, length: 1 },Snip { offset: 195, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 196, length: 1 },Snip { offset: 196, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 197, length: 1 },Snip { offset: 197, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 198, length: 1 },Snip { offset: 198, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 199, length: 1 },Snip { offset: 199, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 200, length: 1 },Snip { offset: 200, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 201, length: 1 },Snip { offset: 201, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 202, length: 1 },Snip { offset: 202, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 203, length: 1 },Snip { offset: 203, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 204, length: 1 },Snip { offset: 204, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 205, length: 1 },Snip { offset: 205, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 206, length: 1 },Snip { offset: 206, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 207, length: 1 },Snip { offset: 207, length: 1 }),
                    SourceEvent::Char('#').localize(Snip { offset: 208, length: 1 },Snip { offset: 208, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 209, length: 1 },Snip { offset: 209, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 210, length: 1 },Snip { offset: 210, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 211, length: 1 },Snip { offset: 211, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 212, length: 1 },Snip { offset: 212, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 213, length: 1 },Snip { offset: 213, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 214, length: 1 },Snip { offset: 214, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 215, length: 1 },Snip { offset: 215, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 216, length: 1 },Snip { offset: 216, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 217, length: 1 },Snip { offset: 217, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 218, length: 1 },Snip { offset: 218, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 219, length: 1 },Snip { offset: 219, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 220, length: 1 },Snip { offset: 220, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 221, length: 1 },Snip { offset: 221, length: 1 }),
                    SourceEvent::Char('w').localize(Snip { offset: 222, length: 1 },Snip { offset: 222, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 223, length: 1 },Snip { offset: 223, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 224, length: 1 },Snip { offset: 224, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 225, length: 1 },Snip { offset: 225, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 226, length: 1 },Snip { offset: 226, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 227, length: 1 },Snip { offset: 227, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 228, length: 1 },Snip { offset: 228, length: 1 }),
                ],
            }).localize(Snip { offset: 172, length: 57 },Snip { offset: 172, length: 57 }),
            ParserEvent::Char('d').localize(Snip { offset: 229, length: 1 },Snip { offset: 229, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 230, length: 1 },Snip { offset: 230, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 231, length: 1 },Snip { offset: 231, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 232, length: 1 },Snip { offset: 232, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 233, length: 1 },Snip { offset: 233, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 234, length: 1 },Snip { offset: 234, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 235, length: 1 },Snip { offset: 235, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 236, length: 1 },Snip { offset: 236, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 237, length: 1 },Snip { offset: 237, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 238, length: 1 },Snip { offset: 238, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 239, length: 1 },Snip { offset: 239, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 240, length: 1 },Snip { offset: 240, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 241, length: 1 },Snip { offset: 241, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 242, length: 1 },Snip { offset: 242, length: 1 }),
            ParserEvent::Char('(').localize(Snip { offset: 243, length: 1 },Snip { offset: 243, length: 1 }),
            ParserEvent::Char(')').localize(Snip { offset: 244, length: 1 },Snip { offset: 244, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::A, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 245, length: 1 },Snip { offset: 245, length: 1 }),
                end: ().localize(Snip { offset: 248, length: 1 },Snip { offset: 248, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 245, length: 1 },Snip { offset: 245, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 246, length: 1 },Snip { offset: 246, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 247, length: 1 },Snip { offset: 247, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 248, length: 1 },Snip { offset: 248, length: 1 }),
                ],
            }).localize(Snip { offset: 245, length: 4 },Snip { offset: 245, length: 4 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Code, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 249, length: 1 },Snip { offset: 249, length: 1 }),
                end: ().localize(Snip { offset: 255, length: 1 },Snip { offset: 255, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 249, length: 1 },Snip { offset: 249, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 250, length: 1 },Snip { offset: 250, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 251, length: 1 },Snip { offset: 251, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 252, length: 1 },Snip { offset: 252, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 253, length: 1 },Snip { offset: 253, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 254, length: 1 },Snip { offset: 254, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 255, length: 1 },Snip { offset: 255, length: 1 }),
                ],
            }).localize(Snip { offset: 249, length: 7 },Snip { offset: 249, length: 7 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 256, length: 1 },Snip { offset: 256, length: 1 }),
            ParserEvent::Char('A').localize(Snip { offset: 257, length: 1 },Snip { offset: 257, length: 1 }),
            ParserEvent::Char('P').localize(Snip { offset: 258, length: 1 },Snip { offset: 258, length: 1 }),
            ParserEvent::Char('I').localize(Snip { offset: 259, length: 1 },Snip { offset: 259, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 260, length: 1 },Snip { offset: 260, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 261, length: 1 },Snip { offset: 261, length: 1 }),
                end: ().localize(Snip { offset: 264, length: 1 },Snip { offset: 264, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 261, length: 1 },Snip { offset: 261, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 262, length: 1 },Snip { offset: 262, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 263, length: 1 },Snip { offset: 263, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 264, length: 1 },Snip { offset: 264, length: 1 }),
                ],
            }).localize(Snip { offset: 261, length: 4 },Snip { offset: 261, length: 4 }),
            ParserEvent::Breaker(Breaker::Line).localize(Snip { offset: 265, length: 3 },Snip { offset: 265, length: 3 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 268, length: 1 },Snip { offset: 268, length: 1 }),
                end: ().localize(Snip { offset: 270, length: 1 },Snip { offset: 270, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 268, length: 1 },Snip { offset: 268, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 269, length: 1 },Snip { offset: 269, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 270, length: 1 },Snip { offset: 270, length: 1 }),
                ],
            }).localize(Snip { offset: 268, length: 3 },Snip { offset: 268, length: 3 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Img, closing: Closing::Void,
                attributes: OptVec::One(("alt".to_string(), None)),
                begin: ().localize(Snip { offset: 271, length: 1 },Snip { offset: 271, length: 1 }),
                end: ().localize(Snip { offset: 377, length: 1 },Snip { offset: 377, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 271, length: 1 },Snip { offset: 271, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 272, length: 1 },Snip { offset: 272, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 273, length: 1 },Snip { offset: 273, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 274, length: 1 },Snip { offset: 274, length: 1 }),
                    SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 275, length: 1 },Snip { offset: 275, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 276, length: 1 },Snip { offset: 276, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 277, length: 1 },Snip { offset: 277, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 278, length: 1 },Snip { offset: 278, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 279, length: 1 },Snip { offset: 279, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 280, length: 1 },Snip { offset: 280, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 281, length: 1 },Snip { offset: 281, length: 1 }),
                    SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 282, length: 1 },Snip { offset: 282, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 283, length: 1 },Snip { offset: 283, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 284, length: 1 },Snip { offset: 284, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 285, length: 1 },Snip { offset: 285, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 286, length: 1 },Snip { offset: 286, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 287, length: 1 },Snip { offset: 287, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 288, length: 1 },Snip { offset: 288, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 289, length: 1 },Snip { offset: 289, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 290, length: 1 },Snip { offset: 290, length: 1 }),
                    SourceEvent::Char('5').localize(Snip { offset: 291, length: 1 },Snip { offset: 291, length: 1 }),
                    SourceEvent::Char('5').localize(Snip { offset: 292, length: 1 },Snip { offset: 292, length: 1 }),
                    SourceEvent::Char('4').localize(Snip { offset: 293, length: 1 },Snip { offset: 293, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 294, length: 1 },Snip { offset: 294, length: 1 }),
                    SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 295, length: 1 },Snip { offset: 295, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 296, length: 1 },Snip { offset: 296, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 297, length: 1 },Snip { offset: 297, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 298, length: 1 },Snip { offset: 298, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 299, length: 1 },Snip { offset: 299, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 300, length: 1 },Snip { offset: 300, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 301, length: 1 },Snip { offset: 301, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 302, length: 1 },Snip { offset: 302, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 303, length: 1 },Snip { offset: 303, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 304, length: 1 },Snip { offset: 304, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 305, length: 1 },Snip { offset: 305, length: 1 }),
                    SourceEvent::Char(':').localize(Snip { offset: 306, length: 1 },Snip { offset: 306, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 307, length: 1 },Snip { offset: 307, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 308, length: 1 },Snip { offset: 308, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 309, length: 1 },Snip { offset: 309, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 310, length: 1 },Snip { offset: 310, length: 1 }),
                    SourceEvent::Char('v').localize(Snip { offset: 311, length: 1 },Snip { offset: 311, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 312, length: 1 },Snip { offset: 312, length: 1 }),
                    SourceEvent::Char('w').localize(Snip { offset: 313, length: 1 },Snip { offset: 313, length: 1 }),
                    SourceEvent::Char('3').localize(Snip { offset: 314, length: 1 },Snip { offset: 314, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 315, length: 1 },Snip { offset: 315, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 316, length: 1 },Snip { offset: 316, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 317, length: 1 },Snip { offset: 317, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 318, length: 1 },Snip { offset: 318, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 319, length: 1 },Snip { offset: 319, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 320, length: 1 },Snip { offset: 320, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 321, length: 1 },Snip { offset: 321, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 322, length: 1 },Snip { offset: 322, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 323, length: 1 },Snip { offset: 323, length: 1 }),
                    SourceEvent::Char('5').localize(Snip { offset: 324, length: 1 },Snip { offset: 324, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 325, length: 1 },Snip { offset: 325, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 326, length: 1 },Snip { offset: 326, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 327, length: 1 },Snip { offset: 327, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 328, length: 1 },Snip { offset: 328, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 329, length: 1 },Snip { offset: 329, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 330, length: 1 },Snip { offset: 330, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 331, length: 1 },Snip { offset: 331, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 332, length: 1 },Snip { offset: 332, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 333, length: 1 },Snip { offset: 333, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 334, length: 1 },Snip { offset: 334, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 335, length: 1 },Snip { offset: 335, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 336, length: 1 },Snip { offset: 336, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 337, length: 1 },Snip { offset: 337, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 338, length: 1 },Snip { offset: 338, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 339, length: 1 },Snip { offset: 339, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 340, length: 1 },Snip { offset: 340, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 341, length: 1 },Snip { offset: 341, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 342, length: 1 },Snip { offset: 342, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 343, length: 1 },Snip { offset: 343, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 344, length: 1 },Snip { offset: 344, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 345, length: 1 },Snip { offset: 345, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 346, length: 1 },Snip { offset: 346, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 347, length: 1 },Snip { offset: 347, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 348, length: 1 },Snip { offset: 348, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 349, length: 1 },Snip { offset: 349, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 350, length: 1 },Snip { offset: 350, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 351, length: 1 },Snip { offset: 351, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 352, length: 1 },Snip { offset: 352, length: 1 }),
                    SourceEvent::Char('v').localize(Snip { offset: 353, length: 1 },Snip { offset: 353, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 354, length: 1 },Snip { offset: 354, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 355, length: 1 },Snip { offset: 355, length: 1 }),
                    SourceEvent::Char('v').localize(Snip { offset: 356, length: 1 },Snip { offset: 356, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 357, length: 1 },Snip { offset: 357, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 358, length: 1 },Snip { offset: 358, length: 1 }),
                    SourceEvent::Char('w').localize(Snip { offset: 359, length: 1 },Snip { offset: 359, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 360, length: 1 },Snip { offset: 360, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 361, length: 1 },Snip { offset: 361, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 362, length: 1 },Snip { offset: 362, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 363, length: 1 },Snip { offset: 363, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 364, length: 1 },Snip { offset: 364, length: 1 }),
                    SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 365, length: 1 },Snip { offset: 365, length: 1 }),
                    SourceEvent::Char('w').localize(Snip { offset: 366, length: 1 },Snip { offset: 366, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 367, length: 1 },Snip { offset: 367, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 368, length: 1 },Snip { offset: 368, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 369, length: 1 },Snip { offset: 369, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 370, length: 1 },Snip { offset: 370, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 371, length: 1 },Snip { offset: 371, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 372, length: 1 },Snip { offset: 372, length: 1 }),
                    SourceEvent::Char('4').localize(Snip { offset: 373, length: 1 },Snip { offset: 373, length: 1 }),
                    SourceEvent::Char('2').localize(Snip { offset: 374, length: 1 },Snip { offset: 374, length: 1 }),
                    SourceEvent::Char('7').localize(Snip { offset: 375, length: 1 },Snip { offset: 375, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 376, length: 1 },Snip { offset: 376, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 377, length: 1 },Snip { offset: 377, length: 1 }),
                ],
            }).localize(Snip { offset: 271, length: 107 },Snip { offset: 271, length: 107 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 378, length: 1 },Snip { offset: 378, length: 1 }),
                end: ().localize(Snip { offset: 381, length: 1 },Snip { offset: 381, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 378, length: 1 },Snip { offset: 378, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 379, length: 1 },Snip { offset: 379, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 380, length: 1 },Snip { offset: 380, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 381, length: 1 },Snip { offset: 381, length: 1 }),
                ],
            }).localize(Snip { offset: 378, length: 4 },Snip { offset: 378, length: 4 }),
            ParserEvent::Breaker(Breaker::Line).localize(Snip { offset: 382, length: 3 },Snip { offset: 382, length: 3 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 385, length: 1 },Snip { offset: 385, length: 1 }),
                end: ().localize(Snip { offset: 406, length: 1 },Snip { offset: 406, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 385, length: 1 },Snip { offset: 385, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 386, length: 1 },Snip { offset: 386, length: 1 }),
                    SourceEvent::Breaker(Breaker::Space).localize(Snip { offset: 387, length: 1 },Snip { offset: 387, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 388, length: 1 },Snip { offset: 388, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 389, length: 1 },Snip { offset: 389, length: 1 }),
                    SourceEvent::Char('=').localize(Snip { offset: 390, length: 1 },Snip { offset: 390, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 391, length: 1 },Snip { offset: 391, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 392, length: 1 },Snip { offset: 392, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 393, length: 1 },Snip { offset: 393, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 394, length: 1 },Snip { offset: 394, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 395, length: 1 },Snip { offset: 395, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 396, length: 1 },Snip { offset: 396, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 397, length: 1 },Snip { offset: 397, length: 1 }),
                    SourceEvent::Char('P').localize(Snip { offset: 398, length: 1 },Snip { offset: 398, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 399, length: 1 },Snip { offset: 399, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 400, length: 1 },Snip { offset: 400, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 401, length: 1 },Snip { offset: 401, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 402, length: 1 },Snip { offset: 402, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 403, length: 1 },Snip { offset: 403, length: 1 }),
                    SourceEvent::Char('g').localize(Snip { offset: 404, length: 1 },Snip { offset: 404, length: 1 }),
                    SourceEvent::Char('"').localize(Snip { offset: 405, length: 1 },Snip { offset: 405, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 406, length: 1 },Snip { offset: 406, length: 1 }),
                ],
            }).localize(Snip { offset: 385, length: 22 },Snip { offset: 385, length: 22 }),
            ParserEvent::Char('T').localize(Snip { offset: 407, length: 1 },Snip { offset: 407, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 408, length: 1 },Snip { offset: 408, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 409, length: 1 },Snip { offset: 409, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 410, length: 1 },Snip { offset: 410, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 411, length: 1 },Snip { offset: 411, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 412, length: 1 },Snip { offset: 412, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 413, length: 1 },Snip { offset: 413, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 414, length: 1 },Snip { offset: 414, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 415, length: 1 },Snip { offset: 415, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 416, length: 1 },Snip { offset: 416, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 417, length: 1 },Snip { offset: 417, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 418, length: 1 },Snip { offset: 418, length: 1 }),
            ParserEvent::Char('y').localize(Snip { offset: 419, length: 1 },Snip { offset: 419, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 420, length: 1 },Snip { offset: 420, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 421, length: 1 },Snip { offset: 421, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 422, length: 1 },Snip { offset: 422, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 423, length: 1 },Snip { offset: 423, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 424, length: 1 },Snip { offset: 424, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 425, length: 1 },Snip { offset: 425, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 426, length: 1 },Snip { offset: 426, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 427, length: 1 },Snip { offset: 427, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 428, length: 1 },Snip { offset: 428, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 429, length: 1 },Snip { offset: 429, length: 1 }),
            ParserEvent::Char('f').localize(Snip { offset: 430, length: 1 },Snip { offset: 430, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 431, length: 1 },Snip { offset: 431, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 432, length: 1 },Snip { offset: 432, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 433, length: 1 },Snip { offset: 433, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 434, length: 1 },Snip { offset: 434, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 435, length: 1 },Snip { offset: 435, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 436, length: 1 },Snip { offset: 436, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 437, length: 1 },Snip { offset: 437, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 438, length: 1 },Snip { offset: 438, length: 1 }),
            ParserEvent::Char('f').localize(Snip { offset: 439, length: 1 },Snip { offset: 439, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 440, length: 1 },Snip { offset: 440, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 441, length: 1 },Snip { offset: 441, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 442, length: 1 },Snip { offset: 442, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 443, length: 1 },Snip { offset: 443, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 444, length: 1 },Snip { offset: 444, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 445, length: 1 },Snip { offset: 445, length: 1 }),
            ParserEvent::Breaker(Breaker::Line).localize(Snip { offset: 446, length: 3 },Snip { offset: 446, length: 3 }),
            ParserEvent::Char('t').localize(Snip { offset: 449, length: 1 },Snip { offset: 449, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 450, length: 1 },Snip { offset: 450, length: 1 }),
            ParserEvent::Char('k').localize(Snip { offset: 451, length: 1 },Snip { offset: 451, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 452, length: 1 },Snip { offset: 452, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 453, length: 1 },Snip { offset: 453, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 454, length: 1 },Snip { offset: 454, length: 1 }),
            ParserEvent::Char('z').localize(Snip { offset: 455, length: 1 },Snip { offset: 455, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 456, length: 1 },Snip { offset: 456, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 457, length: 1 },Snip { offset: 457, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 458, length: 1 },Snip { offset: 458, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 459, length: 1 },Snip { offset: 459, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 460, length: 1 },Snip { offset: 460, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 461, length: 1 },Snip { offset: 461, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 462, length: 1 },Snip { offset: 462, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 463, length: 1 },Snip { offset: 463, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 464, length: 1 },Snip { offset: 464, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 465, length: 1 },Snip { offset: 465, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 466, length: 1 },Snip { offset: 466, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 467, length: 1 },Snip { offset: 467, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 468, length: 1 },Snip { offset: 468, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 469, length: 1 },Snip { offset: 469, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 470, length: 1 },Snip { offset: 470, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 471, length: 1 },Snip { offset: 471, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 472, length: 1 },Snip { offset: 472, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 473, length: 1 },Snip { offset: 473, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 474, length: 1 },Snip { offset: 474, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 475, length: 1 },Snip { offset: 475, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 476, length: 1 },Snip { offset: 476, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 477, length: 1 },Snip { offset: 477, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 478, length: 1 },Snip { offset: 478, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 479, length: 1 },Snip { offset: 479, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 480, length: 1 },Snip { offset: 480, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 481, length: 1 },Snip { offset: 481, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 482, length: 1 },Snip { offset: 482, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 483, length: 1 },Snip { offset: 483, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 484, length: 1 },Snip { offset: 484, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 485, length: 1 },Snip { offset: 485, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 486, length: 1 },Snip { offset: 486, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 487, length: 1 },Snip { offset: 487, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 488, length: 1 },Snip { offset: 488, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 489, length: 1 },Snip { offset: 489, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 490, length: 1 },Snip { offset: 490, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 491, length: 1 },Snip { offset: 491, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 492, length: 1 },Snip { offset: 492, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 493, length: 1 },Snip { offset: 493, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 494, length: 1 },Snip { offset: 494, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 495, length: 1 },Snip { offset: 495, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 496, length: 1 },Snip { offset: 496, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 497, length: 1 },Snip { offset: 497, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 498, length: 1 },Snip { offset: 498, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 499, length: 1 },Snip { offset: 499, length: 1 }),
                end: ().localize(Snip { offset: 502, length: 1 },Snip { offset: 502, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 499, length: 1 },Snip { offset: 499, length: 1 }),
                    SourceEvent::Char('/').localize(Snip { offset: 500, length: 1 },Snip { offset: 500, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 501, length: 1 },Snip { offset: 501, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 502, length: 1 },Snip { offset: 502, length: 1 }),
                ],
            }).localize(Snip { offset: 499, length: 4 },Snip { offset: 499, length: 4 }),
        ].into_iter();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            /*if let ParserEvent::Parsed(tag) = local_event.data() {
                for lse in &tag.raw {
                    let (l,e) = lse.into_inner();
                    println!("SourceEvent::{:?}.localize({:?},{:?}),",e,l.chars(),l.bytes());
                }
                println!("");
            }*/
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
                    println!("Result: {:?}",ev);
                    /*if let ParserEvent::Parsed(tag) = local_event.data() {
                        for (_,attr) in &tag.attributes {
                            if let Some(attr) = attr {
                                println!("[");
                                for lse in attr {
                                    println!("SourceEvent::{:?}.localize({:?},{:?}),",lse.data(),lse.chars(),lse.bytes());
                                }
                                println!("]");
                            }
                        }
                        println!("begin: ().localize({:?},{:?}),",tag.begin.chars(),tag.begin.bytes());
                        println!("end: ().localize({:?},{:?}),",tag.end.chars(),tag.end.bytes());
                    }*/
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

    #[test]
    fn no_tag() {
        let mut src = "#include<iostream>\nusing namespace std;\nint main(){\ncout<<”Hello world!”<<endl;\nreturn 0;\n}\nOutput: Hello world!\n".into_source();
        let mut parser = Builder::new().create();

        let mut res_iter = [
            ParserEvent::Char('#').localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 1, length: 1 },Snip { offset: 1, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 2, length: 1 },Snip { offset: 2, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Other("iostream".to_string()), closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 8, length: 1 }, Snip { offset: 8, length: 1 }),
                end: ().localize(Snip { offset: 17, length: 1 }, Snip { offset: 17, length: 1 }),
                raw: vec![
                    SourceEvent::Char('<').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 13, length: 1 },Snip { offset: 13, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 14, length: 1 },Snip { offset: 14, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 15, length: 1 },Snip { offset: 15, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 16, length: 1 },Snip { offset: 16, length: 1 }),
                    SourceEvent::Char('>').localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
                ],
            }).localize(Snip { offset: 8, length: 10 },Snip { offset: 8, length: 10 }),
            ParserEvent::Char('\n').localize(Snip { offset: 18, length: 1 },Snip { offset: 18, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 19, length: 1 },Snip { offset: 19, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 20, length: 1 },Snip { offset: 20, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 22, length: 1 },Snip { offset: 22, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 23, length: 1 },Snip { offset: 23, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 24, length: 1 },Snip { offset: 24, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 25, length: 1 },Snip { offset: 25, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 26, length: 1 },Snip { offset: 26, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 27, length: 1 },Snip { offset: 27, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 28, length: 1 },Snip { offset: 28, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 29, length: 1 },Snip { offset: 29, length: 1 }),
            ParserEvent::Char('p').localize(Snip { offset: 30, length: 1 },Snip { offset: 30, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 31, length: 1 },Snip { offset: 31, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 32, length: 1 },Snip { offset: 32, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 33, length: 1 },Snip { offset: 33, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 34, length: 1 },Snip { offset: 34, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 35, length: 1 },Snip { offset: 35, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 36, length: 1 },Snip { offset: 36, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 37, length: 1 },Snip { offset: 37, length: 1 }),
            ParserEvent::Char(';').localize(Snip { offset: 38, length: 1 },Snip { offset: 38, length: 1 }),
            ParserEvent::Char('\n').localize(Snip { offset: 39, length: 1 },Snip { offset: 39, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 40, length: 1 },Snip { offset: 40, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 41, length: 1 },Snip { offset: 41, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 42, length: 1 },Snip { offset: 42, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 43, length: 1 },Snip { offset: 43, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 44, length: 1 },Snip { offset: 44, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 45, length: 1 },Snip { offset: 45, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 46, length: 1 },Snip { offset: 46, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 47, length: 1 },Snip { offset: 47, length: 1 }),
            ParserEvent::Char('(').localize(Snip { offset: 48, length: 1 },Snip { offset: 48, length: 1 }),
            ParserEvent::Char(')').localize(Snip { offset: 49, length: 1 },Snip { offset: 49, length: 1 }),
            ParserEvent::Char('{').localize(Snip { offset: 50, length: 1 },Snip { offset: 50, length: 1 }),
            ParserEvent::Char('\n').localize(Snip { offset: 51, length: 1 },Snip { offset: 51, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 52, length: 1 },Snip { offset: 52, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 53, length: 1 },Snip { offset: 53, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 54, length: 1 },Snip { offset: 54, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 55, length: 1 },Snip { offset: 55, length: 1 }),
            ParserEvent::Char('<').localize(Snip { offset: 56, length: 1 },Snip { offset: 56, length: 1 }),
            ParserEvent::Char('<').localize(Snip { offset: 57, length: 1 },Snip { offset: 57, length: 1 }),
            ParserEvent::Char('”').localize(Snip { offset: 58, length: 1 },Snip { offset: 58, length: 3 }),
            ParserEvent::Char('H').localize(Snip { offset: 59, length: 1 },Snip { offset: 61, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 60, length: 1 },Snip { offset: 62, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 61, length: 1 },Snip { offset: 63, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 62, length: 1 },Snip { offset: 64, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 63, length: 1 },Snip { offset: 65, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 64, length: 1 },Snip { offset: 66, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 65, length: 1 },Snip { offset: 67, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 66, length: 1 },Snip { offset: 68, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 67, length: 1 },Snip { offset: 69, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 68, length: 1 },Snip { offset: 70, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 69, length: 1 },Snip { offset: 71, length: 1 }),
            ParserEvent::Char('!').localize(Snip { offset: 70, length: 1 },Snip { offset: 72, length: 1 }),
            ParserEvent::Char('”').localize(Snip { offset: 71, length: 1 },Snip { offset: 73, length: 3 }),
            ParserEvent::Char('<').localize(Snip { offset: 72, length: 1 },Snip { offset: 76, length: 1 }),
        ].into_iter();

        let eof = vec![
            SourceEvent::Char('<').localize(Snip { offset: 73, length: 1 },Snip { offset: 77, length: 1 }),
            SourceEvent::Char('e').localize(Snip { offset: 74, length: 1 },Snip { offset: 78, length: 1 }),
            SourceEvent::Char('n').localize(Snip { offset: 75, length: 1 },Snip { offset: 79, length: 1 }),
            SourceEvent::Char('d').localize(Snip { offset: 76, length: 1 },Snip { offset: 80, length: 1 }),
            SourceEvent::Char('l').localize(Snip { offset: 77, length: 1 },Snip { offset: 81, length: 1 }),
            SourceEvent::Char(';').localize(Snip { offset: 78, length: 1 },Snip { offset: 82, length: 1 }),
            SourceEvent::Char('\n').localize(Snip { offset: 79, length: 1 },Snip { offset: 83, length: 1 }),
            SourceEvent::Char('r').localize(Snip { offset: 80, length: 1 },Snip { offset: 84, length: 1 }),
            SourceEvent::Char('e').localize(Snip { offset: 81, length: 1 },Snip { offset: 85, length: 1 }),
            SourceEvent::Char('t').localize(Snip { offset: 82, length: 1 },Snip { offset: 86, length: 1 }),
            SourceEvent::Char('u').localize(Snip { offset: 83, length: 1 },Snip { offset: 87, length: 1 }),
            SourceEvent::Char('r').localize(Snip { offset: 84, length: 1 },Snip { offset: 88, length: 1 }),
            SourceEvent::Char('n').localize(Snip { offset: 85, length: 1 },Snip { offset: 89, length: 1 }),
            SourceEvent::Char(' ').localize(Snip { offset: 86, length: 1 },Snip { offset: 90, length: 1 }),
            SourceEvent::Char('0').localize(Snip { offset: 87, length: 1 },Snip { offset: 91, length: 1 }),
            SourceEvent::Char(';').localize(Snip { offset: 88, length: 1 },Snip { offset: 92, length: 1 }),
            SourceEvent::Char('\n').localize(Snip { offset: 89, length: 1 },Snip { offset: 93, length: 1 }),
            SourceEvent::Char('}').localize(Snip { offset: 90, length: 1 },Snip { offset: 94, length: 1 }),
            SourceEvent::Char('\n').localize(Snip { offset: 91, length: 1 },Snip { offset: 95, length: 1 }),
            SourceEvent::Char('O').localize(Snip { offset: 92, length: 1 },Snip { offset: 96, length: 1 }),
            SourceEvent::Char('u').localize(Snip { offset: 93, length: 1 },Snip { offset: 97, length: 1 }),
            SourceEvent::Char('t').localize(Snip { offset: 94, length: 1 },Snip { offset: 98, length: 1 }),
            SourceEvent::Char('p').localize(Snip { offset: 95, length: 1 },Snip { offset: 99, length: 1 }),
            SourceEvent::Char('u').localize(Snip { offset: 96, length: 1 },Snip { offset: 100, length: 1 }),
            SourceEvent::Char('t').localize(Snip { offset: 97, length: 1 },Snip { offset: 101, length: 1 }),
            SourceEvent::Char(':').localize(Snip { offset: 98, length: 1 },Snip { offset: 102, length: 1 }),
            SourceEvent::Char(' ').localize(Snip { offset: 99, length: 1 },Snip { offset: 103, length: 1 }),
            SourceEvent::Char('H').localize(Snip { offset: 100, length: 1 },Snip { offset: 104, length: 1 }),
            SourceEvent::Char('e').localize(Snip { offset: 101, length: 1 },Snip { offset: 105, length: 1 }),
            SourceEvent::Char('l').localize(Snip { offset: 102, length: 1 },Snip { offset: 106, length: 1 }),
            SourceEvent::Char('l').localize(Snip { offset: 103, length: 1 },Snip { offset: 107, length: 1 }),
            SourceEvent::Char('o').localize(Snip { offset: 104, length: 1 },Snip { offset: 108, length: 1 }),
            SourceEvent::Char(' ').localize(Snip { offset: 105, length: 1 },Snip { offset: 109, length: 1 }),
            SourceEvent::Char('w').localize(Snip { offset: 106, length: 1 },Snip { offset: 110, length: 1 }),
            SourceEvent::Char('o').localize(Snip { offset: 107, length: 1 },Snip { offset: 111, length: 1 }),
            SourceEvent::Char('r').localize(Snip { offset: 108, length: 1 },Snip { offset: 112, length: 1 }),
            SourceEvent::Char('l').localize(Snip { offset: 109, length: 1 },Snip { offset: 113, length: 1 }),
            SourceEvent::Char('d').localize(Snip { offset: 110, length: 1 },Snip { offset: 114, length: 1 }),
            SourceEvent::Char('!').localize(Snip { offset: 111, length: 1 },Snip { offset: 115, length: 1 }),
            SourceEvent::Char('\n').localize(Snip { offset: 112, length: 1 },Snip { offset: 116, length: 1 }),
        ];
         
        while let Some(local_event) = match parser.next_event(&mut src) {
            Ok(ope) => ope,
            Err(e) => match e {
                Error::EofInTag(raw) => {
                    match raw.len() != eof.len() {
                        true => panic!("parser and eof_result differs in size"),
                        false => for (d,e) in raw.into_iter().zip(eof.iter()) {
                            println!("Parser: {:?}",d);
                            println!("Result: {:?}",e);
                            assert_eq!(d,*e);
                        }
                    }
                    
                    None
                },
                Error::EndBeforeBegin => panic!("{:?}",e),
                Error::NoBegin => panic!("{:?}",e),
            },
        } {
            /*if let ParserEvent::Parsed(tag) = local_event.data() {
                for lse in &tag.raw {
                    let (l,e) = lse.into_inner();
                    println!("SourceEvent::{:?}.localize({:?},{:?}),",e,l.chars(),l.bytes());
                }
                println!("");
            }*/
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
                    println!("Result: {:?}",ev);
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

    #[test]
    fn auto_detect_01() {
        let mut src = "#include<iostream>\nusing namespace std;\nint main(){\ncout<<”Hello world!”<<endl;\nreturn 0;\n}\nOutput: Hello world!\n".into_source();
        let mut parser = Builder::auto_detect().create();

        let mut res_iter = [
            ParserEvent::Char('#').localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 1, length: 1 },Snip { offset: 1, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 2, length: 1 },Snip { offset: 2, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Char('<').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 13, length: 1 },Snip { offset: 13, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 14, length: 1 },Snip { offset: 14, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 15, length: 1 },Snip { offset: 15, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 16, length: 1 },Snip { offset: 16, length: 1 }),
            ParserEvent::Char('>').localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
            ParserEvent::Char('\n').localize(Snip { offset: 18, length: 1 },Snip { offset: 18, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 19, length: 1 },Snip { offset: 19, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 20, length: 1 },Snip { offset: 20, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 22, length: 1 },Snip { offset: 22, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 23, length: 1 },Snip { offset: 23, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 24, length: 1 },Snip { offset: 24, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 25, length: 1 },Snip { offset: 25, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 26, length: 1 },Snip { offset: 26, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 27, length: 1 },Snip { offset: 27, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 28, length: 1 },Snip { offset: 28, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 29, length: 1 },Snip { offset: 29, length: 1 }),
            ParserEvent::Char('p').localize(Snip { offset: 30, length: 1 },Snip { offset: 30, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 31, length: 1 },Snip { offset: 31, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 32, length: 1 },Snip { offset: 32, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 33, length: 1 },Snip { offset: 33, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 34, length: 1 },Snip { offset: 34, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 35, length: 1 },Snip { offset: 35, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 36, length: 1 },Snip { offset: 36, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 37, length: 1 },Snip { offset: 37, length: 1 }),
            ParserEvent::Char(';').localize(Snip { offset: 38, length: 1 },Snip { offset: 38, length: 1 }),
            ParserEvent::Char('\n').localize(Snip { offset: 39, length: 1 },Snip { offset: 39, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 40, length: 1 },Snip { offset: 40, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 41, length: 1 },Snip { offset: 41, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 42, length: 1 },Snip { offset: 42, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 43, length: 1 },Snip { offset: 43, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 44, length: 1 },Snip { offset: 44, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 45, length: 1 },Snip { offset: 45, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 46, length: 1 },Snip { offset: 46, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 47, length: 1 },Snip { offset: 47, length: 1 }),
            ParserEvent::Char('(').localize(Snip { offset: 48, length: 1 },Snip { offset: 48, length: 1 }),
            ParserEvent::Char(')').localize(Snip { offset: 49, length: 1 },Snip { offset: 49, length: 1 }),
            ParserEvent::Char('{').localize(Snip { offset: 50, length: 1 },Snip { offset: 50, length: 1 }),
            ParserEvent::Char('\n').localize(Snip { offset: 51, length: 1 },Snip { offset: 51, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 52, length: 1 },Snip { offset: 52, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 53, length: 1 },Snip { offset: 53, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 54, length: 1 },Snip { offset: 54, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 55, length: 1 },Snip { offset: 55, length: 1 }),
            ParserEvent::Char('<').localize(Snip { offset: 56, length: 1 },Snip { offset: 56, length: 1 }),
            ParserEvent::Char('<').localize(Snip { offset: 57, length: 1 },Snip { offset: 57, length: 1 }),
            ParserEvent::Char('”').localize(Snip { offset: 58, length: 1 },Snip { offset: 58, length: 3 }),
            ParserEvent::Char('H').localize(Snip { offset: 59, length: 1 },Snip { offset: 61, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 60, length: 1 },Snip { offset: 62, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 61, length: 1 },Snip { offset: 63, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 62, length: 1 },Snip { offset: 64, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 63, length: 1 },Snip { offset: 65, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 64, length: 1 },Snip { offset: 66, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 65, length: 1 },Snip { offset: 67, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 66, length: 1 },Snip { offset: 68, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 67, length: 1 },Snip { offset: 69, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 68, length: 1 },Snip { offset: 70, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 69, length: 1 },Snip { offset: 71, length: 1 }),
            ParserEvent::Char('!').localize(Snip { offset: 70, length: 1 },Snip { offset: 72, length: 1 }),
            ParserEvent::Char('”').localize(Snip { offset: 71, length: 1 },Snip { offset: 73, length: 3 }),
            ParserEvent::Char('<').localize(Snip { offset: 72, length: 1 },Snip { offset: 76, length: 1 }),
            ParserEvent::Char('<').localize(Snip { offset: 73, length: 1 },Snip { offset: 77, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 74, length: 1 },Snip { offset: 78, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 75, length: 1 },Snip { offset: 79, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 76, length: 1 },Snip { offset: 80, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 77, length: 1 },Snip { offset: 81, length: 1 }),
            ParserEvent::Char(';').localize(Snip { offset: 78, length: 1 },Snip { offset: 82, length: 1 }),
            ParserEvent::Char('\n').localize(Snip { offset: 79, length: 1 },Snip { offset: 83, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 80, length: 1 },Snip { offset: 84, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 81, length: 1 },Snip { offset: 85, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 82, length: 1 },Snip { offset: 86, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 83, length: 1 },Snip { offset: 87, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 84, length: 1 },Snip { offset: 88, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 85, length: 1 },Snip { offset: 89, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 86, length: 1 },Snip { offset: 90, length: 1 }),
            ParserEvent::Char('0').localize(Snip { offset: 87, length: 1 },Snip { offset: 91, length: 1 }),
            ParserEvent::Char(';').localize(Snip { offset: 88, length: 1 },Snip { offset: 92, length: 1 }),
            ParserEvent::Char('\n').localize(Snip { offset: 89, length: 1 },Snip { offset: 93, length: 1 }),
            ParserEvent::Char('}').localize(Snip { offset: 90, length: 1 },Snip { offset: 94, length: 1 }),
            ParserEvent::Char('\n').localize(Snip { offset: 91, length: 1 },Snip { offset: 95, length: 1 }),
            ParserEvent::Char('O').localize(Snip { offset: 92, length: 1 },Snip { offset: 96, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 93, length: 1 },Snip { offset: 97, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 94, length: 1 },Snip { offset: 98, length: 1 }),
            ParserEvent::Char('p').localize(Snip { offset: 95, length: 1 },Snip { offset: 99, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 96, length: 1 },Snip { offset: 100, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 97, length: 1 },Snip { offset: 101, length: 1 }),
            ParserEvent::Char(':').localize(Snip { offset: 98, length: 1 },Snip { offset: 102, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 99, length: 1 },Snip { offset: 103, length: 1 }),
            ParserEvent::Char('H').localize(Snip { offset: 100, length: 1 },Snip { offset: 104, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 101, length: 1 },Snip { offset: 105, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 102, length: 1 },Snip { offset: 106, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 103, length: 1 },Snip { offset: 107, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 104, length: 1 },Snip { offset: 108, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 105, length: 1 },Snip { offset: 109, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 106, length: 1 },Snip { offset: 110, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 107, length: 1 },Snip { offset: 111, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 108, length: 1 },Snip { offset: 112, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 109, length: 1 },Snip { offset: 113, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 110, length: 1 },Snip { offset: 114, length: 1 }),
            ParserEvent::Char('!').localize(Snip { offset: 111, length: 1 },Snip { offset: 115, length: 1 }),
            ParserEvent::Char('\n').localize(Snip { offset: 112, length: 1 },Snip { offset: 116, length: 1 }),
        ].into_iter();
         
        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            /*if let ParserEvent::Parsed(tag) = local_event.data() {
                for lse in &tag.raw {
                    let (l,e) = lse.into_inner();
                    println!("SourceEvent::{:?}.localize({:?},{:?}),",e,l.chars(),l.bytes());
                }
                println!("");
            }*/
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
                    println!("Result: {:?}",ev);
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }
    
    /*#[test]
    fn basic_pipe() {
        let mut src = "<h1>Hello, world!</h1>Привет, мир, &#x2<wbr>764;!"
            .into_source()
            .pipe(Builder::new().create().piped(|t: Tag| {
                // skip all tags
                Some(SourceEvent::Breaker(Breaker::Word))
            }));

        while let Some(local_se) = src.next_char().unwrap() {
            println!("{:?}",local_se);
        }
        panic!();
    }

    #[test]
    fn basic_pipe_ent() {
        let mut src = "<h1>Hello, world!</h1>Привет, мир, &#x2<wbr>764;!"
            .into_source()
            .pipe(crate::tagger::Builder::new().create().piped(|t: Tag| {
                Some(match t.name {
                    TagName::Wbr => SourceEvent::Breaker(Breaker::None),
                    _ => SourceEvent::Breaker(Breaker::Word),
                })
            }))
            .pipe(crate::entities::Builder::new().create().into_piped());

        while let Some(local_se) = src.next_char().unwrap() {
            println!("{:?}",local_se);
        }
        panic!();
    }
     */
        
}
