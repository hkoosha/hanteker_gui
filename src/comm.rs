use std::fmt::{Display, Formatter};

use hanteker_lib::device::cfg::{
    AwgType, Coupling, DeviceFunction, Probe, RunningStatus, Scale, TimeScale, TriggerMode,
};

#[derive(Debug)]
pub(crate) enum DevCommand {
    Capture(Vec<usize>, usize),

    Connect,
    Disconnect,

    DeviceFunction(DeviceFunction),

    ScopeRunning(RunningStatus),

    ChannelEnable(usize, bool),
    Coupling(usize, Coupling),
    Probe(usize, Probe),
    Scale(usize, Scale),
    Offset(usize, f32),
    BwLimit(usize, bool),

    TimeScale(TimeScale),
    TimeOffset(f32),
    TriggerSource(usize),
    TriggerMode(TriggerMode),
    TriggerLevel(f32),

    AwgRunningStatus(RunningStatus),
    AwgFrequency(f32),
    AwgAmplitude(f32),
    AwgType(AwgType),
    AwgOffset(f32),
    AwgDutySquare(f32),
    AwgDutyRamp(f32),
    AwgDutyTrap(f32, f32, f32),
}

pub(crate) enum DevCommandResult {
    EmptyResult,
    CaptureResult(Vec<u8>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum Severity {
    INFO,
    ERROR,
}

impl Display for Severity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Severity::INFO => "INFO",
                Severity::ERROR => "ERROR",
            }
        )
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct TextMessage {
    pub(crate) msg: String,
    pub(crate) severity: Severity,
}

impl TextMessage {
    pub(crate) fn info(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            severity: Severity::INFO,
        }
    }

    pub(crate) fn error(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            severity: Severity::ERROR,
        }
    }
}
