use anyhow::Result;
use rustyline::error::ReadlineError;
// use rustyline::KeyEvent;
// use rustyline::Cmd;
use crate::sdk::context::Context;
use crate::cmd::Command;

pub fn interactive<Co, Ex, Ev, Wa>(cmd: &mut Command<Co, Ex, Ev, Wa>, ctx: &mut Context<Co, Ex, Ev, Wa>) -> Result<()> {
    let mut rl = rustyline::Editor::<()>::new();
    // rl.bind_sequence(KeyEvent::ctrl('d'), Cmd::EndOfFile);
    loop {
        let line = rl.readline("cldi> ");
        match line {
            Ok(line) => {
                rl.add_history_entry(&line);

                let args = match shell_words::split(&line) {
                    Ok(args) => args,
                    Err(e) => {
                        println!("parse error: `{}`", e);
                        continue;
                    }
                };
                let input = std::iter::once(cmd.get_name().into()).chain(args);
                if let Err(e) = cmd.try_exec_from(ctx, input) {
                    println!("{:?}", e);
                }
            }
            Err(ReadlineError::Eof) => break,
            Err(ReadlineError::Interrupted) => println!("press CTRL+D to exit"),
            Err(e) => {
                println!("readline error {}", e)
            }
        }
    }

    Ok(())
}
