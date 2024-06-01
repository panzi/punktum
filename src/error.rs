#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ErrorKind {
    OptionsParseError,
    IOError,
    SyntaxError,
    ExecError,
    NotEnoughArguments,
}

impl std::fmt::Display for ErrorKind {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    cause: Option<Box<dyn std::error::Error>>,
}

impl Error {
    #[inline]
    pub fn new<E>(kind: ErrorKind, cause: E) -> Self
    where E: Into<Box<dyn std::error::Error + Send + Sync>> {
        Self { cause: Some(cause.into()), kind }
    }

    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Self {
        Self { kind, cause: None }
    }
}

impl std::fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(cause) = &self.cause {
            write!(f, "{}: {}", self.kind, cause)
        } else {
            std::fmt::Display::fmt(&self.kind, f)
        }
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
        Self::new(ErrorKind::IOError, error)
    }
}

impl From<SyntaxError> for Error {
    #[inline]
    fn from(error: SyntaxError) -> Self {
        Self::new(ErrorKind::SyntaxError, error)
    }
}

#[derive(Debug)]
pub struct SyntaxError {
    lineno: usize,
    column: usize,
}

impl SyntaxError {
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

impl std::fmt::Display for SyntaxError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "on line {} at column {}", self.lineno, self.column)
    }
}

impl std::error::Error for SyntaxError {}
