use crate::*;
use tokio::fs::OpenOptions;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::process::Command;

use std::{
    process::{exit, ExitStatus, Output, Stdio, Termination},
    thread,
};


/// Run a shell command asynchronously, with streaming stdout/stderr + file logging
#[instrument(skip(cmnd, env, identifier_reign))]
pub async fn run(
    cmnd: &str,
    env: &[(&str, &str)],
    identifier_reign: &str,
) -> Result<ExitStatus, Error> {
    let args = cmnd.split_whitespace().collect::<Vec<&str>>();
    let command = args[0];
    let mut cmd = Command::new(command);
    cmd.kill_on_drop(false);
    cmd.current_dir(DEFAULT_SHABLE_DIR);
    cmd.args(&args[1..]);
    cmd.envs(env.to_vec());
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let stdout = child
        .stdout
        .take()
        .expect("child did not have a handle to stdout");
    let stderr = child
        .stderr
        .take()
        .expect("child did not have a handle to stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let stdout_log_name = &format!("logfile-{identifier_reign}-{command}-stdout.log");
    let mut stdout_logfile = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(stdout_log_name)
        .await?;
    let stderr_log_name = &format!("logfile-{identifier_reign}-{command}-stderr.log");
    let mut stderr_logfile = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&stderr_log_name)
        .await?;

    while let Some(line) = stdout_reader.next_line().await? {
        info!("{line}");
        let bytes = format!("{line}\n").into_bytes();
        stdout_logfile.write_all(&bytes).await?;
        stdout_logfile.flush().await?
    }
    while let Some(line) = stderr_reader.next_line().await? {
        warn!("ERR {line}");
        let bytes = format!("{line}\n").into_bytes();
        stderr_logfile.write_all(&bytes).await?;
        stderr_logfile.flush().await?
    }

    let status = tokio::spawn(async move {
        let status = child
            .wait()
            .await
            .expect("child process encountered an error");
        trace!("Child {status}");
        status
    })
    .await?;

    // delete stdout/ stderr if empty:
    if stdout_logfile.metadata().await?.len() == 0 {
        trace!("Dropping empty stdout log: {stdout_log_name}");
        drop(stdout_logfile);
        tokio::fs::remove_file(stdout_log_name).await?;
    }
    if stderr_logfile.metadata().await?.len() == 0 {
        trace!("Dropping empty stderr log: {stderr_log_name}");
        drop(stderr_logfile);
        tokio::fs::remove_file(stderr_log_name).await?;
    }

    let code = status.code().unwrap_or(-1);
    if status.success() && code == 0 {
        Ok(status)
    } else {
        Err(anyhow!("Command: \"{cmnd}\". Exit code: {code}"))
    }
}


/// create archive with all necessary files
#[instrument(skip(default_env))]
pub async fn tar_command(
    operation: &ReignOperation,
    default_env: &[(&str, &str)],
) -> Result<ExitStatus, Error> {
    let op_uuid = &operation.op_uuid;
    let remote_user = &operation.remote_user;
    let files_to_sync = gather_files_to_sync().await?;
    let files_count = files_to_sync.len();
    let files_to_sync_str = files_to_sync
        .into_iter()
        .map(|file| file.replace(&format!("{DEFAULT_SHABLE_DIR}/"), ""))
        .collect::<Vec<_>>()
        .join(" ");
    let command = &format!(
        "tar --zstd -cf {op_uuid}{DEFAULT_ARCHIVE_EXT} --uname {remote_user} --gname {remote_user} --no-xattrs {files_to_sync_str}"
    );
    trace!("Cmd: {command}");
    info!("Building archive… (total files: {files_count})");
    run(command, default_env, op_uuid).await
}


