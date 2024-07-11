use super::{
    entities::{Entity,Instance},
    state::EntityState,
};
use crate::{
    ParserResult,
    Source, SourceEvent, ParserEvent, SourceResult, Local,
    Parser, Runtime, PipeParser, IntoPipeParser,
};

/*

  Entities: https://www.w3.org/TR/html5/entities.json


    Semicolon may be optional (html legacy)

    &lt;   &#60;  &#x3C;

    "&" + Name + ";"  

    // without parametr entities [ %name; ]  ????

	NameStartChar	   ::=   	":" | [A-Z] | "_" | [a-z] | [#xC0-#xD6] | [#xD8-#xF6] | [#xF8-#x2FF] | [#x370-#x37D] | [#x37F-#x1FFF] | [#x200C-#x200D] | [#x2070-#x218F] | [#x2C00-#x2FEF] | [#x3001-#xD7FF] | [#xF900-#xFDCF] | [#xFDF0-#xFFFD] | [#x10000-#xEFFFF]
   	NameChar	   ::=   	NameStartChar | "-" | "." | [0-9] | #xB7 | [#x0300-#x036F] | [#x203F-#x2040]
   	Name	   ::=   	NameStartChar (NameChar)*

*/



#[derive(Debug,Clone)]
pub struct Builder {

}
impl Builder {
    pub fn new() -> Builder {
        Builder { }
    }
    pub fn create(self) -> EntityParser {
        EntityParser(Runtime::new(()))
    }
}

pub struct EntityParser(Runtime<EntityState,Entity,()>);
impl Parser for EntityParser {
    type Data = Entity;
    
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Entity> {
        self.0.next_event(src)
    }
}

impl IntoPipeParser for EntityParser {
    type Piped = PipedEntityParser;
    
    fn into_piped(self) -> Self::Piped {
        PipedEntityParser {
            parser: self.0,
            tmp: None,
        }
    }
}

pub struct PipedEntityParser {
    parser: Runtime<EntityState,Entity,()>,
    tmp: Option<Local<SourceEvent>>,
}

impl PipeParser for PipedEntityParser {
    fn next_char<S: Source>(&mut self, src: &mut S) -> SourceResult {
        Ok(match self.tmp.take() {
            Some(local_se) => Some(local_se), 
            None => match self.parser.next_event(src)? {
                Some(local_ent) => {
                    let (local,ent) = local_ent.into_inner();
                    match ent {
                        ParserEvent::Char(c) => Some(local.local(SourceEvent::Char(c))),
                        ParserEvent::Breaker(b) => Some(local.local(SourceEvent::Breaker(b))),
                        ParserEvent::Parsed(ent) => match ent.entity {
                            Instance::Char(c) => Some(local.local(SourceEvent::Char(c))),
                            Instance::Char2(c1,c2) => {
                                self.tmp = Some(local.local(SourceEvent::Char(c2)));
                                Some(local.local(SourceEvent::Char(c1)))
                            },
                        },
                    }
                },
                None => None,
            },
        })
    }
}


#[cfg(test)]
mod tests {
    use crate::*;
    use super::*;
    
    /*#[test]
    fn basic() {
        let mut src = " &blabla; &#111111111; &quot &AMP; &&GreaterGreater; &#128175; &#x2764;".into_source();
        let mut parser = Builder::new().create();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            println!("{:?}",local_event);
        }
        panic!();
    }

    #[test]
    fn basic_piped() {
        let mut src = " &blabla; &#111111111; &quot &AMP; &&GreaterGreater; &#128175; &#x2764;".into_source();
        let mut parser = Builder::new().create().into_piped();

        while let Some(local_se) = parser.next_char(&mut src).unwrap() {
            println!("{:?}",local_se);
        }
        panic!();
    }

    #[test]
    fn basic_piped_2() {        
        let mut src = " &blabla; &#111111111; &quot &AMP; &&GreaterGreater; &#128175; &#x2764;"
            .into_source()
            .pipe(Builder::new().create().into_piped());

        while let Some(local_se) = src.next_char().unwrap() {
            println!("{:?}",local_se);
        }
        panic!();
    }
    
    #[test]
    fn basic_piped_3() {        
        let mut src = " &blabla; &#111111111; &quot &AMP; &&GreaterGreater; &#128175; &#x2764;"
            .into_source()
            .pipe(Builder::new().create().piped(|_| None));

        while let Some(local_se) = src.next_char().unwrap() {
            println!("{:?}",local_se);
        }
        panic!();
    }
*/
}

