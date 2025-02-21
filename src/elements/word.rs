//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

mod brace_expansion;
pub mod tilde_expansion;
pub mod substitution;
mod path_expansion;
mod split;

use crate::{ShellCore, Feeder};
use crate::elements::subword;
use crate::error::parse::ParseError;
use crate::error::exec::ExecError;
use super::subword::Subword;
use super::subword::simple::SimpleSubword;

#[derive(Debug, Clone, Default)]
pub struct Word {
    pub text: String,
    pub subwords: Vec<Box<dyn Subword>>,
}

impl From<&String> for Word {
    fn from(s: &String) -> Self {
        Self {
            text: s.to_string(),
            subwords: vec![Box::new(SimpleSubword{text: s.to_string() })],
        }
    }
}

impl From<Box::<dyn Subword>> for Word {
    fn from(subword: Box::<dyn Subword>) -> Self {
        Self {
            text: subword.get_text().to_string(),
            subwords: vec![subword],
        }
    }
}

impl From<Vec<Box::<dyn Subword>>> for Word {
    fn from(subwords: Vec<Box::<dyn Subword>>) -> Self {
        Self {
            text: subwords.iter().map(|s| s.get_text()).collect(),
            subwords: subwords,
        }
    }
}

impl Word {
    pub fn eval(&mut self, core: &mut ShellCore) -> Result<Vec<String>, ExecError> {
        let ws_after_brace_exp = match core.db.flags.contains('B') {
            true  => brace_expansion::eval(&mut self.clone()),
            false => vec![self.clone()],
        };

        let mut ws = vec![];
        for w in ws_after_brace_exp {
            let expanded = w.tilde_and_dollar_expansion(core)?;
            ws.append( &mut expanded.split_and_path_expansion(core) );
        }
        Ok( Self::make_args(&mut ws) )
    }

    pub fn eval_as_value(&self, core: &mut ShellCore) -> Result<String, ExecError> {
        let mut ws = match self.tilde_and_dollar_expansion(core) {
            Ok(w)  => w.path_expansion(core),
            Err(e) => return Err(e),
        };

        Ok( Self::make_args(&mut ws).join(" ") )
    }

    pub fn eval_for_case_word(&self, core: &mut ShellCore) -> Option<String> {
        match self.tilde_and_dollar_expansion(core) {
            Ok(mut w) => w.make_unquoted_word(),
            Err(e)    => {
                e.print(core);
                return None;
            },
        }
    }

    pub fn eval_for_regex(&self, core: &mut ShellCore) -> Option<String> {
        match self.tilde_and_dollar_expansion(core) {
            Ok(mut w) => w.make_regex(),
            Err(e)    => {
                e.print(core);
                return None;
            },
        }
    }

    pub fn eval_for_case_pattern(&self, core: &mut ShellCore) -> Option<String> {
        match self.tilde_and_dollar_expansion(core) {
            Ok(mut w) => Some(w.make_glob_string()),
            Err(e)    => {
                e.print(core);
                return None;
            },
        }
    }

    pub fn tilde_and_dollar_expansion(&self, core: &mut ShellCore) -> Result<Word, ExecError> {
        let mut w = self.clone();
        tilde_expansion::eval(&mut w, core);
        substitution::eval(&mut w, core)?;
        Ok(w)
    }

    pub fn split_and_path_expansion(&self, core: &mut ShellCore) -> Vec<Word> {
        let mut ans = vec![];
        let extglob = core.shopts.query("extglob");

        let splitted = split::eval(self, core);
        if core.options.query("noglob") {
            return splitted;
        }

        for mut w in splitted {
            ans.append(&mut path_expansion::eval(&mut w, extglob) );
        }
        ans
    }

    pub fn path_expansion(&self, core: &mut ShellCore) -> Vec<Word> {
        let mut ans = vec![];
        let extglob = core.shopts.query("extglob");

        let splitted = vec![self.clone()];
        if core.options.query("noglob") {
            return splitted;
        }

        for mut w in splitted {
            ans.append(&mut path_expansion::eval(&mut w, extglob) );
        }
        ans
    }

    fn make_args(words: &mut Vec<Word>) -> Vec<String> {
        words.iter_mut()
              .map(|w| w.make_unquoted_word())
              .filter(|w| *w != None)
              .map(|w| w.unwrap())
              .collect()
    }

    pub fn make_unquoted_word(&mut self) -> Option<String> {
        let sw: Vec<Option<String>> = self.subwords.iter_mut()
            .map(|s| s.make_unquoted_string())
            .filter(|s| *s != None)
            .collect();

        if sw.is_empty() {
            return None;
        }

        Some(sw.into_iter().map(|s| s.unwrap()).collect::<String>())
    }

    pub fn make_regex(&mut self) -> Option<String> {
        let sw: Vec<Option<String>> = self.subwords.iter_mut()
            .map(|s| s.make_regex())
            .filter(|s| *s != None)
            .collect();

        if sw.is_empty() {
            return None;
        }

        Some(sw.into_iter().map(|s| s.unwrap()).collect::<String>())
    }

    fn make_glob_string(&mut self) -> String {
        self.subwords.iter_mut()
            .map(|s| s.make_glob_string())
            .collect::<Vec<String>>()
            .concat()
    }

    fn scan_pos(&self, s: &str) -> Vec<usize> {
        self.subwords.iter()
            .enumerate()
            .filter(|e| e.1.get_text() == s)
            .map(|e| e.0)
            .collect()
    }

    fn push(&mut self, subword: &Box<dyn Subword>) {
        self.text += &subword.get_text().to_string();
        self.subwords.push(subword.clone());
    }

    pub fn parse(feeder: &mut Feeder, core: &mut ShellCore, as_operand: bool)
        -> Result<Option<Word>, ParseError> {
        if feeder.starts_with("#") {
            return Ok(None);
        }
        if as_operand && feeder.starts_with("}") {
            return Ok(None);
        }

        let mut ans = Word::default();
        while let Some(sw) = subword::parse(feeder, core)? {
            match sw.is_extglob() {
                false => ans.push(&sw),
                true  => {
                    let mut sws = sw.get_child_subwords();
                    ans.subwords.append(&mut sws);
                },
            }

            if as_operand {
                if feeder.starts_with("]")
                || feeder.starts_with("}")
                || feeder.scanner_math_symbol(core) != 0 {
                    break;
                }
            }
        }

        match ans.subwords.len() {
            0 => Ok(None),
            _ => Ok(Some(ans)),
        }
    }
}
