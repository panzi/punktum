#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ErrorKind {
    OptionsParseError,
    IOError,
    SyntaxError,
    ExecError,
    IllegalArgument,
    NotEnoughArguments,
}

impl std::fmt::Display for ErrorKind {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SourceLocation {
    lineno: usize,
    column: usize,
}

impl SourceLocation {
    #[inline]
    pub fn new(lineno: usize, column: usize) -> Self {
        Self { lineno, column }
    }

    #[inline]
    pub fn lineno(&self) -> usize {
        self.lineno
    }

    #[inline]
    pub fn column(&self) -> usize {
        self.column
    }
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    cause: Option<Box<dyn std::error::Error>>,
    location: Option<SourceLocation>,
}

impl Error {
    #[inline]
    pub fn new<E>(kind: ErrorKind, cause: E, location: SourceLocation) -> Self
    where E: Into<Box<dyn std::error::Error + Send + Sync>> {
        Self {
            cause: Some(cause.into()),
            kind,
            location: Some(location),
        }
    }

    #[inline]
    pub fn with_cause<E>(kind: ErrorKind, cause: E) -> Self
    where E: Into<Box<dyn std::error::Error + Send + Sync>> {
        Self {
            cause: Some(cause.into()),
            kind,
            location: None,
        }
    }

    #[inline]
    pub fn with_location(kind: ErrorKind, location: SourceLocation) -> Self {
        Self {
            cause: None,
            kind,
            location: Some(location),
        }
    }

    #[inline]
    pub fn syntax_error(lineno: usize, column: usize) -> Self {
        Self {
            cause: None,
            kind: ErrorKind::SyntaxError,
            location: Some(SourceLocation::new(lineno, column)),
        }
    }

    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    #[inline]
    pub fn location(&self) -> &Option<SourceLocation> {
        &self.location
    }
}

impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Self {
        Self { kind, cause: None, location: None }
    }
}

impl std::fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.kind, f)?;

        if let Some(location) = self.location {
            write!(f, " on line {} at column {}", location.lineno, location.column)?;
        }

        if let Some(cause) = &self.cause {
            write!(f, ": {cause}")?
        }

        Ok(())
    }
}

impl std::error::Error for Error {
    #[inline]
    fn cause(&self) -> Option<&dyn std::error::Error> {
        if let Some(cause) = &self.cause {
            Some(cause.as_ref())
        } else {
            None
        }
    }
}

impl From<std::io::Error> for Error {
    #[inline]
    fn from(error: std::io::Error) -> Self {
        Self::with_cause(ErrorKind::IOError, error)
    }
}
