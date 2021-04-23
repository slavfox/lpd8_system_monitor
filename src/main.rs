// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright 2021, Slavfox.
#![warn(clippy::pedantic, clippy::nursery)]

mod utility;

use std::sync::mpsc;
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

use midir::{ConnectError, MidiOutput, MidiOutputConnection};
use sysinfo::{RefreshKind, System, SystemExt};

use utility::{note_off, note_on, Pad, Resources};

macro_rules! pad_worker {
    ($threads:expr, $src:ident, $pad:expr) => {
        $threads.push(spawn(move || {
            let mut connection = connect(stringify!($src)).unwrap();
            let mut duty_cycle = 0f32;
            loop {
                match $src.try_recv() {
                    Ok(val) => {
                        duty_cycle = val;
                    }
                    Err(_) => {}
                }
                pwm(&mut connection, duty_cycle, $pad);
            }
        }))
    };
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
    let on_time = (DURATION as f32 * duty_cycle * duty_cycle) as u64;
    sleep(Duration::from_millis(on_time));
    if duty_cycle < 1.0 {
        conn.send(&note_off(&pad)).unwrap();
        sleep(Duration::from_millis(DURATION - on_time));
    }
}
const DURATION: u64 = 20;
const REFRESH_INTERVAL: u64 = 100;

fn main() {
    let (cpu_usage_tx, cpu_usage_rx) = mpsc::channel();
    let (cpu_temp_tx, cpu_temp_rx) = mpsc::channel();
    let (memory_usage_tx, memory_usage_rx) = mpsc::channel();
    let (network_sent_tx, network_sent_rx) = mpsc::channel();
    let (network_received_tx, network_received_rx) = mpsc::channel();
    let (network_sent_errors_tx, network_sent_errors_rx) = mpsc::channel();
    let (network_received_errors_tx, network_received_errors_rx) =
        mpsc::channel();
    let (clock_tx, clock_rx) = mpsc::channel();

    let mut threads: Vec<JoinHandle<()>> = vec![];
    pad_worker!(threads, cpu_usage_rx, Pad::Pad1);
    pad_worker!(threads, cpu_temp_rx, Pad::Pad2);
    pad_worker!(threads, memory_usage_rx, Pad::Pad3);
    pad_worker!(threads, clock_rx, Pad::Pad4);
    pad_worker!(threads, network_sent_rx, Pad::Pad5);
    pad_worker!(threads, network_received_rx, Pad::Pad6);
    pad_worker!(threads, network_sent_errors_rx, Pad::Pad7);
    pad_worker!(threads, network_received_errors_rx, Pad::Pad8);
    threads.push(spawn(move || loop {
        clock_tx.send(1.0).unwrap();
        clock_tx.send(0.0).unwrap();
        sleep(Duration::from_secs(1));
    }));
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
            cpu_usage_tx
                .send(system.get_cpu_usage_percent() / 100.0)
                .unwrap();
            cpu_temp_tx
                .send(system.get_cpu_temperature_percent())
                .unwrap();
            memory_usage_tx
                .send(system.get_memory_usage_percent())
                .unwrap();
            network_sent_tx
                .send(system.get_network_transmitted_percent())
                .unwrap();
            network_received_tx
                .send(system.get_network_received_percent())
                .unwrap();
            network_sent_errors_tx
                .send(system.get_network_transmitted_error_percent())
                .unwrap();
            network_received_errors_tx
                .send(system.get_network_received_error_percent())
                .unwrap();
            sleep(Duration::from_millis(REFRESH_INTERVAL));
        }
    }));
    for thread in threads {
        thread.join().unwrap();
    }
}
