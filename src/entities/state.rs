
use super::{
    entities::{Entity,Instance,ENTITIES},     
};
use crate::{
    Error, Local, ParserEvent,
    NextState, InnerState, ParserState,
    SourceEvent, Breaker,
};


#[derive(Debug)]
pub(in super) enum EntityState {
    // Entities
    Init,
    MayBeEntity(Local<char>),
    MayBeNumEntity(Local<char>,Local<char>),
    EntityNamed(ReadEntity),
    EntityNumber(ReadEntity),
    EntityNumberX(ReadEntity),
}
impl Default for EntityState {
    fn default() -> EntityState {
        EntityState::Init
    }
}

#[derive(Debug)]
pub(in super) struct ReadEntity {
    begin: Local<char>,
    current: Local<char>,
    content: String,
    chars: Vec<Local<char>>,
}
impl ReadEntity {
    fn named_into_state(self) -> NextState<EntityState,Entity> {
        let mut ns = InnerState::empty();
        match ENTITIES.get(&self.content) {
            Some(e) => ns = ns.with_event(create_entity_event(self,e)?),
            None => for c in self.chars {
                ns = ns.with_event(c.map(|c| ParserEvent::Char(c)));
            },
        }
        Ok(ns)
    }
    fn number_into_state(self) -> NextState<EntityState,Entity> {
        let mut ns = InnerState::empty();
        match match u32::from_str_radix(&self.content,10) {
            Ok(u) => char::from_u32(u),
            Err(_) => None,
        } {
            Some(e) => ns = ns.with_event(create_entity_event(self,Instance::Char(e))?),
            None => for c in self.chars {
                ns = ns.with_event(c.map(|c| ParserEvent::Char(c)));
            },
        }
        Ok(ns)
    }
    fn number_x_into_state(self) -> NextState<EntityState,Entity> {
        let mut ns = InnerState::empty();
        match match u32::from_str_radix(&self.content,16) {
            Ok(u) => char::from_u32(u),
            Err(_) => None,
        } {
            Some(e) => ns = ns.with_event(create_entity_event(self,Instance::Char(e))?),
            None => for c in self.chars {
                ns = ns.with_event(c.map(|c| ParserEvent::Char(c)));
            },
        }
        Ok(ns)
    }
    fn failed_into_state(self) -> NextState<EntityState,Entity> {
        let mut ns = InnerState::empty();
        for c in self.chars {
            ns = ns.with_event(c.map(|c| ParserEvent::Char(c)));
        }
        Ok(ns)
    }
}

impl ParserState for EntityState {
    type Context = ();
    type Data = Entity;
    
    fn eof(self, _: &Self::Context) -> NextState<EntityState,Entity> {        
        Ok(match self {
            EntityState::Init => InnerState::empty(),
            EntityState::MayBeEntity(amp_char) => InnerState::empty().with_event(amp_char.map(|c| ParserEvent::Char(c))),
            EntityState::MayBeNumEntity(amp_char,hash_char) => {
                InnerState::empty()
                    .with_event(amp_char.map(|c| ParserEvent::Char(c)))
                    .with_event(hash_char.map(|c| ParserEvent::Char(c)))
            },
            EntityState::EntityNamed(ent) => ent.named_into_state()?,
            EntityState::EntityNumber(ent) |
            EntityState::EntityNumberX(ent) => ent.failed_into_state()?,
        })
    }
    fn next_state(self, local_src: Local<SourceEvent>,  _: &Self::Context) -> NextState<EntityState,Entity> {
        match self {
            EntityState::Init => init(local_src),
            EntityState::MayBeEntity(amp_char) => may_be_entity(amp_char,local_src),
            EntityState::MayBeNumEntity(amp_char,hash_char) => may_be_num_entity(amp_char,hash_char,local_src),
            EntityState::EntityNamed(ent) => entity_named(ent,local_src),
            EntityState::EntityNumber(ent) => entity_number(ent,local_src),
            EntityState::EntityNumberX(ent) => entity_number_x(ent,local_src),
        }
    }
}

