use rustyline::error::ReadlineError;
use rustyline::Editor;

pub struct Interactive;

impl Interactive {
    pub fn run() {
        println!("interactive mode unimplemented yet, but you can have fun here.");

        // `()` can be used when no completer is required
        let mut rl = Editor::<()>::new();
        loop {
            let readline = rl.readline("> ");
            match readline {
                Ok(line) => {
                    println!("Line: {}", line);
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
        rl.save_history("history.txt").unwrap();
    }
}
