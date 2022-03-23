#![feature(iter_intersperse)]
#![windows_subsystem = "windows"]

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use anyhow::bail;
use druid::im::Vector;
use druid::widget::{Button, CrossAxisAlignment, Flex, Label, MainAxisAlignment, Switch};
use druid::{AppLauncher, Data, UnitPoint, Widget, WidgetExt, WindowDesc};
use druid_widget_nursery::{DropdownSelect, WidgetExt as WidgetExtNursery};
use hanteker_lib::device::cfg::*;
use log::{debug, error, info, trace};
use pretty_env_logger::formatted_builder;

use crate::dev::{handler_thread, DevCommand};
use crate::widget::f32_formatter::float_text_unrestricted;
use crate::widget::label::{label, label_c, label_ct};
use crate::widget::scope::ScopeGraph;
use crate::widget::{lens_of, t, tt};

mod dev;
mod widget;

#[derive(Clone)]
pub struct HantekState {
    messages: VecDeque<String>,
    connected: bool,
    initializing: bool,
    cfg: HantekConfig,
    tx: Sender<DevCommand>,
    rx: Arc<Receiver<Result<(), String>>>,
}

impl Data for HantekState {
    fn same(&self, other: &Self) -> bool {
        self.messages == other.messages
            && self.connected == other.connected
            && self.initializing == other.initializing
            && self.cfg.same(&other.cfg)
    }
}

impl HantekState {
    fn new(rx: Sender<DevCommand>, tx: Arc<Receiver<Result<(), String>>>) -> Self {
        Self {
            cfg: HantekConfig::new(2),
            messages: VecDeque::new(),
            connected: false,
            initializing: true,
            tx: rx,
            rx: tx,
        }
    }

    fn message(&mut self, message: impl Into<String>) {
        // self.messages.pop_front();
        self.messages.push_back(message.into());
    }

    fn get_messages(&self) -> String {
        // TODO make rev() on iter work.
        let len = self.messages.len();
        let mut vec = self
            .messages
            .iter()
            .cloned()
            .zip((0..len).into_iter())
            .map(|(s, i)| format!("{}: {}", i, s))
            .intersperse("\n".to_string())
            .collect::<Vec<String>>();
        vec.reverse();
        vec.into_iter().collect()
    }

