use reign_ng::*;

use futures::future::join_all;
use std::env::args;


#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
#[instrument]
async fn main() -> Result<(), Error> {
    let _log_reload_handle = initialize_logger();
    let args: Vec<String> = args().collect();
    if args.len() < 4 {
        error!("Usage: bin/reign-multi inventory reign-name hostname,hostname2,…");
        let slice = &args[1..];
        return Err(anyhow!("Insuficient arguments: {:?}", slice));
    }

    // gracefully handle interrupts
    tokio::spawn(async move {
        // wait for ctrl-c trigger
        tokio::signal::ctrl_c().await?;

        // ctrl-c triggered
        warn!("Interrupted Reign.");

        Err::<(), Error>(anyhow!("Interrupted"))
    });

    let inventory = &args[1];
    let reign_name = &args[2];
    let remote_hosts = args[3].split(',').collect::<Vec<_>>();

    let mut futures = vec![];
    for remote_host in remote_hosts {
        let op = call_operation(ReignOperation::new(reign_name, inventory, remote_host));
        futures.push(op);
    }
    let results = join_all(futures).await;
    let all_results = results.into_iter().collect::<Vec<_>>();
    let fails: Vec<_> = all_results.iter().filter(|elem| elem.is_err()).collect();
    if !fails.is_empty() {
        Err(anyhow!("Failed to process async operation(s): {fails:#?}!"))
    } else {
        Ok(())
    }
}
