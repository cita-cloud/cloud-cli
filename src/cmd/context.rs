use crate::core::context::Context;
use super::Command;
use clap::Arg;
use crate::core::client::GrpcClientBehaviour;
use serde_json::json;
use crate::display::Display;
use anyhow::anyhow;


pub fn save<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
{
    Command::<Context<Co, Ex, Ev>>::new("save-context")
        .arg(
            Arg::new("context-name")
                .takes_value(true)
                .required(true)
        )
        .handler(|_cmd, m, ctx| {
            let context_name = m.value_of("context-name").unwrap();
            let current_setting = ctx.current_setting.clone();
            ctx.config.context_settings.insert(context_name.into(), current_setting);
            ctx.config.save()?;

            Ok(())
        })
}

pub fn delete<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
{
    Command::<Context<Co, Ex, Ev>>::new("delete-context")
        .arg(
            Arg::new("context-name")
                .takes_value(true)
                .required(true)
        )
        .handler(|_cmd, m, ctx| {
            let context_name = m.value_of("context-name").unwrap();
            ctx.config.context_settings.remove(context_name).ok_or_else(|| anyhow!("context `{}` not found", context_name))?;
            ctx.config.save()?;

            Ok(())
        })
}

pub fn switch<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: GrpcClientBehaviour,
    Ex: GrpcClientBehaviour,
    Ev: GrpcClientBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("switch-context")
        .arg(
            Arg::new("context-name")
                .takes_value(true)
                .required(true)
        )
        .handler(|_cmd, m, ctx| {
            let context_name = m.value_of("context-name").unwrap();
            ctx.switch_context_to(context_name)?;

            Ok(())
        })
}

pub fn list<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
{
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

pub fn context_cmd<'help, Co, Ex, Ev>() -> Command<'help, Context<Co, Ex, Ev>>
where
    Co: GrpcClientBehaviour,
    Ex: GrpcClientBehaviour,
    Ev: GrpcClientBehaviour,
{
    Command::<Context<Co, Ex, Ev>>::new("context")
        .alias("ctx")
        .arg_required_else_help(true)
        .subcommands([
            save().name("save"),
            switch().name("switch"),
            list().name("list").aliases(&["ls", "l"]),
            delete().name("delete").alias("del"),
        ])
}