    // ------------

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn disconnect(&mut self) {
        let my_name = "disconnect()";
        trace!("UI => {}", my_name);

        self.initializing = false;

        self.message("disconnecting");
        self.tx.send(DevCommand::Disconnect).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("disconnected");
                self.connected = false;
                self.messages.clear();
            }
            Err(error) => self.message(error),
        };
    }

    fn connect(&mut self) {
        let my_name = "connect()";
        trace!("UI => {}", my_name);

        self.message("connecting");
        self.tx.send(DevCommand::Connect).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.connected = true;
                self.initializing = false;
                match self.try_connect() {
                    Ok(_) => {
                        self.message("connected");
                    }
                    Err(error) => {
                        self.connected = true;
                        self.initializing = false;
                        self.message(error.to_string());
                    }
                }
            }
            Err(error) => self.message(error),
        };
    }

    fn try_connect(&mut self) -> anyhow::Result<()> {
        self.set_running(true);
        self.send_running()?;

        self.set_coupling(1, Coupling::DC);
        self.send_coupling(1)?;
        self.set_coupling(2, Coupling::DC);
        self.send_coupling(2)?;

        self.set_probe(1, Probe::X1);
        self.send_probe(1)?;
        self.set_probe(2, Probe::X1);
        self.send_probe(2)?;

        self.set_scale0(1, Scale::v10);
        self.send_scale(1)?;
        self.set_scale0(2, Scale::v10);
        self.send_scale(2)?;

        self.set_offset(1, 0.0);
        self.send_offset(1)?;
        self.set_offset(2, 0.0);
        self.send_offset(2)?;

        self.set_enabled_channel(1, true);
        self.send_channel_enable(1)?;
        self.set_enabled_channel(2, false);
        self.send_channel_enable(2)?;

        self.set_bw_limit(1, false);
        self.send_bw_limit(1)?;
        self.set_bw_limit(2, false);
        self.send_bw_limit(2)?;

        self.set_time_scale(TimeScale::ms1);
        self.send_time_scale()?;
        self.set_time_offset(0.0);
        self.send_time_offset()?;
        self.set_trigger_source(1);
        self.send_trigger_source()?;
        self.set_trigger_mode(TriggerMode::Auto);
        self.send_trigger_mode()?;
        self.set_trigger_level(0.0);
        self.send_trigger_level()?;

        self.set_awg_running(RunningStatus::Stop);
        self.send_awg_running()?;
        self.set_awg_frequency(1.0);
        self.send_awg_frequency()?;
        self.set_awg_amplitude(1.0);
        self.send_awg_amplitude()?;
        self.set_awg_type(AwgType::Square);
        self.send_awg_type()?;
        self.set_awg_offset(0.0);
        self.send_awg_offset()?;
        self.set_awg_duty_square(0.0);
        self.send_awg_duty_square()?;
        self.set_awg_duty_ramp(0.0);
        self.send_awg_duty_ramp()?;
        self.set_awg_duty_trap_low(0.0);
        self.set_awg_duty_trap_high(0.0);
        self.set_awg_duty_trap_rise(0.0);
        self.send_awg_duty_trap()?;

        Ok(())
    }

    // ------------

    fn on_running(&mut self) {
        let _err = self.send_running();
    }

    fn send_running(&mut self) -> anyhow::Result<()> {
        let my_name = "on_running()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else if self.is_running_disabled() {
            trace!("UI => {}, SKIPPED/NO_STATE", my_name);
        } else {
            trace!("UI => {}", my_name);
        }

        let new_status = self.cfg.running_status.as_ref().unwrap();
        debug!("UI => {}::{}", my_name, new_status.my_to_string());

        self.tx
            .send(DevCommand::ScopeRunning(new_status.clone()))
            .unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                match new_status {
                    RunningStatus::Start => self.message("running"),
                    RunningStatus::Stop => self.message("stopped"),
                };
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_running(&self) -> bool {
        self.cfg
            .running_status
            .as_ref()
            .unwrap_or(&RunningStatus::Stop)
            .is_start()
    }

    fn set_running(&mut self, new_value: bool) {
        self.cfg.running_status = Some(match new_value {
            true => RunningStatus::Start,
            false => RunningStatus::Stop,
        });
    }

    fn is_running_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.running_status.is_none()
    }

    // ------------

    fn on_coupling(&mut self, channel: usize) {
        let _err = self.send_coupling(channel);
    }

    fn send_coupling(&mut self, channel: usize) -> anyhow::Result<()> {
        let my_name = format!("on_coupling(channel={})", channel);
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let coupling = self.cfg.channel_coupling[&channel]
            .as_ref()
            .unwrap()
            .clone();
        debug!("UI => {}::{}", my_name, coupling.my_to_string());

        self.message(format!(
            "setting coupling={}, channel={}",
            coupling.my_to_string(),
            channel
        ));

        self.tx
            .send(DevCommand::Coupling(channel, coupling))
            .unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("coupling set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn is_coupling_disabled(&self, channel: usize) -> bool {
        !self.is_connected() || self.cfg.channel_coupling[&channel].is_none()
    }

    fn get_coupling(&self, channel: usize) -> Coupling {
        self.cfg.channel_coupling[&channel]
            .as_ref()
            .unwrap_or(&Coupling::AC)
            .clone()
    }

    fn set_coupling(&mut self, channel: usize, new_value: Coupling) {
        self.cfg.channel_coupling.insert(channel, Some(new_value));
    }

    // ------------

    fn on_probe(&mut self, channel: usize) {
        let _err = self.send_probe(channel);
    }

    fn send_probe(&mut self, channel: usize) -> anyhow::Result<()> {
        let my_name = format!("on_probe(channel={})", channel);
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let probe = self.cfg.channel_probe[&channel].as_ref().unwrap().clone();
        debug!("UI => {}::{}", my_name, probe.my_to_string());

        self.message(format!(
            "setting probe={}, channel={}",
            probe.my_to_string(),
            channel
        ));

        self.tx.send(DevCommand::Probe(channel, probe)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("probe set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn is_probe_disabled(&self, channel: usize) -> bool {
        !self.is_connected() || self.cfg.channel_probe[&channel].is_none()
    }

    fn get_probe(&self, channel: usize) -> Probe {
        self.cfg.channel_probe[&channel]
            .as_ref()
            .unwrap_or(&Probe::X1)
            .clone()
    }

    fn set_probe(&mut self, channel: usize, new_value: Probe) {
        self.cfg.channel_probe.insert(channel, Some(new_value));
    }

    // ------------

    fn on_scale(&mut self, channel: usize) {
        let _err = self.send_scale(channel);
    }

    fn send_scale(&mut self, channel: usize) -> anyhow::Result<()> {
        let my_name = format!("on_scale(channel={})", channel);
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let scale = self.cfg.channel_scale[&channel].as_ref().unwrap().clone();
        debug!("UI => {}::{}", my_name, scale.my_to_string());

        self.message(format!(
            "setting scale={}, channel={}",
            scale.my_to_string(),
            channel
        ));

        self.tx.send(DevCommand::Scale(channel, scale)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("scale set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn is_scale_disabled(&self, channel: usize) -> bool {
        !self.is_connected() || self.cfg.channel_scale[&channel].is_none()
    }

    fn get_scale(&self, channel: usize) -> Scale {
        self.cfg.channel_scale[&channel]
            .as_ref()
            .unwrap_or(&Scale::mv10)
            .clone()
    }

    fn set_scale0(&mut self, channel: usize, new_value: Scale) {
        self.cfg.channel_scale.insert(channel, Some(new_value));
    }

    // ------------

    fn on_offset(&mut self, channel: usize) {
        let _err = self.send_offset(channel);
    }

    fn send_offset(&mut self, channel: usize) -> anyhow::Result<()> {
        let my_name = format!("on_offset(channel={})", channel);
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let offset = self.cfg.channel_offset[&channel].unwrap();
        debug!("UI => {}::{}", my_name, offset);

        self.message(format!("setting offset={}, channel={}", offset, channel));

        self.tx.send(DevCommand::Offset(channel, offset)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("offset set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn is_offset_disabled(&self, channel: usize) -> bool {
        !self.is_connected() || self.cfg.channel_offset[&channel].is_none()
    }

    fn get_offset(&self, channel: usize) -> f32 {
        self.cfg.channel_offset[&channel].unwrap_or(0.0f32)
    }

    fn set_offset(&mut self, channel: usize, new_value: f32) {
        self.cfg.channel_offset.insert(channel, Some(new_value));
    }

    // ------------

    fn on_channel_enable(&mut self, channel: usize) {
        let _err = self.send_channel_enable(channel);
    }

    fn send_channel_enable(&mut self, channel: usize) -> anyhow::Result<()> {
        let my_name = format!("on_channel_enable(channel={})", channel);
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let enabled = self.cfg.enabled_channels[&channel].unwrap();
        debug!("UI => {}::{}", my_name, enabled);

        self.message(format!(
            "setting channel status, channel={}, status={}",
            channel,
            match enabled {
                true => "enabled",
                false => "disabled",
            }
        ));

        self.tx
            .send(DevCommand::ChannelEnable(channel, enabled))
            .unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("channel status set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn is_channel_enable_disabled(&self, channel: usize) -> bool {
        !self.is_connected() || self.cfg.enabled_channels[&channel].is_none()
    }

    fn get_enabled_channel(&self, channel: usize) -> bool {
        self.cfg.enabled_channels[&channel].unwrap_or(false)
    }

    fn set_enabled_channel(&mut self, channel: usize, new_value: bool) {
        self.cfg.enabled_channels.insert(channel, Some(new_value));
    }

    // ------------

    fn on_bw_limit(&mut self, channel: usize) {
        let _err = self.send_bw_limit(channel);
    }

    fn send_bw_limit(&mut self, channel: usize) -> anyhow::Result<()> {
        let my_name = format!("on_bw_limit(channel={})", channel);
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let enabled = self.cfg.channel_bandwidth_limit[&channel].unwrap();
        debug!("UI => {}::{}", my_name, enabled);

        self.message(format!(
            "setting channel bandwidth limit, channel={}, limit={}",
            channel,
            match enabled {
                true => "enabled",
                false => "disabled",
            }
        ));

        self.tx.send(DevCommand::BwLimit(channel, enabled)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("channel bandwidth limit configuration set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_bw_limit(&self, channel: usize) -> bool {
        self.cfg.channel_bandwidth_limit[&channel].unwrap_or(false)
    }

    fn set_bw_limit(&mut self, channel: usize, new_value: bool) {
        self.cfg
            .channel_bandwidth_limit
            .insert(channel, Some(new_value));
    }

    fn is_bw_limit_enable_disabled(&self, channel: usize) -> bool {
        !self.is_connected() || self.cfg.channel_bandwidth_limit[&channel].is_none()
    }

    // ------------

    fn on_time_scale(&mut self) {
        let _err = self.send_time_scale();
    }

    fn send_time_scale(&mut self) -> anyhow::Result<()> {
        let my_name = "on_time_scale()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let scale = self.cfg.time_scale.as_ref().unwrap().clone();
        debug!("UI => {}::{}", my_name, scale.my_to_string());

        self.message(format!("setting time_scale={}", scale.my_to_string()));

        self.tx.send(DevCommand::TimeScale(scale)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("time scale set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_time_scale(&self) -> TimeScale {
        self.cfg
            .time_scale
            .as_ref()
            .unwrap_or(&TimeScale::ns5)
            .clone()
    }

    fn set_time_scale(&mut self, new_value: TimeScale) {
        self.cfg.time_scale = Some(new_value);
    }

    fn is_time_scale_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.time_scale.is_none()
    }

    // ------------

    fn on_time_offset(&mut self) {
        let _err = self.send_time_offset();
    }

    fn send_time_offset(&mut self) -> anyhow::Result<()> {
        let my_name = "on_time_offset()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let time_offset = self.cfg.time_offset.unwrap();
        debug!("UI => {}::{}", my_name, time_offset);

        self.message(format!("setting time_offset={}", time_offset));

        self.tx.send(DevCommand::TimeOffset(time_offset)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("time offset set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_time_offset(&self) -> f32 {
        self.cfg.time_offset.unwrap_or(0.0)
    }

    fn set_time_offset(&mut self, new_value: f32) {
        self.cfg.time_offset = Some(new_value);
    }

    fn is_time_offset_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.time_offset.is_none()
    }

    // ------------

    fn on_trigger_source(&mut self) {
        let _err = self.send_trigger_source();
    }

    fn send_trigger_source(&mut self) -> anyhow::Result<()> {
        let my_name = "on_trigger_source()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let source = self.cfg.trigger_source_channel.unwrap();
        debug!("UI => {}::{}", my_name, source);

        self.message(format!("setting trigger_source_channel={}", source));

        self.tx.send(DevCommand::TriggerSource(source)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("trigger source set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_trigger_source(&self) -> usize {
        self.cfg.trigger_source_channel.unwrap_or(1usize)
    }

    fn set_trigger_source(&mut self, new_value: usize) {
        self.cfg.trigger_source_channel = Some(new_value);
    }

    fn is_trigger_source_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.trigger_source_channel.is_none()
    }

    // ------------

    fn on_trigger_mode(&mut self) {
        let _err = self.send_trigger_mode();
    }

    fn send_trigger_mode(&mut self) -> anyhow::Result<()> {
        let my_name = "on_trigger_mode()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let mode = self.cfg.trigger_mode.as_ref().unwrap().clone();
        debug!("UI => {}::{}", my_name, mode.my_to_string());

        self.message(format!("setting trigger_mode={}", mode.my_to_string()));

        self.tx.send(DevCommand::TriggerMode(mode)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("trigger mode set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_trigger_mode(&self) -> TriggerMode {
        self.cfg
            .trigger_mode
            .as_ref()
            .unwrap_or(&TriggerMode::Auto)
            .clone()
    }

    fn set_trigger_mode(&mut self, new_value: TriggerMode) {
        self.cfg.trigger_mode = Some(new_value);
    }

    fn is_trigger_mode_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.trigger_mode.is_none()
    }

    // ------------

    fn on_trigger_level(&mut self) {
        let _err = self.send_trigger_level();
    }

    fn send_trigger_level(&mut self) -> anyhow::Result<()> {
        let my_name = "on_trigger_level()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let level = self.cfg.trigger_level.unwrap();
        debug!("UI => {}::{}", my_name, level);

        self.message(format!("setting trigger_level={}", level));

        self.tx.send(DevCommand::TriggerLevel(level)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("trigger level set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_trigger_level(&self) -> f32 {
        self.cfg.trigger_level.unwrap_or(0.0)
    }

    fn set_trigger_level(&mut self, new_value: f32) {
        self.cfg.trigger_level = Some(new_value);
    }

    fn is_trigger_level_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.trigger_level.is_none()
    }

    // ------------

    fn on_awg_running(&mut self) {
        let _err = self.send_awg_running();
    }

    fn send_awg_running(&mut self) -> anyhow::Result<()> {
        let my_name = "on_awg_running()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let status = self.cfg.awg_running_status.as_ref().unwrap().clone();
        debug!("UI => {}::{}", my_name, status.my_to_string());

        self.message(format!(
            "setting awg_running_status={}",
            status.my_to_string()
        ));

        self.tx.send(DevCommand::AwgRunningStatus(status)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("awg running status set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_awg_running(&self) -> RunningStatus {
        self.cfg
            .awg_running_status
            .as_ref()
            .unwrap_or(&RunningStatus::Stop)
            .clone()
    }

    fn set_awg_running(&mut self, new_value: RunningStatus) {
        self.cfg.awg_running_status = Some(new_value);
    }

    fn is_awg_running_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.awg_running_status.is_none()
    }

    // ------------

    fn on_awg_frequency(&mut self) {
        let _err = self.send_awg_frequency();
    }

    fn send_awg_frequency(&mut self) -> anyhow::Result<()> {
        let my_name = "on_awg_frequency()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let frequency = self.cfg.awg_frequency.unwrap();
        debug!("UI => {}::{}", my_name, frequency);

        self.message(format!("setting awg_running_frequency={}", frequency));

        self.tx.send(DevCommand::AwgFrequency(frequency)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("awg frequency set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_awg_frequency(&self) -> f32 {
        self.cfg.awg_frequency.unwrap_or(0.0)
    }

    fn set_awg_frequency(&mut self, new_value: f32) {
        self.cfg.awg_frequency = Some(new_value);
    }

    fn is_awg_frequency_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.awg_frequency.is_none()
    }

    // ------------

    fn on_awg_amplitude(&mut self) {
        let _err = self.send_awg_amplitude();
    }

    fn send_awg_amplitude(&mut self) -> anyhow::Result<()> {
        let my_name = "on_awg_amplitude()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let amplitude = self.cfg.awg_amplitude.unwrap();
        debug!("UI => {}::{}", my_name, amplitude);

        self.message(format!("setting awg_running_amplitude={}", amplitude));

        self.tx.send(DevCommand::AwgAmplitude(amplitude)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("awg amplitude set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_awg_amplitude(&self) -> f32 {
        self.cfg.awg_amplitude.unwrap_or(0.0)
    }

    fn set_awg_amplitude(&mut self, new_value: f32) {
        self.cfg.awg_amplitude = Some(new_value);
    }

    fn is_awg_amplitude_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.awg_amplitude.is_none()
    }

    // ------------

    fn on_awg_type(&mut self) {
        let _err = self.send_awg_type();
    }

    fn send_awg_type(&mut self) -> anyhow::Result<()> {
        let my_name = "on_awg_type()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let awg_type = self.cfg.awg_type.as_ref().unwrap().clone();
        debug!("UI => {}::{}", my_name, awg_type.my_to_string());

        self.message(format!(
            "setting awg_running_type={}",
            awg_type.my_to_string()
        ));

        self.tx.send(DevCommand::AwgType(awg_type)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("awg type set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_awg_type(&self) -> AwgType {
        self.cfg
            .awg_type
            .as_ref()
            .unwrap_or(&AwgType::Square)
            .clone()
    }

    fn set_awg_type(&mut self, new_value: AwgType) {
        self.cfg.awg_type = Some(new_value);
    }

    fn is_awg_type_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.awg_type.is_none()
    }

    // ------------

    fn on_awg_offset(&mut self) {
        let _err = self.send_awg_offset();
    }

    fn send_awg_offset(&mut self) -> anyhow::Result<()> {
        let my_name = "on_awg_offset()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let offset = self.cfg.awg_offset.unwrap();
        debug!("UI => {}::{}", my_name, offset);

        self.message(format!("setting awg_offset={}", offset));

        self.tx.send(DevCommand::AwgOffset(offset)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("awg offset set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_awg_offset(&self) -> f32 {
        self.cfg.awg_offset.unwrap_or(0.0)
    }

    fn set_awg_offset(&mut self, new_value: f32) {
        self.cfg.awg_offset = Some(new_value);
    }

    fn is_awg_offset_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.awg_offset.is_none()
    }

    // ------------

    fn on_awg_duty_square(&mut self) {
        let _err = self.send_awg_duty_square();
    }

    fn send_awg_duty_square(&mut self) -> anyhow::Result<()> {
        let my_name = "on_awg_duty_square()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let duty = self.cfg.awg_duty_square.unwrap();
        debug!("UI => {}::{}", my_name, duty);

        self.message(format!("setting awg_duty_square={}", duty));

        self.tx.send(DevCommand::AwgDutySquare(duty)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("awg duty square set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_awg_duty_square(&self) -> f32 {
        self.cfg.awg_duty_square.unwrap_or(0.0)
    }

    fn set_awg_duty_square(&mut self, new_value: f32) {
        self.cfg.awg_duty_square = Some(new_value);
    }

    fn is_awg_duty_square_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.awg_duty_square.is_none()
    }

    // ------------

    fn on_awg_duty_ramp(&mut self) {
        let _err = self.send_awg_duty_ramp();
    }

    fn send_awg_duty_ramp(&mut self) -> anyhow::Result<()> {
        let my_name = "on_awg_duty_ramp()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let duty = self.cfg.awg_duty_ramp.unwrap();
        debug!("UI => {}::{}", my_name, duty);

        self.message(format!("setting awg_duty_ramp={}", duty));

        self.tx.send(DevCommand::AwgDutyRamp(duty)).unwrap();
        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("awg duty ramp set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_awg_duty_ramp(&self) -> f32 {
        self.cfg.awg_duty_ramp.unwrap_or(0.0)
    }

    fn set_awg_duty_ramp(&mut self, new_value: f32) {
        self.cfg.awg_duty_ramp = Some(new_value);
    }

    fn is_awg_duty_ramp_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.awg_duty_ramp.is_none()
    }

    // ------------

    fn on_awg_duty_trap(&mut self) {
        let _err = self.send_awg_duty_trap();
    }

    fn send_awg_duty_trap(&mut self) -> anyhow::Result<()> {
        let my_name = "on_awg_duty_trap()";
        if self.initializing {
            trace!("UI => {}, SKIPPED/INIT", my_name);
            return Ok(());
        } else if !self.is_connected() {
            self.message("not connected");
            error!("UI => {}, SKIPPED/NOT_CONNECTED", my_name);
            bail!("not connected");
        } else {
            trace!("UI => {}", my_name);
        }

        let duty = self.cfg.awg_duty_trap.as_ref().unwrap().clone();
        debug!("UI => {}::{}", my_name, duty);

        self.message(format!(
            "setting awg_duty_trap={}/{}/{}",
            duty.high, duty.low, duty.rise
        ));
        self.tx
            .send(DevCommand::AwgDutyTrap(duty.high, duty.low, duty.rise))
            .unwrap();

        match self.rx.recv().unwrap() {
            Ok(_) => {
                self.message("awg duty trap set");
                Ok(())
            }
            Err(error) => {
                self.message(error.clone());
                bail!(error);
            }
        }
    }

    fn get_awg_duty_trap_high(&self) -> f32 {
        self.cfg
            .awg_duty_trap
            .as_ref()
            .unwrap_or(&TrapDuty::ZERO)
            .clone()
            .high
    }

    fn get_awg_duty_trap_low(&self) -> f32 {
        self.cfg
            .awg_duty_trap
            .as_ref()
            .unwrap_or(&TrapDuty::ZERO)
            .clone()
            .low
    }

    fn get_awg_duty_trap_rise(&self) -> f32 {
        self.cfg
            .awg_duty_trap
            .as_ref()
            .unwrap_or(&TrapDuty::ZERO)
            .clone()
            .rise
    }

    fn set_awg_duty_trap_high(&mut self, new_value: f32) {
        let low = self.get_awg_duty_trap_low();
        let rise = self.get_awg_duty_trap_rise();
        let high = new_value;
        self.cfg.awg_duty_trap = Some(TrapDuty { high, low, rise });
    }

    fn set_awg_duty_trap_low(&mut self, new_value: f32) {
        let low = new_value;
        let rise = self.get_awg_duty_trap_rise();
        let high = self.get_awg_duty_trap_high();
        self.cfg.awg_duty_trap = Some(TrapDuty { high, low, rise });
    }

    fn set_awg_duty_trap_rise(&mut self, new_value: f32) {
        let low = self.get_awg_duty_trap_low();
        let rise = new_value;
        let high = self.get_awg_duty_trap_high();
        self.cfg.awg_duty_trap = Some(TrapDuty { high, low, rise });
    }

    fn is_awg_duty_trap_disabled(&self) -> bool {
        !self.is_connected() || self.cfg.awg_duty_trap.is_none()
    }
}

fn build_connect_panel() -> impl Widget<HantekState> {
    let connect_button = Button::new("Connect")
        .on_click(|_, state: &mut HantekState, _| state.connect())
        .disabled_if(|state: &HantekState, _| state.is_connected());

    let disconnect_button = Button::new("Disconnect")
        .on_click(|_, state: &mut HantekState, _| state.disconnect())
        .disabled_if(|state: &HantekState, _| !state.is_connected());

    Flex::row()
        .with_flex_child(connect_button, 1.0)
        .with_flex_spacer(0.2)
        .with_flex_child(disconnect_button, 1.0)
}

fn build_channel_panel(channel: usize) -> impl Widget<HantekState> {
    let enabled_switch = Switch::new()
        .lens(lens_of(
            move |state: &HantekState| state.get_enabled_channel(channel),
            move |state, new_value| state.set_enabled_channel(channel, new_value),
        ))
        .disabled_if(move |state: &HantekState, _| state.is_channel_enable_disabled(channel))
        .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_channel_enable(channel));
    let enabled = Flex::row()
        .with_flex_child(label("Enabled"), 1.0)
        .with_flex_child(enabled_switch, 1.0)
        .align_horizontal(UnitPoint::CENTER)
        .padding(5.0);

    let coupling_options = DropdownSelect::new(Vector::from(Coupling::my_options()))
        .lens(lens_of(
            move |state: &HantekState| state.get_coupling(channel),
            move |state: &mut HantekState, new_value| state.set_coupling(channel, new_value),
        ))
        .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_coupling(channel))
        .disabled_if(move |state: &HantekState, _| state.is_coupling_disabled(channel));
    let coupling = Flex::row()
        .with_flex_child(label("Coupling"), 1.0)
        .with_flex_child(coupling_options, 1.0);

    let probe_options = DropdownSelect::new(Vector::from(Probe::my_options()))
        .lens(lens_of(
            move |state: &HantekState| state.get_probe(channel),
            move |state, new_value| state.set_probe(channel, new_value),
        ))
        .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_probe(channel))
        .disabled_if(move |state: &HantekState, _| state.is_probe_disabled(channel));
    let probe = Flex::row()
        .with_flex_child(label("Probe"), 1.0)
        .with_flex_child(probe_options, 1.0);

    let scale_options = DropdownSelect::new(Vector::from(Scale::my_options()))
        .lens(lens_of(
            move |state: &HantekState| state.get_scale(channel),
            move |state, new_value| state.set_scale0(channel, new_value),
        ))
        .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_scale(channel))
        .disabled_if(move |state: &HantekState, _| state.is_scale_disabled(channel));
    let scale = Flex::row()
        .with_flex_child(label("Scale"), 1.0)
        .with_flex_child(scale_options, 1.0);

    let offset_input = float_text_unrestricted()
        .lens(lens_of(
            move |state: &HantekState| state.get_offset(channel),
            move |state: &mut HantekState, new_value| state.set_offset(channel, new_value),
        ))
        .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_offset(channel))
        .disabled_if(move |state: &HantekState, _| state.is_offset_disabled(channel));
    let offset = Flex::row()
        .with_flex_child(label("Offset"), 1.0)
        .with_flex_child(offset_input, 1.0);

    let bw_limit_switch = Switch::new()
        .lens(lens_of(
            move |state: &HantekState| state.get_bw_limit(channel),
            move |state: &mut HantekState, new_value| state.set_bw_limit(channel, new_value),
        ))
        .disabled_if(move |state: &HantekState, _| state.is_bw_limit_enable_disabled(channel))
        .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_bw_limit(channel));
    let bandwidth_limit = Flex::row()
        .with_flex_child(label("Bandwidth Limit"), 1.0)
        .with_flex_child(bw_limit_switch, 1.0)
        .align_horizontal(UnitPoint::CENTER)
        .padding(5.0);

    Flex::column()
        .with_flex_child(label_ct(format!("Channel {}", channel)), 1.0)
        .with_flex_child(enabled, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(coupling, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(probe, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(scale, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(offset, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(bandwidth_limit, 1.0)
        .with_flex_spacer(0.1)
}

fn build_scope_panel() -> impl Widget<HantekState> {
    let enabled = Flex::row()
        .with_flex_child(label("Running"), 1.0)
        .with_flex_child(
            Switch::new()
                .lens(lens_of(
                    |state: &HantekState| state.get_running(),
                    |state: &mut HantekState, new_value| state.set_running(new_value),
                ))
                .disabled_if(|state: &HantekState, _| state.is_running_disabled())
                .on_change(|_, _, data_mut: &mut HantekState, _| data_mut.on_running()),
            1.0,
        )
        .align_horizontal(UnitPoint::CENTER)
        .padding(5.0);

    let time_scale_options = DropdownSelect::new(Vector::from(TimeScale::my_options()))
        .lens(lens_of(
            |state: &HantekState| state.get_time_scale(),
            |state: &mut HantekState, new_value| state.set_time_scale(new_value),
        ))
        .disabled_if(|state: &HantekState, _| state.is_time_scale_disabled())
        .on_change(|_, _, data_mut: &mut HantekState, _| data_mut.on_time_scale());
    let time_scale = Flex::row()
        .with_flex_child(label("Time Scale"), 1.0)
        .with_flex_child(time_scale_options, 1.0);

    let time_offset = Flex::row()
        .with_flex_child(label("Time Offset"), 1.0)
        .with_flex_child(
            float_text_unrestricted()
                .lens(lens_of(
                    |state: &HantekState| state.get_time_offset(),
                    |state: &mut HantekState, new_value| state.set_time_offset(new_value),
                ))
                .disabled_if(|state: &HantekState, _| state.is_time_offset_disabled())
                .on_change(|_, _, data_mut: &mut HantekState, _| data_mut.on_time_offset()),
            1.0,
        );
    let trigger_source = Flex::row()
        .with_flex_child(label("Trigger Source"), 1.0)
        .with_flex_child(
            DropdownSelect::new(Vector::from(vec![("Channel 1", 1), ("Channel 2", 2)]))
                .lens(lens_of(
                    |state: &HantekState| state.get_trigger_source(),
                    |state: &mut HantekState, new_value| state.set_trigger_source(new_value),
                ))
                .disabled_if(|state: &HantekState, _| state.is_trigger_source_disabled())
                .on_change(|_, _, data_mut: &mut HantekState, _| data_mut.on_trigger_source()),
            1.0,
        );
    let trigger_mode = Flex::row()
        .with_flex_child(label("Trigger Mode"), 1.0)
        .with_flex_child(
            DropdownSelect::new(Vector::from(TriggerMode::my_options()))
                .lens(lens_of(
                    |state: &HantekState| state.get_trigger_mode(),
                    |state: &mut HantekState, new_value| state.set_trigger_mode(new_value),
                ))
                .disabled_if(|state: &HantekState, _| state.is_trigger_mode_disabled())
                .on_change(|_, _, data_mut: &mut HantekState, _| data_mut.on_trigger_mode()),
            1.0,
        );

    let trigger_level = Flex::row()
        .with_flex_child(label("Trigger Level"), 1.0)
        .with_flex_child(
            float_text_unrestricted()
                .lens(lens_of(
                    |state: &HantekState| state.get_trigger_level(),
                    |state: &mut HantekState, new_value| state.set_trigger_level(new_value),
                ))
                .disabled_if(|state: &HantekState, _| state.is_trigger_level_disabled())
                .on_change(|_, _, data_mut: &mut HantekState, _| data_mut.on_trigger_level()),
            1.0,
        );

    Flex::column()
        .with_flex_child(label_c("Scope"), 1.0)
        .with_flex_child(enabled, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(time_scale, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(time_offset, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(trigger_source, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(trigger_mode, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(trigger_level, 1.0)
        .with_flex_spacer(0.1)
}

fn build_awg_panel() -> impl Widget<HantekState> {
    let awg_running = Flex::row()
        .with_flex_child(label("Running"), 1.0)
        .with_flex_child(
            Switch::new()
                .lens(lens_of(
                    |state: &HantekState| state.get_awg_running().is_start(),
                    |state: &mut HantekState, new_value| {
                        state.set_awg_running(match new_value {
                            true => RunningStatus::Start,
                            false => RunningStatus::Stop,
                        })
                    },
                ))
                .disabled_if(|state: &HantekState, _| state.is_awg_running_disabled())
                .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_awg_running()),
            1.0,
        )
        .align_horizontal(UnitPoint::CENTER)
        .padding(5.0);

    let awg_type = Flex::row()
        .with_flex_child(label("AWG Type"), 1.0)
        .with_flex_child(
            DropdownSelect::new(Vector::from(AwgType::my_options()))
                .lens(lens_of(
                    |state: &HantekState| state.get_awg_type(),
                    |state: &mut HantekState, new_value| state.set_awg_type(new_value),
                ))
                .disabled_if(|state: &HantekState, _| state.is_awg_type_disabled())
                .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_awg_type()),
            1.0,
        );

    let frequency = Flex::row()
        .with_flex_child(label("Frequency"), 1.0)
        .with_flex_child(
            float_text_unrestricted()
                .lens(lens_of(
                    move |state: &HantekState| state.get_awg_frequency(),
                    move |state: &mut HantekState, new_value| state.set_awg_frequency(new_value),
                ))
                .disabled_if(|state: &HantekState, _| state.is_awg_frequency_disabled())
                .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_awg_frequency()),
            1.0,
        );

    let amplitude = Flex::row()
        .with_flex_child(label("Amplitude"), 1.0)
        .with_flex_child(
            float_text_unrestricted()
                .lens(lens_of(
                    move |state: &HantekState| state.get_awg_amplitude(),
                    move |state: &mut HantekState, new_value| state.set_awg_amplitude(new_value),
                ))
                .disabled_if(|state: &HantekState, _| state.is_awg_amplitude_disabled())
                .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_awg_amplitude()),
            1.0,
        );

    let offset = Flex::row()
        .with_flex_child(label("Offset"), 1.0)
        .with_flex_child(
            float_text_unrestricted()
                .lens(lens_of(
                    move |state: &HantekState| state.get_awg_offset(),
                    move |state: &mut HantekState, new_value| state.set_awg_offset(new_value),
                ))
                .disabled_if(|state: &HantekState, _| state.is_awg_offset_disabled())
                .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_awg_offset()),
            1.0,
        );

    let duty_square = Flex::row()
        .with_flex_child(label("Duty / Square"), 1.0)
        .with_flex_child(
            float_text_unrestricted()
                .lens(lens_of(
                    move |state: &HantekState| state.get_awg_duty_square(),
                    move |state: &mut HantekState, new_value| state.set_awg_duty_square(new_value),
                ))
                .disabled_if(|state: &HantekState, _| state.is_awg_duty_square_disabled())
                .on_change(move |_, _, data_mut: &mut HantekState, _| {
                    data_mut.on_awg_duty_square()
                }),
            1.0,
        );

    let duty_ramp = Flex::row()
        .with_flex_child(label("Duty / Ramp"), 1.0)
        .with_flex_child(
            float_text_unrestricted()
                .lens(lens_of(
                    move |state: &HantekState| state.get_awg_duty_ramp(),
                    move |state: &mut HantekState, new_value| state.set_awg_duty_ramp(new_value),
                ))
                .disabled_if(|state: &HantekState, _| state.is_awg_duty_ramp_disabled())
                .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_awg_duty_ramp()),
            1.0,
        );

    let duty_trap_high = Flex::row()
        .with_flex_child(label("Duty / Trap::High"), 1.0)
        .with_flex_child(
            float_text_unrestricted()
                .lens(lens_of(
                    move |state: &HantekState| state.get_awg_duty_trap_high(),
                    move |state: &mut HantekState, new_value| {
                        state.set_awg_duty_trap_high(new_value)
                    },
                ))
                .disabled_if(|state: &HantekState, _| state.is_awg_duty_trap_disabled())
                .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_awg_duty_trap()),
            1.0,
        );

    let duty_trap_low = Flex::row()
        .with_flex_child(label("Duty / Trap::Low"), 1.0)
        .with_flex_child(
            float_text_unrestricted()
                .lens(lens_of(
                    move |state: &HantekState| state.get_awg_duty_trap_low(),
                    move |state: &mut HantekState, new_value| {
                        state.set_awg_duty_trap_low(new_value)
                    },
                ))
                .disabled_if(|state: &HantekState, _| state.is_awg_duty_trap_disabled())
                .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_awg_duty_trap()),
            1.0,
        );

    let duty_trap_rise = Flex::row()
        .with_flex_child(label("Duty / Trap::Rise"), 1.0)
        .with_flex_child(
            float_text_unrestricted()
                .lens(lens_of(
                    move |state: &HantekState| state.get_awg_duty_trap_rise(),
                    move |state: &mut HantekState, new_value| {
                        state.set_awg_duty_trap_rise(new_value)
                    },
                ))
                .disabled_if(|state: &HantekState, _| state.is_awg_duty_trap_disabled())
                .on_change(move |_, _, data_mut: &mut HantekState, _| data_mut.on_awg_duty_trap()),
            1.0,
        );

    Flex::column()
        .with_flex_child(label_c("AWG"), 1.0)
        .with_flex_child(awg_running, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(awg_type, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(frequency, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(amplitude, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(offset, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(duty_square, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(duty_ramp, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(duty_trap_high, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(duty_trap_low, 1.0)
        .with_flex_spacer(0.1)
        .with_flex_child(duty_trap_rise, 1.0)
        .with_flex_spacer(0.1)
}

fn build_control_panel() -> impl Widget<HantekState> {
    Flex::column()
        .must_fill_main_axis(true)
        .with_flex_child(
            Flex::row()
                .with_flex_child(build_channel_panel(1), 1.0)
                .with_flex_child(build_channel_panel(2), 1.0),
            1.0,
        )
        .with_flex_child(
            Flex::row()
                .with_flex_child(
                    Flex::column()
                        .with_flex_child(build_scope_panel(), 3.0)
                        .with_flex_child(build_connect_panel(), 1.0)
                        .main_axis_alignment(MainAxisAlignment::End),
                    1.0,
                )
                .with_flex_child(build_awg_panel(), 1.0),
            1.4,
        )
}

fn build_scope_graph() -> impl Widget<HantekState> {
    ScopeGraph
}

fn build_ui() -> impl Widget<HantekState> {
    let messages = Label::new(|data: &String, _env: &_| data.clone())
        .lens(lens_of(
            |state: &HantekState| state.get_messages(),
            |_, _| {},
        ))
        .padding(5.0)
        .expand_width()
        .scroll()
        .vertical()
        .fix_height(100.0)
        .expand_width();

    Flex::column()
        .must_fill_main_axis(true)
        .with_flex_spacer(0.0)
        .with_child(messages)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_flex_child(
            Flex::row()
                .with_flex_child(build_scope_graph(), 2.5)
                .with_flex_spacer(0.1)
                .with_flex_child(build_control_panel(), 1.0),
            1.0,
        )
}

pub fn main() -> anyhow::Result<()> {
    let mut builder = formatted_builder();
    builder.parse_filters("TRACE");
    builder.init();

    info!("running handler thread");
    let (rx, tx) = handler_thread();

    debug!("creating UI window");
    let window = WindowDesc::new(
        build_ui(),
        // .debug_paint_layout()
    )
    .with_min_size((1024., 740.))
    .window_size((1724., 740.))
    .title(tt("window-title", "Hantek"));

    let state = HantekState::new(rx, Arc::new(tx));

    info!("launching UI");
    AppLauncher::with_window(window)
        .log_to_console()
        .launch(state)?;

    Ok(())
}
