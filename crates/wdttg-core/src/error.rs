use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("parse error at line {line}: {message}")]
    Parse { line: usize, message: String },

    #[error("validation error: {0}")]
    Validation(String),

    #[error("overlap detected: {0}")]
    Overlap(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("entry not found")]
    NotFound,
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
        assert!(err.to_string().contains("gone"));
    }

    #[test]
    fn parse_error_display() {
        let err = Error::Parse {
            line: 5,
            message: "bad datetime".into(),
        };
        assert_eq!(err.to_string(), "parse error at line 5: bad datetime");
    }

    #[test]
    fn result_type_works_with_question_mark() {
        fn fallible() -> Result<i32> {
            Ok(42)
        }
        assert_eq!(fallible().unwrap(), 42);
    }

    #[test]
    fn overlap_error_display() {
        let err = Error::Overlap("09:00-10:00 conflicts with 09:30-11:00".into());
        assert!(err.to_string().contains("overlap detected"));
    }

    #[test]
    fn not_found_display() {
        let err = Error::NotFound;
        assert_eq!(err.to_string(), "entry not found");
    }
}
