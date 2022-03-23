// Copyright Rivtower Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use clap::Arg;

use anyhow::anyhow;
use serde_json::json;

use crate::{
    cmd::Command,
    core::{client::GrpcClientBehaviour, context::Context},
    display::Display,
};

// TODO: consider if it's appropriate to use config internals here.
// I think it's OK, at least for now. If it gets complicated, we should
// use methods provided by Context instead of depending on its internals.

pub fn save<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("save-context")
        .arg(Arg::new("context-name").takes_value(true).required(true))
        .handler(|_cmd, m, ctx| {
            let context_name = m.value_of("context-name").unwrap();
            let current_setting = ctx.current_setting.clone();
            ctx.config
                .context_settings
                .insert(context_name.into(), current_setting);
            ctx.config.save()?;

            Ok(())
        })
}

pub fn delete<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("delete-context")
        .arg(Arg::new("context-name").takes_value(true).required(true))
        .handler(|_cmd, m, ctx| {
            let context_name = m.value_of("context-name").unwrap();
            ctx.config
                .context_settings
                .remove(context_name)
                .ok_or_else(|| anyhow!("context `{}` not found", context_name))?;
            ctx.config.save()?;

            Ok(())
        })
}

pub fn list<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("list-context")
        .about("list contexts")
        .handler(|_cmd, _m, ctx| {
            let json = json!({
                "current_context": &ctx.current_setting,
                "default_context": &ctx.config.default_context,
                "contexts": &ctx.config.context_settings,
            });
            println!("{}", json.display());

            Ok(())
        })
}

pub fn default<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: GrpcClientBehaviour,
    Ex: GrpcClientBehaviour,
    Ev: GrpcClientBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("default-context")
        .about("set a context as default and switch current context to it")
        .arg(Arg::new("context-name").takes_value(true).required(true))
        .handler(|_cmd, m, ctx| {
            let context_name = m.value_of("context-name").unwrap();
            let setting = ctx.get_context_setting(context_name)?.clone();
            ctx.config.default_context = context_name.into();
            ctx.config.save()?;
            ctx.switch_context(setting)?;

            Ok(())
        })
}

pub fn context_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: GrpcClientBehaviour,
    Ex: GrpcClientBehaviour,
    Ev: GrpcClientBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("context")
        .alias("ctx")
        .about("Context commands")
        .subcommand_required_else_help(true)
        .subcommands([
            save().name("save"),
            list().name("list").aliases(&["ls", "l"]),
            delete().name("delete").aliases(&["del", "rm"]),
            default().name("default"),
        ])
}
