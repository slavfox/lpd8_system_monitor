// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2021, Slavfox.
use crate::utility::Pad::Pad4;
use sysinfo::{
    ComponentExt, NetworkExt, NetworksExt, ProcessExt, ProcessorExt, System,
    SystemExt,
};

pub enum Pad {
    Pad1,
    Pad2,
    Pad3,
    Pad4,
    Pad5,
    Pad6,
    Pad7,
    Pad8,
}

pub fn pad_to_midi_note(pad: &Pad) -> u8 {
    match pad {
        Pad::Pad1 => 0x24,
        Pad::Pad2 => 0x25,
        Pad::Pad3 => 0x26,
        Pad::Pad4 => 0x27,
        Pad::Pad5 => 0x28,
        Pad::Pad6 => 0x29,
        Pad::Pad7 => 0x2A,
        Pad::Pad8 => 0x2B,
    }
}

pub fn note_on(pad: &Pad) -> [u8; 3] { [0x90, pad_to_midi_note(pad), 0x7F] }

pub fn note_off(pad: &Pad) -> [u8; 3] { [0x80, pad_to_midi_note(pad), 0x7F] }

pub fn get_core_usage_percent(system: &mut System) -> Vec<f32> {
    system
        .get_processors()
        .iter()
        .map(|proc| proc.get_cpu_usage() / 100.0)
        .collect()
}

pub fn get_memory_usage_percent(system: &mut System) -> f32 {
    (system.get_used_memory() as f32) / (system.get_total_memory() as f32)
}

pub fn get_network_transmitted_percent(system: &mut System) -> f32 {
    let networks = system.get_networks();
    for (_, network) in networks {
        if network.get_transmitted() > 0 {
            return 1.0;
        }
    }
    0.0
}

pub fn get_cpu_temperature_percent(system: &mut System) -> f32 {
    let components = system.get_components();
    components
        .iter()
        .find(|cmp| cmp.get_label().contains("Package id"))
        .unwrap_or(components.first().unwrap())
        .get_temperature()
        / 100.0
}
