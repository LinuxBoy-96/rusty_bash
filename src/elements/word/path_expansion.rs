//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::elements::word::Word;
use crate::utils::glob;

pub fn eval(word: &mut Word) -> Vec<Word> {
    let paths = expand(&word.make_glob_string());
    vec![word.clone()]
}

fn expand(pattern: &str) -> Vec<String> {
    if "*?@+![".chars().all(|c| ! pattern.contains(c)) {
        return vec![];
    }

    directory::glob("", patttern)
}
