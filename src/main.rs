use std::env;

use reign_ng::*;

use chrono::Local;
use futures::join;


#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
#[instrument]
async fn main() -> Result<(), Error> {
    let _log_reload_handle = initialize_logger();
    info!(
        "Starting {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );


    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        error!("Usage: bin/reign inventory reign-name hostname");
        return Err(anyhow!("Insuficient arguments: {:?}", &args[1..]));
    }

    let inventory = &args[1];
    let reign_name = &args[2];
    let remote_host = &args[3];

    // info!("Determine the remote host type…");


    let op_uuid = &uuidv4::uuid::v4();
    let remote_project_path = &format!("/tmp/reigns_{op_uuid}");
    let remote_user = &std::env::var("RUN_AS").unwrap_or(String::from(""));
    let debug_shable = &std::env::var("DEBUG").unwrap_or_default();
    let the_path = &std::env::var("PATH")?;
    let default_env = &[
        ("SKIP_ENV_VALIDATION", "1"),
        ("RUN_AS", remote_user),
        ("PATH", the_path),
        ("DEBUG", debug_shable),
    ];
    debug!(
        "Settings: Reign: {reign_name}, Inventory: {inventory}, {}Hostname: {remote_host}, UUID: {op_uuid}",
        if remote_user.is_empty() {
            String::new()
        } else {
            format!("User: {remote_user}, ")
        }
    );

    // start of the process
    let start = Local::now();

    // chain the run and check the status
    let tar_fut = tar_command(remote_user, op_uuid, default_env);
    let mkdir_fut = ssh_mkdir_command(
        remote_user,
        remote_host,
        remote_project_path,
        op_uuid,
        default_env,
    );
    let sync_fut = upload_command(
        remote_user,
        remote_host,
        remote_project_path,
        op_uuid,
        default_env,
    );
    let unpack_fut = unpack_command(
        remote_user,
        remote_host,
        remote_project_path,
        op_uuid,
        default_env,
    );
    let reign_fut = reign_command(
        remote_user,
        remote_host,
        remote_project_path,
        inventory,
        reign_name,
        op_uuid,
        default_env,
    );
    let cleanup_fut = cleanup_command(
        op_uuid,
        remote_user,
        remote_host,
        remote_project_path,
        default_env,
    );

    let cleanup_fail_fut = cleanup_command(
        op_uuid,
        remote_user,
        remote_host,
        remote_project_path,
        default_env,
    );

    // run these two at once:
    let _ = join!(tar_fut, mkdir_fut);

    // perform tasks in order:
    match sync_fut.await {
        Ok(_) => {
            match unpack_fut.await {
                Ok(_) => {
                    let taken_init = Local::now();
                    let taken_s = (taken_init - start).num_seconds();
                    let user_override = if remote_user.is_empty() {
                        String::new()
                    } else {
                        format!("{remote_user}@")
                    };
                    info!(
                        "Ready: '{reign_name}' on {user_override}{remote_host} (took: {taken_s} seconds)"
                    );
                    match reign_fut.await {
                        Ok(_) => {
                            cleanup_fut.await?;
                            let reign_s = (Local::now() - taken_init).num_seconds();
                            info!(
                                "Success: '{reign_name}' on {remote_user}@{remote_host} (took: {reign_s} seconds)"
                            );
                            Ok(())
                        }
                        Err(e) => {
                            cleanup_fail_fut.await?;
                            error!("{e}");
                            Err(e)
                        }
                    }
                }
                Err(e) => {
                    cleanup_fail_fut.await?;
                    error!("{e}");
                    Err(e)
                }
            }
        }
        Err(e) => {
            cleanup_fail_fut.await?;
            error!("{e}");
            Err(e)
        }
    }
}
