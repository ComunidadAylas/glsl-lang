use thiserror::Error;

use lang_util::{
    located::{Located, LocatedBuilder},
    FileId,
};

use crate::{last::LocatedIterator, parser, types::path::ParsedPath};

use super::{
    event::Event,
    expand::{ExpandEvent, ExpandOne},
    ProcessorState,
};

pub fn parse(input: &str) -> parser::Ast {
    parser::Parser::new(input).parse()
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum ProcessStrError {
    #[error("an import was requested without a filesystem context")]
    ImportRequested(ParsedPath),
}

pub fn process(input: &str, state: ProcessorState) -> ExpandStr {
    let file_id = FileId::new(0);
    let ast = parser::Parser::new(input).parse();
    ExpandStr {
        inner: ExpandOne::new((file_id, ast), state),
        final_state: None,
    }
}

pub struct ExpandStr {
    inner: ExpandOne,
    final_state: Option<ProcessorState>,
}

impl ExpandStr {
    pub fn tokenize(
        self,
        current_version: u16,
        target_vulkan: bool,
        registry: &crate::exts::Registry,
    ) -> crate::last::Tokenizer<'_, Self> {
        crate::last::Tokenizer::new(self, current_version, target_vulkan, registry)
    }

    pub fn into_state(mut self) -> Option<ProcessorState> {
        self.final_state.take()
    }
}

impl Iterator for ExpandStr {
    type Item = Result<Event, Located<ProcessStrError>>;

    fn next(&mut self) -> Option<Self::Item> {
        let event = self.inner.next()?;
        match event {
            ExpandEvent::Event(event) => Some(Ok(event)),
            ExpandEvent::EnterFile(node, path) => Some(Err(LocatedBuilder::new()
                .pos(node.text_range())
                .resolve_file(self.inner.location())
                .finish(ProcessStrError::ImportRequested(path)))),
            ExpandEvent::Completed(state) => {
                self.final_state = Some(state);
                None
            }
        }
    }
}

impl LocatedIterator for ExpandStr {
    fn location(&self) -> &crate::processor::expand::ExpandLocation {
        self.inner.location()
    }
}

#[cfg(test)]
mod tests {
    fn assert_send<T: Send>() {}

    #[test]
    fn test_error_send() {
        assert_send::<super::ProcessStrError>();
    }
}
