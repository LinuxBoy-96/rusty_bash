//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

mod value_check;
mod substr;
mod remove;
mod replace;

use crate::{ShellCore, Feeder};
use crate::elements::subword;
use crate::elements::subword::Subword;
use crate::elements::subscript::Subscript;
use crate::elements::word::Word;
use crate::utils;
use crate::error::parse::ParseError;
use crate::error::exec::ExecError;
use self::remove::Remove;
use self::replace::Replace;
use self::substr::Substr;
use self::value_check::ValueCheck;
use super::filler::FillerSubword;

#[derive(Debug, Clone, Default)]
struct Param {
    name: String,
    subscript: Option<Subscript>,
}

#[derive(Debug, Clone, Default)]
pub struct BracedParam {
    text: String,
    array: Vec<String>,

    param: Param,
    replace: Option<Replace>,
    substr: Option<Substr>,
    remove: Option<Remove>,
    value_check: Option<ValueCheck>,

    unknown: String,
    is_array: bool,
    num: bool,
    indirect: bool,
}

impl Subword for BracedParam {
    fn get_text(&self) -> &str { &self.text.as_ref() }
    fn boxed_clone(&self) -> Box<dyn Subword> {Box::new(self.clone())}

    fn substitute(&mut self, core: &mut ShellCore) -> Result<(), ExecError> {
        self.check()?;

        if self.indirect {
            if let Some(sub) = &self.param.subscript {
                if sub.text == "[*]" || sub.text == "[@]" {
                    if self.replace.is_some() 
                    || self.substr.is_some()
                    || self.remove.is_some()
                    || self.value_check.is_some() {
                        let msg = core.db.get_array_all(&self.param.name).join(" ");
                        return Err(ExecError::InvalidName(msg));
                    }

                    return self.index_replace(core);
                }
            }
            self.indirect_replace(core)?;
        }

        if self.param.subscript.is_some() {
            if self.param.name == "@" {
                return Err(ExecError::BadSubstitution("@".to_string()));
            }
            return self.subscript_operation(core);
        }

        if self.param.name == "@" {
            if let Some(s) = self.substr.as_mut() {
                return s.set_partial_position_params(&mut self.array, &mut self.text, core);
            }
        }

        let value = core.db.get_param(&self.param.name).unwrap_or_default();
        self.text = match self.num {
            true  => value.chars().count().to_string(),
            false => value.to_string(),
        };

        self.optional_operation(core)
    }

    fn set_text(&mut self, text: &str) { self.text = text.to_string(); }

    fn get_alternative_subwords(&self) -> Vec<Box<dyn Subword>> {
        if self.value_check.is_none() {
            return vec![];
        }

        let check = self.value_check.clone().unwrap();
        match &check.alternative_value {
            Some(w) => w.subwords.to_vec(),
            None    => vec![],
        }
    }

    fn is_array(&self) -> bool {self.is_array && ! self.num}
    fn get_array_elem(&self) -> Vec<String> {self.array.clone()}
}

impl BracedParam {
    fn check(&mut self) -> Result<(), ExecError> {
        if self.param.name.is_empty() || ! utils::is_param(&self.param.name) {
            return Err(ExecError::BadSubstitution(self.text.clone()));
        }
        if self.unknown.len() > 0 
        && ! self.unknown.starts_with(",") {
            return Err(ExecError::BadSubstitution(self.text.clone()));
        }
        Ok(())
    }

    fn index_replace(&mut self, core: &mut ShellCore) -> Result<(), ExecError> {
        if ! core.db.has_value(&self.param.name) {
            self.text = "".to_string();
            return Ok(());
        }

        if ! core.db.is_array(&self.param.name) && ! core.db.is_assoc(&self.param.name) {
            self.text = "0".to_string();
            return Ok(());
        }

        self.array = core.db.get_indexes_all(&self.param.name);
        self.text = self.array.join(" ");

        Ok(())
    }

    fn indirect_replace(&mut self, core: &mut ShellCore) -> Result<(), ExecError> {
        let mut sw = self.clone();
        sw.indirect = false;
        sw.replace = None;
        sw.substr = None;
        sw.remove = None;
        sw.value_check = None;
        sw.unknown = String::new();
        sw.is_array = false;
        sw.num = false;

        sw.substitute(core)?;

        if sw.text.contains('[') {
            let mut feeder = Feeder::new(&("${".to_owned() + &sw.text + "}" ));
            if let Ok(Some(mut bp)) = BracedParam::parse(&mut feeder, core) {
                bp.substitute(core)?;
                self.param.name = bp.param.name;
                self.param.subscript = bp.param.subscript;
            }else{
                return Err(ExecError::InvalidName(sw.text.clone()));
            }
        }else{
            self.param.name = sw.text.clone();
            self.param.subscript = None;
        }

        if ! utils::is_param(&self.param.name) {
            return Err(ExecError::InvalidName(self.param.name.clone()));
        }
        Ok(())
    }

