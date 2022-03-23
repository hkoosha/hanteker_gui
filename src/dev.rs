use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use hanteker_lib::device::cfg::{
    AwgType, Coupling, Probe, RunningStatus, Scale, TimeScale, TriggerMode,
};
use hanteker_lib::models::hantek2d42::Hantek2D42;

fn exit() -> ! {
    std::process::exit(0);
}

fn exit_err(message: String) -> ! {
    eprintln!("{}", message);
    std::process::exit(0);
}

#[derive(Debug)]
pub(crate) enum DevCommand {
    Connect,
    Disconnect,
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

fn handle(rx: Receiver<DevCommand>, tx: Sender<Result<(), String>>) {
    let context = match libusb::Context::new() {
        Ok(context) => context,
        // TODO show a popup window about failure and then quit.
        Err(error) => exit_err(format!("error while opening USB context: {}", error)),
    };

    let mut device: Option<Hantek2D42> = None;

    loop {
        match rx.recv() {
            Err(_) => exit(),
            Ok(cmd) => {
                match &cmd {
                    DevCommand::Connect => {
                        if let Some(old_device) = &mut device {
                            old_device.usb.release().expect("could not release device");
                        }
                        match Hantek2D42::open(&context, Duration::from_millis(1000)) {
                            Ok(hantek) => {
                                device = Some(hantek);
                                match device.as_mut().unwrap().usb.claim() {
                                    Ok(_) => tx.send(Ok(())).unwrap_or_else(|_| exit()),
                                    Err(error) => {
                                        tx.send(Err(format!(
                                            "failed to claim device: {}",
                                            error.my_to_string()
                                        )))
                                        .unwrap_or_else(|_| exit());
                                    }
                                }
                            }
                            Err(error) => {
                                device = None;
                                tx.send(Err(format!(
                                    "failed to open device: {}",
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                        continue;
                    }
                    DevCommand::Disconnect => {
                        if device.is_none() {
                            tx.send(Ok(())).unwrap_or_else(|_| exit());
                            continue;
                        }
                        device
                            .as_mut()
                            .unwrap()
                            .usb
                            .release()
                            .expect("could not disconnect");
                        device = None;
                        tx.send(Ok(())).unwrap_or_else(|_| exit());
                        continue;
                    }
                    _ => {}
                }

                let device = match &mut device {
                    None => {
                        tx.send(Err("not connected".to_string()))
                            .unwrap_or_else(|_| exit());
                        continue;
                    }
                    Some(device) => device,
                };

                match cmd {
                    DevCommand::Connect => unreachable!(),
                    DevCommand::Disconnect => unreachable!(),
                    DevCommand::Coupling(channel, coupling) => {
                        match device.set_channel_coupling(channel, coupling.clone()) {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!("failed to set channel coupling, channel={}, coupling={}, error={}",
                                                    channel, coupling.my_to_string(), error.my_to_string())))
                                  .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::ChannelEnable(channel, enabled) => {
                        let result = match enabled {
                            true => device.enable_channel(channel),
                            false => device.disable_channel(channel),
                        };
                        match result {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to set channel status, channel={}, error={}",
                                    channel,
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::Probe(channel, probe) => {
                        match device.set_channel_probe(channel, probe.clone()) {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to set channel probe, channel={}, probe={}, error={}",
                                    channel,
                                    probe.my_to_string(),
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::Scale(channel, scale) => {
                        match device.set_channel_scale(channel, scale.clone()) {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to set channel scale, channel={}, scale={}, error={}",
                                    channel,
                                    scale.my_to_string(),
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::Offset(channel, offset) => {
                        match device.set_channel_offset_with_auto_adjustment(channel, offset) {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to set channel offset, channel={}, offset={}, error={}",
                                    channel,
                                    offset,
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::BwLimit(channel, bw_limit) => {
                        let result = match bw_limit {
                            true => device.channel_enable_bandwidth_limit(channel),
                            false => device.channel_disable_bandwidth_limit(channel),
                        };
                        match result {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to set channel bandwidth limit, channel={}, error={}",
                                    channel,
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::TimeScale(time_scale) => {
                        match device.set_time_scale(time_scale.clone()) {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to set time scale, time_scale={}, error={}",
                                    time_scale.my_to_string(),
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::TimeOffset(offset) => {
                        match device.set_time_offset_with_auto_adjustment(offset) {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to set time offset, time_offset={}, error={}",
                                    offset,
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::TriggerSource(channel) => {
                        match device.set_trigger_source(channel) {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to set trigger source, trigger_source={}, error={}",
                                    channel,
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::TriggerMode(mode) => match device.set_trigger_mode(mode.clone()) {
                        Ok(_) => {
                            tx.send(Ok(())).unwrap_or_else(|_| exit());
                        }
                        Err(error) => {
                            tx.send(Err(format!(
                                "failed to set trigger mode, trigger_mode={}, error={}",
                                mode.my_to_string(),
                                error.my_to_string()
                            )))
                            .unwrap_or_else(|_| exit());
                        }
                    },
                    DevCommand::TriggerLevel(level) => {
                        match device.set_trigger_level_with_auto_adjustment(level) {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to set trigger level, trigger_level={}, error={}",
                                    level,
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::ScopeRunning(status) => {
                        let running_result = match status {
                            RunningStatus::Start => device.start(),
                            RunningStatus::Stop => device.stop(),
                        };
                        match running_result {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to change running status, error={}",
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::AwgRunningStatus(status) => {
                        let running_result = match status {
                            RunningStatus::Start => device.awg_start(),
                            RunningStatus::Stop => device.awg_stop(),
                        };
                        match running_result {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to change awg running status, error={}",
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::AwgFrequency(frequency) => {
                        match device.set_awg_frequency(frequency) {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to set awg frequency, error={}",
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::AwgAmplitude(amplitude) => {
                        match device.set_awg_amplitude(amplitude) {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to set awg amplitude, error={}",
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::AwgType(awg_type) => match device.set_awg_type(awg_type) {
                        Ok(_) => {
                            tx.send(Ok(())).unwrap_or_else(|_| exit());
                        }
                        Err(error) => {
                            tx.send(Err(format!(
                                "failed to set awg type, error={}",
                                error.my_to_string()
                            )))
                            .unwrap_or_else(|_| exit());
                        }
                    },
                    DevCommand::AwgOffset(offset) => match device.set_awg_offset(offset) {
                        Ok(_) => {
                            tx.send(Ok(())).unwrap_or_else(|_| exit());
                        }
                        Err(error) => {
                            tx.send(Err(format!(
                                "failed to set awg offset, error={}",
                                error.my_to_string()
                            )))
                            .unwrap_or_else(|_| exit());
                        }
                    },
                    DevCommand::AwgDutySquare(duty_square) => {
                        match device.set_awg_duty_square(duty_square) {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to set awg duty::square, error={}",
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                    DevCommand::AwgDutyRamp(duty_ramp) => match device.set_awg_duty_ramp(duty_ramp)
                    {
                        Ok(_) => {
                            tx.send(Ok(())).unwrap_or_else(|_| exit());
                        }
                        Err(error) => {
                            tx.send(Err(format!(
                                "failed to set awg duty::ramp, error={}",
                                error.my_to_string()
                            )))
                            .unwrap_or_else(|_| exit());
                        }
                    },
                    DevCommand::AwgDutyTrap(high, low, rise) => {
                        match device.set_awg_duty_trap(high, low, rise) {
                            Ok(_) => {
                                tx.send(Ok(())).unwrap_or_else(|_| exit());
                            }
                            Err(error) => {
                                tx.send(Err(format!(
                                    "failed to set awg duty::trap, error={}",
                                    error.my_to_string()
                                )))
                                .unwrap_or_else(|_| exit());
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn handler_thread() -> (Sender<DevCommand>, Receiver<Result<(), String>>) {
    let (tx0, rx0) = mpsc::channel();
    let (tx1, rx1) = mpsc::channel();
    thread::spawn(move || handle(rx0, tx1));
    (tx0, rx1)
}
