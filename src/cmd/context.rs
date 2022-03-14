use clap::Arg;

use anyhow::anyhow;
use serde_json::json;

use crate::{
    cmd::Command,
    core::{client::GrpcClientBehaviour, context::Context},
    display::Display,
};

// TODO: consider if it's appropriate to use config internals here.
// I think it's OK, at least for now.

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

pub fn default<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>> {
    Command::<Context<Co, Ex, Ev>>::new("default-context")
        .about("set a context as default")
        .arg(Arg::new("context-name").takes_value(true).required(true))
        .handler(|_cmd, m, ctx| {
            let context_name = m.value_of("context-name").unwrap();
            ctx.get_context_setting(context_name)?;
            ctx.config.default_context = context_name.into();
            ctx.config.save()?;

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
        .about("context commands")
        .subcommand_required_else_help(true)
        .subcommands([
            save().name("save"),
            list().name("list").aliases(&["ls", "l"]),
            delete().name("delete").aliases(&["del", "rm"]),
            default().name("default"),
        ])
}
