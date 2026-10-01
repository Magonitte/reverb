//! Execução de processos externos: sempre em Job Object (Windows) ou grupo de processos (Unix),
//! sem janela de console e com kill-on-drop (protocolo §7, regra 4).

use std::ffi::OsStr;
use std::io;
use std::process::Stdio;
use std::time::Duration;

use process_wrap::tokio::*;

/// Resultado de `run_capture`.
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Cria o comando já embrulhado: `JobObject` + `CREATE_NO_WINDOW` + `KillOnDrop` no Windows,
/// `ProcessGroup` + `KillOnDrop` no Unix. Argumentos sempre como lista (nunca string de shell).
/// Ajuste o restante (cwd, env, pipes) com `command_mut()` antes de `spawn_tool`.
pub fn tool_command<I, S>(program: impl AsRef<OsStr>, args: I) -> CommandWrap
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut wrap = CommandWrap::with_new(program, |command| {
        command.args(args);
        command.stdin(Stdio::null());
    });
    #[cfg(windows)]
    {
        use windows::Win32::System::Threading::CREATE_NO_WINDOW;
        wrap.wrap(CreationFlags(CREATE_NO_WINDOW));
        wrap.wrap(JobObject);
    }
    #[cfg(unix)]
    {
        wrap.wrap(ProcessGroup::leader());
        wrap.wrap(KillGroupOnDrop);
    }
    wrap.wrap(KillOnDrop);
    wrap
}

/// No Unix, o `KillOnDrop` do tokio só mata o processo líder; este shim mata o grupo inteiro.
#[cfg(unix)]
#[derive(Debug, Clone, Copy)]
struct KillGroupOnDrop;

#[cfg(unix)]
impl CommandWrapper for KillGroupOnDrop {
    fn wrap_child(
        &mut self,
        child: Box<dyn ChildWrapper>,
        _core: &CommandWrap,
    ) -> io::Result<Box<dyn ChildWrapper>> {
        Ok(Box::new(GroupKillChild(Some(child))))
    }
}

#[cfg(unix)]
#[derive(Debug)]
struct GroupKillChild(Option<Box<dyn ChildWrapper>>);

#[cfg(unix)]
impl ChildWrapper for GroupKillChild {
    fn inner(&self) -> &dyn ChildWrapper {
        self.0.as_deref().expect("filho presente")
    }
    fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
        self.0.as_deref_mut().expect("filho presente")
    }
    fn into_inner(mut self: Box<Self>) -> Box<dyn ChildWrapper> {
        self.0.take().expect("filho presente")
    }
}

#[cfg(unix)]
impl Drop for GroupKillChild {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.start_kill();
        }
    }
}

/// Inicia o processo. Use `kill_tree` para encerrar a árvore inteira.
pub fn spawn_tool(mut command: CommandWrap) -> io::Result<Box<dyn ChildWrapper>> {
    command.spawn()
}

/// Mata o processo e todos os descendentes e espera o encerramento.
pub async fn kill_tree(child: &mut dyn ChildWrapper) -> io::Result<()> {
    Box::into_pin(child.kill()).await
}

/// Executa até o fim capturando stdout/stderr. Estourar `timeout` mata a árvore (kill-on-drop).
pub async fn run_capture<I, S>(
    program: impl AsRef<OsStr>,
    args: I,
    configure: impl FnOnce(&mut tokio::process::Command),
    timeout: Duration,
) -> io::Result<ToolOutput>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut wrap = tool_command(program, args);
    {
        let command = wrap.command_mut();
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        configure(command);
    }
    let child = spawn_tool(wrap)?;
    let output = tokio::time::timeout(timeout, Box::into_pin(child.wait_with_output()))
        .await
        .map_err(|_| {
            io::Error::new(io::ErrorKind::TimedOut, "o processo excedeu o tempo limite")
        })??;
    Ok(ToolOutput {
        success: output.status.success(),
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[cfg(test)]
mod tests;
