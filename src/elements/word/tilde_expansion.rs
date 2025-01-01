//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::ShellCore;
use crate::elements::word::Word;
use nix::unistd::User;
use super::subword::simple::SimpleSubword;

pub fn eval(word: &mut Word, core: &mut ShellCore) {
    let length = match prefix_length(word) {
        0 => return,
        n => n,
    };

    let text: String = word.subwords[1..length].iter()
               .map(|e| e.get_text().to_string())
               .collect::<Vec<String>>()
               .concat();

    let value = get_value(&text, core);
    if value.is_empty() {
        return;
    }
    word.subwords[0] = Box::new( SimpleSubword{ text: value } );
    word.subwords[1..length].iter_mut().for_each(|w| w.set_text(""));
}

fn prefix_length(word: &Word) -> usize {
    if word.subwords.is_empty() || word.subwords[0].get_text() != "~" {
        return 0;
    }

    match word.subwords.iter().position(|e| e.get_text() == "/") {
        None    => word.subwords.len(),
        Some(n) => n,
    }
}

fn get_value(text: &str, core: &mut ShellCore) -> String {
    let key = match text {
        "" => "HOME",
        "+" => "PWD",
        "-" => "OLDPWD",
        _ => return get_home_dir(text),
    };

    core.data.get_param(key).to_string()
}

fn get_home_dir(user: &str) -> String {
    match User::from_name(user) {
        Ok(Some(u)) => u.dir
                        .into_os_string()
                        .into_string()
                        .unwrap(),
        _ => String::new(),
    }
}
