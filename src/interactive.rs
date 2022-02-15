use anyhow::Result;
use rustyline::error::ReadlineError;
// use rustyline::KeyEvent;
// use rustyline::Cmd;
use crate::{
    sdk::{ self, context::Context },
    cmd::Command,
    config::Config,
    crypto::{ EthCrypto, SmCrypto },
};


enum MultiCryptoContext {
    Sm,
    Eth,
}


pub fn interactive() -> Result<()> {
    let config = Config {
        controller_addr: "localhost:50005".into(),
        executor_addr: "localhost:50002".into(),
        default_account: None,
        wallet_dir: "d:/cld/cloud-cli/tmp-wallet".into(),
    };

    let mut ctx = sdk::context::from_config::<SmCrypto>(&config).unwrap();
    let mut cmd = all_cmd();

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
                if let Err(e) = cmd.exec_from(&mut ctx, input) {
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
