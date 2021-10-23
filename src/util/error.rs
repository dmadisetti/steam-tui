use crate::util::parser::Command;

use shellexpand::LookupError;
use std::error;
use exitfailure::ExitFailure;
use std::fmt;
use std::fmt::Debug;
use std::io;
use std::str::Utf8Error;
use std::sync::mpsc::{RecvError, SendError};
use std::sync::PoisonError;

#[derive(Debug)]
pub enum STError {
    Io(io::Error),
    Process(io::Error),
    Recv(RecvError),
    Problem(String),
}

impl From<io::Error> for STError {
    fn from(err: io::Error) -> STError {
        STError::Io(err)
    }
}

impl From<exitfailure::ExitFailure> for STError {
    fn from(err: exitfailure::ExitFailure) -> STError {
        STError::Problem(format!("{:?}", err))
    }
}

impl From<RecvError> for STError {
    fn from(err: RecvError) -> STError {
        STError::Recv(err)
    }
}

impl From<SendError<Command>> for STError {
    fn from(err: SendError<Command>) -> STError {
        STError::Problem(format!("{:?}", err))
    }
}

impl From<SendError<String>> for STError {
    fn from(err: SendError<String>) -> STError {
        STError::Problem(format!("{:?}", err))
    }
}

impl From<Box<dyn error::Error>> for STError {
    fn from(err: Box<dyn error::Error>) -> STError {
        STError::Problem(format!("{:?}", err))
    }
}

impl From<serde_json::Error> for STError {
    fn from(err: serde_json::Error) -> STError {
        STError::Problem(format!("{:?}", err))
    }
}

impl From<Utf8Error> for STError {
    fn from(err: Utf8Error) -> STError {
        STError::Problem(format!("{:?}", err))
    }
}

impl<T> From<PoisonError<T>> for STError {
    fn from(err: PoisonError<T>) -> STError {
        STError::Problem(format!("{:?}", err))
    }
}

impl<T: Debug> From<LookupError<T>> for STError {
    fn from(err: LookupError<T>) -> STError {
        STError::Problem(format!("{:?}", err))
    }
}

impl fmt::Display for STError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            STError::Process(e) => write!(
                f,
                "An error occured spawning the steamcmd process. Do you have it installed?\n{:?}",
                e
            ),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl error::Error for STError {
    fn description(&self) -> &str {
        "woosp"
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        // Pass on reference
        None
    }
}
