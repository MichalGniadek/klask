use crate::ExecuteError;
use std::{
    io::{BufRead, BufReader, Read},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
};

pub struct ChildApp {
    child: Child,
    stdout: Option<Receiver<Option<String>>>,
    stderr: Option<Receiver<Option<String>>>,
    output: String,
}

impl ChildApp {
    pub fn run(cmd: &mut Command) -> Result<Self, ExecuteError> {
        let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

        let stdout =
            Self::spawn_thread_reader(child.stdout.take().ok_or(ExecuteError::NoStdoutOrStderr)?);
        let stderr =
            Self::spawn_thread_reader(child.stderr.take().ok_or(ExecuteError::NoStdoutOrStderr)?);

        Ok(ChildApp {
            child,
            stdout: Some(stdout),
            stderr: Some(stderr),
            output: String::new(),
        })
    }

    pub fn read(&mut self) -> &str {
        let mut read = |stdio: &mut Option<Receiver<Option<String>>>| {
            if let Some(receiver) = stdio {
                for line in receiver.try_iter() {
                    if let Some(line) = line {
                        self.output.push_str(&line);
                    } else {
                        *stdio = None;
                        return;
                    }
                }
            }
        };
        read(&mut self.stdout);
        read(&mut self.stderr);
        &self.output
    }

    pub fn is_running(&self) -> bool {
        self.stdout.is_some() || self.stderr.is_some()
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
        self.stdout = None;
        self.stderr = None;
    }

    fn spawn_thread_reader<R: Read + Send + Sync + 'static>(stdio: R) -> Receiver<Option<String>> {
        let mut reader = BufReader::new(stdio);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || loop {
            let mut output = String::new();
            if let Ok(0) = reader.read_line(&mut output) {
                // End of output
                let _ = tx.send(None);
                break;
            }
            // Send returns error only if data will never be received
            if tx.send(Some(output)).is_err() {
                break;
            }
        });
        rx
    }
}
