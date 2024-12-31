//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::{ShellCore, Feeder, Script};
use super::{Command, Redirect};
use crate::elements::command;

#[derive(Debug, Default)]
pub struct BraceCommand {
    text: String,
    script: Option<Script>,
    redirects: Vec<Redirect>,
    force_fork: bool,
}

impl Command for BraceCommand {
    fn run(&mut self, core: &mut ShellCore, _: bool) {
        match self.script {
            Some(ref mut s) => s.exec(core),
            _ => panic!("SUSH INTERNAL ERROR (ParenCommand::exec)"),
        }
    }

    fn get_text(&self) -> String { self.text.clone() }
    fn get_redirects(&mut self) -> &mut Vec<Redirect> { &mut self.redirects }
    fn set_force_fork(&mut self) { self.force_fork = true; }
    fn force_fork(&self) -> bool { self.force_fork }
}

impl BraceCommand {
    pub fn parse(feeder: &mut Feeder, core: &mut ShellCore) -> Option<BraceCommand> {
        let mut ans = Self::default();
        if command::eat_inner_script(feeder, core, "{", vec!["}"], &mut ans.script) {
            ans.text.push_str("{");
            ans.text.push_str(&ans.script.as_ref().unwrap().get_text());
            ans.text.push_str(&feeder.consume(1));

            command::eat_redirects(feeder, core, &mut ans.redirects, &mut ans.text);
            Some(ans)
        }else{
            None
        }
    }
}
