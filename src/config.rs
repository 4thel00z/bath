use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathEntry {
    pub path: String,
    pub program: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VarKind {
    Scalar,
    List,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomVarDef {
    pub name: String,
    pub kind: VarKind,
    /// Separator between parts when `kind` is `List`.
    /// For `Scalar`, this is ignored and may be empty.
    pub separator: String,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Entry {
    // Paths & Programs
    Path(PathEntry),
    CPath(String),

    // Include paths
    CInclude(String),
    CPlusInclude(String),
    OBJCInclude(String),

    // Compiler flags
    CPPFlag(String),
    CFlag(String),
    CXXFlag(String),

    // Linker flags
    LDFlag(String),

    // Library paths
    LibraryPath(String),
    LDLibraryPath(String),
    LDRunPath(String),

    // Toolchain settings
    RanLib(String),
    CC(String),
    CXX(String),
    AR(String),
    Strip(String),
    GCCExecPrefix(String),
    CollectGCCOptions(String),
    Lang(String),

    // Custom env vars
    //
    // - CustomScalar is a single value (no separator semantics)
    // - CustomPart is a single part of a list-like variable (joined by `separator` at export time)
    CustomScalar {
        name: String,
        value: String,
    },
    CustomPart {
        name: String,
        value: String,
        separator: String,
    },
}

impl Entry {
    /// Returns the corresponding environment variable name.
    pub fn var_name(&self) -> Cow<'static, str> {
        match self {
            Entry::Path(_) => Cow::Borrowed("PATH"),
            Entry::CPath(_) => Cow::Borrowed("CPATH"),
            Entry::CInclude(_) => Cow::Borrowed("C_INCLUDE_PATH"),
            Entry::CPlusInclude(_) => Cow::Borrowed("CPLUS_INCLUDE_PATH"),
            Entry::OBJCInclude(_) => Cow::Borrowed("OBJC_INCLUDE_PATH"),
            Entry::CPPFlag(_) => Cow::Borrowed("CPPFLAGS"),
            Entry::CFlag(_) => Cow::Borrowed("CFLAGS"),
            Entry::CXXFlag(_) => Cow::Borrowed("CXXFLAGS"),
            Entry::LDFlag(_) => Cow::Borrowed("LDFLAGS"),
            Entry::LibraryPath(_) => Cow::Borrowed("LIBRARY_PATH"),
            Entry::LDLibraryPath(_) => Cow::Borrowed("LD_LIBRARY_PATH"),
            Entry::LDRunPath(_) => Cow::Borrowed("LD_RUN_PATH"),
            Entry::RanLib(_) => Cow::Borrowed("RANLIB"),
            Entry::CC(_) => Cow::Borrowed("CC"),
            Entry::CXX(_) => Cow::Borrowed("CXX"),
            Entry::AR(_) => Cow::Borrowed("AR"),
            Entry::Strip(_) => Cow::Borrowed("STRIP"),
            Entry::GCCExecPrefix(_) => Cow::Borrowed("GCC_EXEC_PREFIX"),
            Entry::CollectGCCOptions(_) => Cow::Borrowed("COLLECT_GCC_OPTIONS"),
            Entry::Lang(_) => Cow::Borrowed("LANG"),

            Entry::CustomScalar { name, .. } | Entry::CustomPart { name, .. } => {
                Cow::Owned(name.clone())
            }
        }
    }

    /// Returns the default separator used when joining multiple entries.
    pub fn separator(&self) -> Cow<'static, str> {
        match self {
            // PATH and library paths are colon separated.
            Entry::Path(_)
            | Entry::CPath(_)
            | Entry::CInclude(_)
            | Entry::CPlusInclude(_)
            | Entry::OBJCInclude(_)
            | Entry::LibraryPath(_)
            | Entry::LDLibraryPath(_)
            | Entry::LDRunPath(_) => Cow::Borrowed(":"),
            Entry::CustomPart { separator, .. } => Cow::Owned(separator.clone()),
            // Other flags are space separated.
            _ => Cow::Borrowed(" "),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separator_is_colon_for_path_like_vars() {
        assert_eq!(
            Entry::Path(PathEntry {
                path: "/opt/bin".to_string(),
                program: "tool".to_string(),
                version: "1".to_string(),
            })
            .separator()
            .as_ref(),
            ":",
        );

        assert_eq!(
            Entry::CPath("/opt/include".to_string())
                .separator()
                .as_ref(),
            ":"
        );
        assert_eq!(
            Entry::CInclude("/opt/include".to_string())
                .separator()
                .as_ref(),
            ":"
        );
        assert_eq!(
            Entry::CPlusInclude("/opt/include".to_string()).separator(),
            Cow::Borrowed(":")
        );
        assert_eq!(
            Entry::OBJCInclude("/opt/include".to_string()).separator(),
            Cow::Borrowed(":")
        );

        assert_eq!(
            Entry::LibraryPath("/opt/lib".to_string())
                .separator()
                .as_ref(),
            ":"
        );
        assert_eq!(
            Entry::LDLibraryPath("/opt/lib".to_string()).separator(),
            Cow::Borrowed(":")
        );
        assert_eq!(
            Entry::LDRunPath("/opt/lib".to_string())
                .separator()
                .as_ref(),
            ":"
        );
    }

    #[test]
    fn separator_is_space_for_flags() {
        assert_eq!(
            Entry::CFlag("-O2 -Wall".to_string()).separator().as_ref(),
            " "
        );
        assert_eq!(
            Entry::CXXFlag("-O2 -Wall".to_string()).separator().as_ref(),
            " "
        );
        assert_eq!(
            Entry::CPPFlag("-DDEBUG".to_string()).separator().as_ref(),
            " "
        );
        assert_eq!(
            Entry::LDFlag("-L/opt/lib".to_string()).separator().as_ref(),
            " "
        );
    }
}

impl std::fmt::Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Entry::Path(pe) => write!(f, "PATH: {} ({} {})", pe.path, pe.program, pe.version),
            Entry::CPath(s) => write!(f, "CPATH: {}", s),
            Entry::CInclude(s) => write!(f, "C_INCLUDE_PATH: {}", s),
            Entry::CPlusInclude(s) => write!(f, "CPLUS_INCLUDE_PATH: {}", s),
            Entry::OBJCInclude(s) => write!(f, "OBJC_INCLUDE_PATH: {}", s),
            Entry::CPPFlag(s) => write!(f, "CPPFLAGS: {}", s),
            Entry::CFlag(s) => write!(f, "CFLAGS: {}", s),
            Entry::CXXFlag(s) => write!(f, "CXXFLAGS: {}", s),
            Entry::LDFlag(s) => write!(f, "LDFLAGS: {}", s),
            Entry::LibraryPath(s) => write!(f, "LIBRARY_PATH: {}", s),
            Entry::LDLibraryPath(s) => write!(f, "LD_LIBRARY_PATH: {}", s),
            Entry::LDRunPath(s) => write!(f, "LD_RUN_PATH: {}", s),
            Entry::RanLib(s) => write!(f, "RANLIB: {}", s),
            Entry::CC(s) => write!(f, "CC: {}", s),
            Entry::CXX(s) => write!(f, "CXX: {}", s),
            Entry::AR(s) => write!(f, "AR: {}", s),
            Entry::Strip(s) => write!(f, "STRIP: {}", s),
            Entry::GCCExecPrefix(s) => write!(f, "GCC_EXEC_PREFIX: {}", s),
            Entry::CollectGCCOptions(s) => write!(f, "COLLECT_GCC_OPTIONS: {}", s),
            Entry::Lang(s) => write!(f, "LANG: {}", s),

            Entry::CustomScalar { name, value } => write!(f, "{}: {}", name, value),
            Entry::CustomPart { name, value, .. } => write!(f, "{}: {}", name, value),
        }
    }
}

/// An environment profile holds a name and a list of entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvProfile {
    pub name: String,
    pub entries: Vec<Entry>,
}

impl EnvProfile {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            entries: Vec::new(),
        }
    }
}
