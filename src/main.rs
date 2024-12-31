//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

mod core;
mod feeder;
mod elements;
mod signal;
mod utils;

use std::{env, process};
use crate::core::ShellCore;
use crate::utils::exit;
use crate::elements::script::Script;
use crate::feeder::{Feeder, InputError};
use utils::file_check;
use std::sync::atomic::Ordering::Relaxed;

fn show_version() {
    eprintln!("Sushi Shell 202305_5");
    eprintln!("© 2023 Ryuichi Ueda");
    eprintln!("License: BSD 3-Clause\n");

    eprintln!("This is open source software. You can redistirbute and use in source\nand binary forms with or without modification under the license.");
    eprintln!("There is no warranty, to the extent permitted by law.");
    process::exit(0);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "--version" {
        show_version();
    }

    let mut core = ShellCore::new();
    signal::run_signal_check(&mut core);
    main_loop(&mut core);
}

fn input_interrupt_check(feeder: &mut Feeder, core: &mut ShellCore) -> bool {
    if ! core.sigint.load(Relaxed) { //core.input_interrupt {
        return false;
    }

    core.sigint.store(false, Relaxed); //core.input_interrupt = false;
    core.data.set_param("?", "130");
    feeder.consume(feeder.len());
    true
}

fn main_loop(core: &mut ShellCore) {
    let mut feeder = Feeder::new();
    loop {
        core.jobtable_check_status();
        core.jobtable_print_status_change();

        match feeder.feed_line(core) {
            Ok(()) => {}, 
            Err(InputError::Interrupt) => {
                input_interrupt_check(&mut feeder, core);
                continue;
            },
            _ => break,
        }

        match Script::parse(&mut feeder, core){
            Some(mut s) => s.exec(core),
            None => {},
        }
        core.sigint.store(false, Relaxed);
    }

    exit::normal(core)
}
