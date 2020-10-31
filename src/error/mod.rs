
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum RexecErrorType{
    FailedToExecuteProcess,
    FailedToKillProcess,
    UnexpectedEof,
    FailedToCreateSocketAddress,
    FailedToStartWebServer,
    FailedToSendStartCommand,
    InvalidCreateProcessRequest,
}

#[derive(Debug)]
pub struct RexecError{
    pub code : RexecErrorType,
    pub message: String,
}

impl fmt::Display for RexecErrorType{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RexecErrorType::FailedToExecuteProcess=> write!(f, "FailedToExecuteProcess"),
            RexecErrorType::FailedToKillProcess=> write!(f, "FailedToKillProcess"),
            RexecErrorType::UnexpectedEof=>write!(f,"UnexpectedEof"),
            RexecErrorType::FailedToCreateSocketAddress=>write!(f,"FailedToCreateSocketAddress"),
            RexecErrorType::FailedToStartWebServer=>write!(f,"FailedToStartWebServer"),
            RexecErrorType::FailedToSendStartCommand=>write!(f,"FailedToSendStartCommand"),
            RexecErrorType::InvalidCreateProcessRequest=>write!(f,"InvalidCreateProcessRequest"),
        }

    }
}

impl RexecError{
    pub fn code(code:  RexecErrorType) ->Self {
        RexecError{code, message : String::from("")}
    }
    pub fn code_msg(code:  RexecErrorType, message : String) ->Self {
        RexecError{code, message}
    }
}
impl Error for RexecError{}

impl fmt::Display for RexecError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[Rexec] {} : \"{}\"", self.code, self.message)
    }
}