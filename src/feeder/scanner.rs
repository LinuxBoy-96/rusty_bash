//SPDX-FileCopyrightText: 2023 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use super::Feeder;
use crate::ShellCore;

impl Feeder {
    fn feed_and_connect(&mut self, core: &mut ShellCore) {
        self.remaining.pop();
        self.remaining.pop();
        let _ = self.feed_additional_line_core(core);
    }

    fn backslash_check_and_feed(&mut self, starts: Vec<&str>, core: &mut ShellCore) {
        let check = |s: &str| self.remaining.starts_with(&(s.to_owned() + "\\\n"));
        if starts.iter().any(|s| check(s)) {
            self.feed_and_connect(core);
        }
    }

    fn scanner_chars(&mut self, judge: fn(char) -> bool,
                     core: &mut ShellCore) -> usize {
        loop {
            let mut ans = 0;
            for ch in self.remaining.chars() {
                if judge(ch) {
                    ans += ch.len_utf8();
                } else {
                    break;
                }
            }

            if &self.remaining[ans..] == "\\\n" {
                self.feed_and_connect(core);
            }else{
                return ans;
            }
        }
    }

    fn scanner_one_of(&self, cands: &[&str]) -> usize {
        for c in cands {
            if self.starts_with(c) {
                return c.len();
            }
        }
        0
    }

    pub fn scanner_subword_symbol(&self) -> usize {
        self.scanner_one_of(&["{", "}", ",", "$", "~", "/"])
    }

    pub fn scanner_escaped_char(&mut self, core: &mut ShellCore) -> usize {
        if self.starts_with("\\\n") {
            self.feed_and_connect(core);
        }

        if ! self.starts_with("\\") {
            return 0;
        }

        match self.remaining.chars().nth(1) {
            Some(ch) => 1 + ch.len_utf8(),
            None =>     1,
        }
    }

    pub fn scanner_dollar_special_and_positional_param(&mut self, core: &mut ShellCore) -> usize {
        if ! self.starts_with("$") {
            return 0;
        }
        self.backslash_check_and_feed(vec!["$"], core);

        match self.remaining.chars().nth(1) {
            Some(c) => match "$?*@#-!_0123456789".find(c) {
                Some(_) => 2,
                None    => 0,
            },
            None    => 0,
        }
    }

    pub fn scanner_subword(&mut self) -> usize {
        let mut ans = 0;
        for ch in self.remaining.chars() {
            if " \t\n;&|()<>{},\\'$/~".find(ch) != None {
                break;
            }
            ans += ch.len_utf8();
        }
        ans
    }

    pub fn scanner_single_quoted_subword(&mut self, core: &mut ShellCore) -> usize {
        if ! self.starts_with("'") {
            return 0;
        }
        if self.starts_with("''") {
            return 2;
        }

        loop {
            if let Some(n) = self.remaining[1..].find("'") {
                return n + 2;
            }else if ! self.feed_additional_line(core) {
                return 0;
            }
        }
    }

    pub fn scanner_blank(&mut self, core: &mut ShellCore) -> usize {
        let judge = |ch| " \t".find(ch).is_some();
        self.scanner_chars(judge, core)
    }

    pub fn scanner_multiline_blank(&mut self, core: &mut ShellCore) -> usize {
        let judge = |ch| " \t\n".find(ch).is_some();
        self.scanner_chars(judge, core)
    }

    pub fn scanner_nonnegative_integer(&mut self, core: &mut ShellCore) -> usize {
        let judge = |ch: char| ch.is_ascii_digit();
        self.scanner_chars(judge, core)
    }

    pub fn scanner_name(&mut self, core: &mut ShellCore) -> usize {
        let head = self.remaining.chars().nth(0).unwrap_or('0');
        if head.is_ascii_digit() {
            return 0;
        }
    
        let judge = |ch: char| ch == '_' || ch.is_ascii_digit()
                         || ch.is_ascii_lowercase()
                         || ch.is_ascii_uppercase();
        self.scanner_chars(judge, core)
    }

    pub fn scanner_job_end(&mut self) -> usize {
        self.scanner_one_of(&[";", "&", "\n"])
    }

    pub fn scanner_and_or(&mut self, core: &mut ShellCore) -> usize {
        self.backslash_check_and_feed(vec!["|", "&"], core);
        self.scanner_one_of(&["||", "&&"])
    }

    pub fn scanner_pipe(&mut self, core: &mut ShellCore) -> usize {
        self.backslash_check_and_feed(vec!["|"], core);
        if self.starts_with("||") {
            return 0;
        }
        self.scanner_one_of(&["|&","|"])
    }

    pub fn scanner_comment(&self) -> usize {
        if ! self.remaining.starts_with("#") {
            return 0;
        }

        let mut ans = 0;
        for ch in self.remaining.chars() {
            if "\n".find(ch).is_some() {
                break;
            }
            ans += ch.len_utf8();
        }
        ans
    }

    pub fn scanner_redirect_symbol(&mut self, core: &mut ShellCore) -> usize {
        self.backslash_check_and_feed(vec![">", "&"], core);
        self.scanner_one_of(&["&>", ">&", ">>", "<", ">"])
    }
}
