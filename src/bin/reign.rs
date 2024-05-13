use reign_ng::*;

use std::env::args;


#[tokio::main]
#[instrument]
async fn main() -> Result<(), Error> {
    let _log_reload_handle = initialize_logger();
    let args: Vec<String> = args().collect();
    if args.len() < 4 {
        error!("Usage: bin/reign inventory reign-name hostname");
        return Err(anyhow!("Insuficient arguments: {:?}", &args[1..]));
    }

    let inventory = &args[1];
    let reign_name = &args[2];
    let remote_host = &args[3];

    // gracefully handle interrupts
    tokio::spawn(async move {
        // wait for ctrl-c trigger
        tokio::signal::ctrl_c().await?;

        // ctrl-c triggered
        warn!("Interrupted Reign.");

        Err::<(), Error>(anyhow!("Interrupted"))
    });

    let operation = ReignOperation::new(reign_name, inventory, remote_host);
    call_operation(operation).await
}