/// make remote dirs
#[instrument(skip(default_env))]
pub async fn ssh_mkdir_command(
    operation: &ReignOperation,
    default_env: &[(&str, &str)],
) -> Result<ExitStatus, Error> {
    let op_uuid = &operation.op_uuid;
    let remote_user = &operation.remote_user_ssh();
    let remote_host = &operation.remote_host;
    let remote_project_path = &operation.remote_project_path();

    let command = &format!("ssh {remote_user}{remote_host} mkdir -p {remote_project_path}");
    trace!("Cmd: {command}");
    info!("Creating remote dirs…");
    run(command, default_env, op_uuid).await
}


/// sync over sftp
#[instrument(skip(default_env))]
pub async fn upload_command(
    operation: &ReignOperation,
    default_env: &[(&str, &str)],
) -> Result<ExitStatus, Error> {
    let op_uuid = &operation.op_uuid;
    let remote_user = &operation.remote_user_ssh();
    let remote_host = &operation.remote_host;
    let remote_project_path = &operation.remote_project_path();
    let file_to_sync = &format!("{op_uuid}{DEFAULT_ARCHIVE_EXT}");
    let command = &format!(
        "scp -4Bp {DEFAULT_SHABLE_DIR}/{file_to_sync} {remote_user}{remote_host}:{remote_project_path}/{file_to_sync}"
    );
    trace!("Cmd: {command}");
    info!("Uploading…");
    run(command, default_env, op_uuid).await
}


/// unpack the tarball
#[instrument(skip(default_env))]
pub async fn unpack_command(
    operation: &ReignOperation,
    default_env: &[(&str, &str)],
) -> Result<ExitStatus, Error> {
    let op_uuid = &operation.op_uuid;
    let remote_user = &operation.remote_user_ssh();
    let remote_host = &operation.remote_host;
    let remote_project_path = &operation.remote_project_path();
    let command = &format!(
        "ssh {remote_user}{remote_host} cd {remote_project_path}; tar xf {op_uuid}{DEFAULT_ARCHIVE_EXT}",
    );
    trace!("Cmd: {command}");
    info!("Unpacking…");
    run(command, default_env, op_uuid).await
}


/// call a reign
#[instrument(skip(operation, default_env))]
pub async fn reign_command(
    operation: &ReignOperation,
    default_env: &[(&str, &str)],
) -> Result<ExitStatus, Error> {
    let op_uuid = &operation.op_uuid;
    let inventory = &operation.inventory;
    let reign_name = &operation.reign_name;
    let remote_user = &operation.remote_user_ssh();
    let remote_host = &operation.remote_host;
    let remote_project_path = &operation.remote_project_path();

    // TODO:  the two special Shable variables, possibly we can get rid of these soon™
    let debug_env = read_env(default_env, "DEBUG");
    let skip_env_validation = read_env(default_env, "SKIP_ENV_VALIDATION");

    let command = &format!(
        "ssh {remote_user}{remote_host} cd {remote_project_path} && /bin/sh -c 'export DEBUG={debug_env} SKIP_ENV_VALIDATION={skip_env_validation} && bin/shable {inventory} {reign_name} 2>&1'"
    );
    trace!("Cmd: {command}");
    info!("Reign => {reign_name} on {remote_user}{remote_host}:{remote_project_path}");
    run(command, default_env, op_uuid).await
}


/// perform cleanup
#[instrument(skip(default_env))]
pub async fn cleanup_command(
    operation: &ReignOperation,
    default_env: &[(&str, &str)],
) -> Result<ExitStatus, Error> {
    let op_uuid = &operation.op_uuid;
    let remote_user = &operation.remote_user_ssh();
    let remote_host = &operation.remote_host;
    let remote_project_path = &operation.remote_project_path();

    let command = &format!("ssh {remote_user}{remote_host} rm -rf {remote_project_path}");
    debug!("Cleanup: {remote_user}{remote_host}:{remote_project_path}");
    run(command, default_env, op_uuid).await.unwrap_or_default();

    let command = &format!("rm -f {op_uuid}{DEFAULT_ARCHIVE_EXT}");
    debug!("Cleanup: {op_uuid}{DEFAULT_ARCHIVE_EXT}");
    run(command, default_env, op_uuid).await
}
