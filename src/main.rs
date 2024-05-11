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

    let operation = &ReignOperation::new(reign_name, inventory, remote_host);

    // extract significant env values
    let remote_user = &operation.remote_user;
    let debug_shable = &std::env::var("DEBUG").unwrap_or_default();
    let command_path = &std::env::var("PATH").unwrap_or_default();
    let validation_shable = &std::env::var("SKIP_ENV_VALIDATION").unwrap_or_default();

    // initialize env for all future commands:
    let default_env = &[
        ("DEBUG", debug_shable.as_str()),
        ("RUN_AS", remote_user.as_str()),
        ("SKIP_ENV_VALIDATION", validation_shable.as_str()),
        ("PATH", command_path.as_str()),
    ];
    debug!("Settings: {:?}", operation);

    // start of the process
    let start = Local::now();

    // chain the run and check the status
    let tar_fut = tar_command(operation, default_env);
    let mkdir_fut = ssh_mkdir_command(operation, default_env);
    let sync_fut = upload_command(operation, default_env);
    let unpack_fut = unpack_command(operation, default_env);
    let reign_fut = reign_command(operation, default_env);
    let cleanup_fut = cleanup_command(operation, default_env);

    // run these two at once:
    let _ = join!(tar_fut, mkdir_fut);

    // perform tasks in order:
    match sync_fut.await {
        Ok(_) => {
            match unpack_fut.await {
                Ok(_) => {
                    let taken_init = Local::now();
                    let taken_s = (taken_init - start).num_seconds();
                    let remote_user = &operation.remote_user_ssh();
                    info!(
                        "Ready: '{reign_name}' on {remote_user}{remote_host} (took: {taken_s} seconds)"
                    );
                    match reign_fut.await {
                        Ok(_) => {
                            let reign_s = (Local::now() - taken_init).num_seconds();
                            cleanup_fut.await?;
                            info!(
                                "Success: '{reign_name}' on {remote_user}{remote_host} (took: {reign_s} seconds)"
                            );
                            Ok(())
                        }
                        Err(e) => {
                            cleanup_fut.await?;
                            error!("{e}");
                            Err(e)
                        }
                    }
                }
                Err(e) => {
                    cleanup_fut.await?;
                    error!("{e}");
                    Err(e)
                }
            }
        }
        Err(e) => {
            cleanup_fut.await?;
            error!("{e}");
            Err(e)
        }
    }
}
