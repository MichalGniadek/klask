use crate::{ExecutionError, CHILD_APP_ENV_VAR};
use eframe::egui;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
};

#[derive(Debug)]
pub struct ChildApp {
    child: Child,
    stdout: Option<Receiver<Option<String>>>,
    stderr: Option<Receiver<Option<String>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StdinType {
    File(String),
    Text(String),
}

impl ChildApp {
    pub fn run(
        args: Vec<String>,
        env: Option<Vec<(String, String)>>,
        stdin: Option<StdinType>,
        working_dir: Option<String>,
        ctx: egui::Context,
    ) -> Result<Self, ExecutionError> {
        let mut child = Command::new(std::env::current_exe()?);

        child
            .env(CHILD_APP_ENV_VAR, "")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(env) = env {
            child.envs(env);
        }

        if let Some(working_dir) = working_dir {
            if !working_dir.is_empty() {
                child.current_dir(PathBuf::from(working_dir).canonicalize()?);
            }
        }

        let mut child = child.spawn()?;

        let stdout = Self::spawn_thread_reader(
            child
                .stdout
                .take()
                .ok_or(ExecutionError::NoStdoutOrStderr)?,
            ctx.clone(),
        );

        let stderr = Self::spawn_thread_reader(
            child
                .stderr
                .take()
                .ok_or(ExecutionError::NoStdoutOrStderr)?,
            ctx,
        );

        if let Some(stdin) = stdin {
            let mut child_stdin = child.stdin.take().unwrap();
            match stdin {
                StdinType::Text(text) => {
                    child_stdin.write_all(text.as_bytes())?;
                }
                StdinType::File(path) => {
                    let mut file = File::open(path)?;
                    std::io::copy(&mut file, &mut child_stdin)?;
                }
            }
        }

        Ok(Self {
            child,
            stdout: Some(stdout),
            stderr: Some(stderr),
        })
    }

    pub fn read(&mut self) -> String {
        let mut out = String::new();
        Self::read_stdio(&mut out, &mut self.stdout);
        Self::read_stdio(&mut out, &mut self.stderr);
        out
    }

    pub fn is_running(&self) -> bool {
        self.stdout.is_some() || self.stderr.is_some()
    }

    pub fn kill(&mut self) {
        drop(self.child.kill());
        self.stdout = None;
        self.stderr = None;
    }

    fn spawn_thread_reader<R: Read + Send + Sync + 'static>(
        stdio: R,
        ctx: egui::Context,
    ) -> Receiver<Option<String>> {
        let mut reader = BufReader::new(stdio);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || loop {
            let mut output = String::new();
            if let Ok(0) = reader.read_line(&mut output) {
                // End of output
                drop(tx.send(None));
                ctx.request_repaint();
                break;
            }
            // Send returns error only if data will never be received
            if tx.send(Some(output)).is_err() {
                break;
            }
            ctx.request_repaint();
        });
        rx
    }

    fn read_stdio(output: &mut String, stdio: &mut Option<Receiver<Option<String>>>) {
        if let Some(receiver) = stdio {
            for line in receiver.try_iter() {
                if let Some(line) = line {
                    output.push_str(&line);
                } else {
                    *stdio = None;
                    return;
                }
            }
        }
    }
}

impl Drop for ChildApp {
    fn drop(&mut self) {
        self.kill();
    }
}