fn init(local_src: Local<SourceEvent>) -> NextState<EntityState,Entity> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            match lc {
                '&' => InnerState::empty()
                    .with_state(EntityState::MayBeEntity(local_char)),
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

fn create_entity_event(entity: ReadEntity, replace: Instance) -> Result<Local<ParserEvent<Entity>>,Error> {    
    Local::from_segment(entity.begin,entity.current)
        .map(|local| {
            let mut v = String::with_capacity(entity.chars.len());
            for c in entity.chars {
                v.push(*c.data());
            }
            local.with_inner(ParserEvent::Parsed(Entity{ value: v, entity: replace }))
        })
}

fn entity_number_x(mut ent: ReadEntity, local_src: Local<SourceEvent>) -> NextState<EntityState,Entity> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            match lc {
                '0' ..= '9' | 'a' ..= 'z' | 'A' ..= 'Z' => {
                    ent.current = local_char;
                    ent.content.push(*local_char.data());
                    ent.chars.push(local_char);
                    InnerState::empty().with_state(EntityState::EntityNumberX(ent))
                },
                ';' => {
                    ent.current = local_char;
                    ent.chars.push(local_char);
                    ent.number_x_into_state()?
                },
                '&'=> ent.failed_into_state()?.with_state(EntityState::MayBeEntity(local_char)),
                _ => ent.failed_into_state()?.with_event(local_char.map(|c| ParserEvent::Char(c))),
            }
        },
        SourceEvent::Breaker(b) => match b {
            Breaker::None => InnerState::empty().with_state(EntityState::EntityNumberX(ent)),
            _ => ent.failed_into_state()?
                .with_event(local_src.local(ParserEvent::Breaker(b))),
        },
    })
}

fn entity_number(mut ent: ReadEntity, local_src: Local<SourceEvent>) -> NextState<EntityState,Entity> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            match lc {
                '0' ..= '9' => {
                    ent.current = local_char;
                    ent.content.push(*local_char.data());
                    ent.chars.push(local_char);
                    InnerState::empty().with_state(EntityState::EntityNumber(ent))
                },
                ';' => {
                    ent.current = local_char;
                    ent.chars.push(local_char);
                    ent.number_into_state()?                
                },
                '&'=> ent.failed_into_state()?.with_state(EntityState::MayBeEntity(local_char)),
                _ => ent.failed_into_state()?.with_event(local_char.map(|c| ParserEvent::Char(c))),
            }
        },
        SourceEvent::Breaker(b) => match b {
            Breaker::None => InnerState::empty().with_state(EntityState::EntityNumber(ent)),
            _ => ent.failed_into_state()?
                .with_event(local_src.local(ParserEvent::Breaker(b))),
        },
    })
}

fn may_be_num_entity(amp_char: Local<char>, hash_char: Local<char>, local_src: Local<SourceEvent>) -> NextState<EntityState,Entity> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            match lc {
                'x' => {
                    let ent = ReadEntity {
                        begin: amp_char,
                        current: local_char,
                        content: String::new(),
                        chars: vec![amp_char,hash_char,local_char],
                    };
                    InnerState::empty()
                        .with_state(EntityState::EntityNumberX(ent))
                },
                '0' ..= '9' => {
                    let ent = ReadEntity {
                        begin: amp_char,
                        current: local_char,
                        content: { let mut s = String::new(); s.push(*local_char.data()); s },
                        chars: vec![amp_char,hash_char,local_char],
                    };
                    InnerState::empty()
                        .with_state(EntityState::EntityNumber(ent))
                },
                '&'=> InnerState::empty()
                    .with_state(EntityState::MayBeEntity(local_char))
                    .with_event(amp_char.map(|c| ParserEvent::Char(c)))
                    .with_event(hash_char.map(|c| ParserEvent::Char(c))),
                _ => InnerState::empty()
                    .with_event(amp_char.map(|c| ParserEvent::Char(c)))
                    .with_event(hash_char.map(|c| ParserEvent::Char(c)))
                    .with_event(local_char.map(|c| ParserEvent::Char(c))),
            }
        },
        SourceEvent::Breaker(b) => match b {
            Breaker::None => InnerState::empty().with_state(EntityState::MayBeNumEntity(amp_char,hash_char)),
            _ => InnerState::empty()
                .with_event(amp_char.map(|c| ParserEvent::Char(c)))
                .with_event(hash_char.map(|c| ParserEvent::Char(c)))
                .with_event(local_src.local(ParserEvent::Breaker(b))),
        },
    })
}

