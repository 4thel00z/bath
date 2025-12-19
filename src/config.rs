use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathEntry {
    pub path: String,
    pub program: String,
    pub version: String,
}

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
}

impl Entry {
    /// Returns the corresponding environment variable name.
    pub fn var_name(&self) -> &'static str {
        match self {
            Entry::Path(_)          => "PATH",
            Entry::CPath(_)         => "CPATH",
            Entry::CInclude(_)      => "C_INCLUDE_PATH",
            Entry::CPlusInclude(_)  => "CPLUS_INCLUDE_PATH",
            Entry::OBJCInclude(_)   => "OBJC_INCLUDE_PATH",
            Entry::CPPFlag(_)       => "CPPFLAGS",
            Entry::CFlag(_)         => "CFLAGS",
            Entry::CXXFlag(_)       => "CXXFLAGS",
            Entry::LDFlag(_)        => "LDFLAGS",
            Entry::LibraryPath(_)   => "LIBRARY_PATH",
            Entry::LDLibraryPath(_) => "LD_LIBRARY_PATH",
            Entry::LDRunPath(_)     => "LD_RUN_PATH",
            Entry::RanLib(_)        => "RANLIB",
            Entry::CC(_)            => "CC",
            Entry::CXX(_)           => "CXX",
            Entry::AR(_)            => "AR",
            Entry::Strip(_)         => "STRIP",
            Entry::GCCExecPrefix(_) => "GCC_EXEC_PREFIX",
            Entry::CollectGCCOptions(_) => "COLLECT_GCC_OPTIONS",
            Entry::Lang(_)          => "LANG",
        }
    }

    /// Returns the default separator used when joining multiple entries.
    pub fn separator(&self) -> &'static str {
        match self {
            // PATH and library paths are colon separated.
            Entry::Path(_)
            | Entry::CPath(_)
            | Entry::CInclude(_)
            | Entry::CPlusInclude(_)
            | Entry::OBJCInclude(_)
            | Entry::LibraryPath(_)
            | Entry::LDLibraryPath(_)
            | Entry::LDRunPath(_) => ":",
            // Other flags are space separated.
            _ => " ",
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
            .separator(),
            ":"
        );

        assert_eq!(Entry::CPath("/opt/include".to_string()).separator(), ":");
        assert_eq!(Entry::CInclude("/opt/include".to_string()).separator(), ":");
        assert_eq!(Entry::CPlusInclude("/opt/include".to_string()).separator(), ":");
        assert_eq!(Entry::OBJCInclude("/opt/include".to_string()).separator(), ":");

        assert_eq!(Entry::LibraryPath("/opt/lib".to_string()).separator(), ":");
        assert_eq!(Entry::LDLibraryPath("/opt/lib".to_string()).separator(), ":");
        assert_eq!(Entry::LDRunPath("/opt/lib".to_string()).separator(), ":");
    }

    #[test]
    fn separator_is_space_for_flags() {
        assert_eq!(Entry::CFlag("-O2 -Wall".to_string()).separator(), " ");
        assert_eq!(Entry::CXXFlag("-O2 -Wall".to_string()).separator(), " ");
        assert_eq!(Entry::CPPFlag("-DDEBUG".to_string()).separator(), " ");
        assert_eq!(Entry::LDFlag("-L/opt/lib".to_string()).separator(), " ");
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
