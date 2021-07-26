use std::{array::IntoIter, collections::HashMap, iter::FromIterator, rc::Rc};

use smol_str::SmolStr;

use lang_util::FileId;

#[macro_use]
pub mod exts;

mod definition;
use definition::Definition;

pub mod event;

pub mod expand;

pub mod fs;

pub mod nodes;
use nodes::{
    Define, DefineObject, Extension, ExtensionName, Version, GL_ARB_SHADING_LANGUAGE_INCLUDE,
    GL_GOOGLE_CPP_STYLE_LINE_DIRECTIVE, GL_GOOGLE_INCLUDE_DIRECTIVE,
};

/// Operating mode for #include directives
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeMode {
    /// No #include directives are allowed
    None,
    /// GL_ARB_shading_language_include runtime includes
    ArbInclude,
    /// GL_GOOGLE_include_directive compile-time includes
    GoogleInclude,
}

impl Default for IncludeMode {
    fn default() -> Self {
        Self::None
    }
}

/// Current state of the preprocessor
#[derive(Debug, Clone)]
pub struct ProcessorState {
    extension_stack: Vec<Extension>,
    include_mode: IncludeMode,
    // use Rc to make cloning the whole struct cheaper
    definitions: HashMap<SmolStr, Definition>,
    version: Version,
    cpp_style_line: bool,
}

impl ProcessorState {
    pub fn extension(&mut self, extension: &nodes::Extension) {
        // Push onto the stack
        self.extension_stack.push(extension.clone());

        // Process include extensions
        let target_include_mode = if extension.name == *GL_ARB_SHADING_LANGUAGE_INCLUDE {
            Some(IncludeMode::ArbInclude)
        } else if extension.name == *GL_GOOGLE_INCLUDE_DIRECTIVE {
            Some(IncludeMode::GoogleInclude)
        } else {
            None
        };

        if let Some(target) = target_include_mode {
            if extension.behavior.is_active() {
                self.include_mode = target;

                // GL_GOOGLE_include_directive enable GL_GOOGLE_cpp_style_line
                if target == IncludeMode::GoogleInclude {
                    self.cpp_style_line = true;
                }
            } else {
                // TODO: Implement current mode as a stack?
                self.include_mode = IncludeMode::None;
            }
        }

        // Process others
        if extension.name == *GL_GOOGLE_CPP_STYLE_LINE_DIRECTIVE {
            if extension.behavior.is_active() {
                self.cpp_style_line = true;
            } else {
                // TODO: Notify instead of silently ignoring?
                if self.include_mode != IncludeMode::GoogleInclude {
                    self.cpp_style_line = false;
                }
            }
        }
    }

    pub fn cpp_style_line(&self) -> bool {
        self.cpp_style_line
    }
}

impl Default for ProcessorState {
    fn default() -> Self {
        Self {
            // Spec 3.3, "The initial state of the compiler is as if the directive
            // `#extension all : disable` was issued
            extension_stack: vec![Extension::disable(ExtensionName::All)],
            // No #include extensions enabled
            include_mode: IncludeMode::None,
            // Spec 3.3, "There is a built-in macro definition for each profile the implementation
            // supports. All implementations provide the following macro:
            // `#define GL_core_profile 1`
            definitions: HashMap::from_iter(
                IntoIter::new([
                    Definition::Regular(
                        Rc::new(Define::object(
                            "GL_core_profile".into(),
                            DefineObject::from_str("1").unwrap(),
                            true,
                        )),
                        FileId::default(),
                    ),
                    Definition::Line,
                    Definition::File,
                    Definition::Version,
                ])
                .map(|definition| (definition.name().into(), definition)),
            ),
            version: Version::default(),
            cpp_style_line: false,
        }
    }
}