fn entity_named(mut ent: ReadEntity, local_src: Local<SourceEvent>) -> NextState<EntityState,Entity> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            match lc {
                ':' | '_' | 'A' ..= 'Z' | 'a' ..= 'z' | '\u{C0}' ..= '\u{D6}' | '\u{D8}' ..= '\u{F6}' | '\u{F8}' ..= '\u{2FF}' |
                '\u{370}' ..= '\u{37D}' | '\u{37F}' ..= '\u{1FFF}' | '\u{200C}' ..= '\u{200D}' | '\u{2070}' ..= '\u{218F}' |
                '\u{2C00}' ..= '\u{2FEF}' | '\u{3001}' ..= '\u{D7FF}' | '\u{F900}' ..= '\u{FDCF}' | '\u{FDF0}' ..= '\u{FFFD}' |
                '\u{10000}' ..= '\u{EFFFF}' |
                '-' | '.' | '0' ..= '9' | '\u{B7}' | '\u{0300}' ..= '\u{036F}' | '\u{203F}' ..= '\u{2040}' => {
                    ent.current = local_char;
                    ent.content.push(*local_char.data());
                    ent.chars.push(local_char);
                    InnerState::empty().with_state(EntityState::EntityNamed(ent))
                },
                ';' => {
                    ent.current = local_char;
                    ent.content.push(*local_char.data());
                    ent.chars.push(local_char);
                    ent.named_into_state()?                
                },
                '&'=> ent.named_into_state()?.with_state(EntityState::MayBeEntity(local_char)),
                _ => ent.named_into_state()?.with_event(local_char.map(|c| ParserEvent::Char(c))),
            }
        },
        SourceEvent::Breaker(b) => match b {
            Breaker::None => InnerState::empty().with_state(EntityState::EntityNamed(ent)),
            _ => ent.named_into_state()?
                .with_event(local_src.local(ParserEvent::Breaker(b))),
        },
    })
}

fn may_be_entity(amp_char: Local<char>, local_src: Local<SourceEvent>) -> NextState<EntityState,Entity> {
    Ok(match *local_src.data() {
        SourceEvent::Char(lc) => {
            let local_char = local_src.local(lc);
            match lc {
                '#' => InnerState::empty().with_state(EntityState::MayBeNumEntity(amp_char,local_char)),
                ':' | '_' | 'A' ..= 'Z' | 'a' ..= 'z' | '\u{C0}' ..= '\u{D6}' | '\u{D8}' ..= '\u{F6}' | '\u{F8}' ..= '\u{2FF}' |
                '\u{370}' ..= '\u{37D}' | '\u{37F}' ..= '\u{1FFF}' | '\u{200C}' ..= '\u{200D}' | '\u{2070}' ..= '\u{218F}' |
                '\u{2C00}' ..= '\u{2FEF}' | '\u{3001}' ..= '\u{D7FF}' | '\u{F900}' ..= '\u{FDCF}' | '\u{FDF0}' ..= '\u{FFFD}' |
                '\u{10000}' ..= '\u{EFFFF}' => {
                    let ent = ReadEntity {
                        begin: amp_char,
                        current: local_char,
                        content: {
                            let mut s = String::new();
                            s.push(*amp_char.data());
                            s.push(*local_char.data());
                            s
                        },
                        chars: vec![amp_char,local_char],
                    };
                    InnerState::empty().with_state(EntityState::EntityNamed(ent))
                }
                '&' => InnerState::empty()
                    .with_state(EntityState::MayBeEntity(local_char))
                    .with_event(amp_char.map(|c| ParserEvent::Char(c))),
                _ => InnerState::empty()
                    .with_event(amp_char.map(|c| ParserEvent::Char(c)))
                    .with_event(local_char.map(|c| ParserEvent::Char(c))),
            }
        },
        SourceEvent::Breaker(b) => match b {
            Breaker::None => InnerState::empty().with_state(EntityState::MayBeEntity(amp_char)),
            _ => InnerState::empty()
                .with_event(amp_char.map(|c| ParserEvent::Char(c)))
                .with_event(local_src.local(ParserEvent::Breaker(b))),
        },
    })
}
