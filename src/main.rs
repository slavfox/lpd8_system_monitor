// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2021, Slavfox.
#![warn(clippy::pedantic, clippy::nursery)]

mod utility;

use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

use midir::{ConnectError, MidiOutput, MidiOutputConnection};
use sysinfo::{RefreshKind, System, SystemExt};

use utility::{
    get_core_usage_percent, get_cpu_temperature_percent,
    get_memory_usage_percent, get_network_transmitted_percent, note_off,
    note_on, Pad,
};

macro_rules! pad_worker {
    ($threads:expr, $src:ident, $pad:expr) => {{
        let $src = Arc::clone(&$src);
        $threads.push(spawn(move || {
            let mut connection = connect(stringify!($src)).unwrap();
            loop {
                let duty_cycle = $src.lock().unwrap().clone();
                pwm(&mut connection, duty_cycle, $pad);
            }
        }))
    }};
}

fn connect(
    client_name: &str,
) -> Result<MidiOutputConnection, ConnectError<MidiOutput>> {
    let midi_out = MidiOutput::new(client_name)
        .expect("Failed to initialize MIDI output!");
    midi_out
        .ports()
        .iter()
        .find(|&port| match midi_out.port_name(port) {
            Ok(name) => name.contains("LPD8"),
            Err(_) => false,
        })
        .and_then(|port| {
            println!(
                "Connecting to port {}.",
                midi_out.port_name(port).expect(
                    "Port disappeared before a connection could be \
                     established."
                )
            );
            Some(midi_out.connect(port, client_name))
        })
        .expect("Could not connect to LPD8")
}

fn pwm(conn: &mut MidiOutputConnection, duty_cycle: f32, pad: Pad) {
    if duty_cycle > 0.0 {
        conn.send(&note_on(&pad)).unwrap();
    }
    let on_time = (DURATION as f32 * duty_cycle) as u64;
    sleep(Duration::from_millis(on_time));
    if duty_cycle < 1.0 {
        conn.send(&note_off(&pad)).unwrap();
        sleep(Duration::from_millis(DURATION - on_time));
    }
}
const DURATION: u64 = 10;
const REFRESH_INTERVAL: u64 = 40;

fn main() {
    let cpu_usage = Arc::new(Mutex::new(0f32));
    let core1_usage = Arc::new(Mutex::new(0f32));
    let core2_usage = Arc::new(Mutex::new(0f32));
    let core3_usage = Arc::new(Mutex::new(0f32));
    let core4_usage = Arc::new(Mutex::new(0f32));
    let cpu_temp = Arc::new(Mutex::new(0f32));
    let memory_usage = Arc::new(Mutex::new(0f32));
    let network_usage = Arc::new(Mutex::new(0f32));

    let mut threads: Vec<JoinHandle<()>> = vec![];
    pad_worker!(threads, cpu_usage, Pad::Pad1);
    pad_worker!(threads, cpu_temp, Pad::Pad2);
    pad_worker!(threads, memory_usage, Pad::Pad3);
    pad_worker!(threads, network_usage, Pad::Pad4);
    pad_worker!(threads, core1_usage, Pad::Pad5);
    pad_worker!(threads, core2_usage, Pad::Pad6);
    pad_worker!(threads, core3_usage, Pad::Pad7);
    pad_worker!(threads, core4_usage, Pad::Pad8);
    {
        let cpu_usage = Arc::clone(&cpu_usage);
        let core1_usage = Arc::clone(&core1_usage);
        let core2_usage = Arc::clone(&core2_usage);
        let core3_usage = Arc::clone(&core3_usage);
        let core4_usage = Arc::clone(&core4_usage);
        let cpu_temp = Arc::clone(&cpu_temp);
        let memory_usage = Arc::clone(&memory_usage);
        let network_usage = Arc::clone(&network_usage);
        threads.push(spawn(move || {
            let mut system = System::new_with_specifics(
                RefreshKind::everything()
                    .without_disks()
                    .without_disks_list()
                    .without_processes()
                    .without_users_list(),
            );
            loop {
                system.refresh_all();
                {
                    let cpu_usages = get_core_usage_percent(&mut system);
                    {
                        let mut core1_usage = core1_usage.lock().unwrap();
                        *core1_usage = cpu_usages[0];
                    }
                    {
                        let mut core2_usage = core2_usage.lock().unwrap();
                        *core2_usage = cpu_usages[1];
                    }
                    {
                        let mut core3_usage = core3_usage.lock().unwrap();
                        *core3_usage = cpu_usages[2];
                    }
                    {
                        let mut core4_usage = core4_usage.lock().unwrap();
                        *core4_usage = cpu_usages[3];
                    }

                    {
                        let mut cpu_usage = cpu_usage.lock().unwrap();
                        *cpu_usage = cpu_usages.iter().sum::<f32>()
                            / cpu_usages.len() as f32;
                    }
                }
                {
                    let mut cpu_temp = cpu_temp.lock().unwrap();
                    *cpu_temp = get_cpu_temperature_percent(&mut system);
                }
                {
                    let mut memory_usage = memory_usage.lock().unwrap();
                    *memory_usage = get_memory_usage_percent(&mut system);
                }
                {
                    let mut network_usage = network_usage.lock().unwrap();
                    *network_usage =
                        get_network_transmitted_percent(&mut system);
                }
                sleep(Duration::from_millis(REFRESH_INTERVAL));
            }
        }))
    }
    for thread in threads {
        thread.join().unwrap();
    }
}