    fn subscript_operation(&mut self, core: &mut ShellCore) -> Result<(), ExecError> {
        if ! core.db.is_array(&self.param.name) && ! core.db.is_assoc(&self.param.name) {
            self.text = "".to_string();
            return Ok(());
        }

        let index = self.param.subscript.clone().unwrap().eval(core, &self.param.name)?;

        if core.db.is_assoc(&self.param.name) {
            return self.subscript_operation_assoc(core, &index);
        }

        if index.as_str() == "@" {
            self.array = core.db.get_array_all(&self.param.name);
        }

        self.text = match (self.num, index.as_str()) {
            (true, "@") => core.db.len(&self.param.name).to_string(),
            (true, _)   => core.db.get_array_elem(&self.param.name, &index).unwrap().chars().count().to_string(),
            (false, _)  => core.db.get_array_elem(&self.param.name, &index).unwrap(),
       };

       self.optional_operation(core)
    }

    fn subscript_operation_assoc(&mut self, core: &mut ShellCore, index: &str) -> Result<(), ExecError> {
        let s = core.db.get_array_elem(&self.param.name, index)?;
        self.text = s;
        Ok(())
    }

    fn optional_operation(&mut self, core: &mut ShellCore) -> Result<(), ExecError> {
        self.text = if let Some(s) = self.substr.as_mut() {
            s.get_text(&self.text, core)?
        }else if let Some(v) = self.value_check.as_mut() {
            v.set(&self.param.name, &self.param.subscript, &self.text, core)?
        }else if let Some(r) = self.remove.as_mut() {
            r.set(&mut self.text, core)?
        }else if let Some(r) = &self.replace {
            r.get_text(&self.text, core)?
        }else{
            self.text.clone()
        };

        Ok(())
    }

    fn eat_subscript(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> Result<bool, ParseError> {
        if let Some(s) = Subscript::parse(feeder, core)? {
            ans.text += &s.text;
            if s.text.contains('@') {
                ans.is_array = true;
            }
            ans.param.subscript = Some(s);
            return Ok(true);
        }

        Ok(false)
    }

    fn eat_subwords(feeder: &mut Feeder, ans: &mut Self, ends: Vec<&str>, core: &mut ShellCore)
        -> Result<Word, ParseError> {
        let mut word = Word::default();
        while ! ends.iter().any(|e| feeder.starts_with(e)) {
            if let Some(sw) = subword::parse_filler(feeder, core)? {
                ans.text += sw.get_text();
                word.text += sw.get_text();
                word.subwords.push(sw);
            }else{
                let c = feeder.consume(1);
                ans.text += &c;
                word.text += &c;
                word.subwords.push(Box::new(FillerSubword{text: c}) );
            }

            if feeder.len() == 0 {
                feeder.feed_additional_line(core)?;
            }
        }

        Ok(word)
    }

    fn eat_param(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        let len = feeder.scanner_name(core);
        if len != 0 {
            ans.param = Param{ name: feeder.consume(len), subscript: None};
            ans.text += &ans.param.name;
            return true;
        }

        let len = feeder.scanner_special_and_positional_param();
        if len != 0 {
            ans.param = Param {name: feeder.consume(len), subscript: None};
            ans.is_array = ans.param.name == "@";
            ans.text += &ans.param.name;
            return true;
        }

        feeder.starts_with("}")
    }

    fn eat_unknown(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> Result<(), ParseError> {
        if feeder.len() == 0 {
            feeder.feed_additional_line(core)?;
        }

        let unknown = match feeder.starts_with("\\}") {
            true  => feeder.consume(2),
            false => {
                let len = feeder.nth(0).unwrap().len_utf8();
                feeder.consume(len)
            },
        };

        ans.unknown += &unknown.clone();
        ans.text += &unknown;
        Ok(())
    }

    pub fn parse(feeder: &mut Feeder, core: &mut ShellCore) -> Result<Option<Self>, ParseError> {
        if ! feeder.starts_with("${") {
            return Ok(None);
        }
        let mut ans = Self::default();
        ans.text += &feeder.consume(2);

        if feeder.starts_with("#") && ! feeder.starts_with("#}") {
            ans.num = true;
            ans.text += &feeder.consume(1);
        }else if feeder.starts_with("!") {
            ans.indirect = true;
            ans.text += &feeder.consume(1);
        }

        if Self::eat_param(feeder, &mut ans, core) {
            Self::eat_subscript(feeder, &mut ans, core)?;
            let _ = ValueCheck::eat(feeder, &mut ans, core)?
                 || Substr::eat(feeder, &mut ans, core)
                 || Remove::eat(feeder, &mut ans, core)?
                 || Replace::eat(feeder, &mut ans, core)?;
        }
        while ! feeder.starts_with("}") {
            Self::eat_unknown(feeder, &mut ans, core)?;
        }

        ans.text += &feeder.consume(1);
        Ok(Some(ans))
    }
}
