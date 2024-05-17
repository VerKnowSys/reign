use reign_ng::*;

use chrono::Local;
use futures::StreamExt;
use std::env::args;


#[tokio::main]
#[instrument]
async fn main() -> Result<(), Error> {
    let _log_reload_handle = initialize_logger();
    let cpu_cores = num_cpus::get();
    let total_time = Local::now();
    let args: Vec<String> = args().collect();
    if args.len() < 4 {
        error!("Usage: bin/reign-multi inventory reign-name hostname hostnameN […]");
        return Err(anyhow!("Insuficient arguments: {:?}", &args[1..]));
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
    let remote_hosts = &args[3..];

    let futures = remote_hosts
        .iter()
        .map(|remote_host| {
            call_operation(ReignOperation::new(reign_name, inventory, remote_host))
        })
        .collect::<Vec<_>>();

    // create a buffered stream that will execute up to cpu_cores futures in parallel
    let stream = futures::stream::iter(futures).buffer_unordered(cpu_cores);

    // wait for all futures to complete
    info!("Streaming the process to max: {cpu_cores} ops at once");
    let results = stream.collect::<Vec<_>>().await;
    let all_results = results.into_iter().collect::<Vec<_>>();
    info!(
        "All operations took {} seconds",
        (Local::now() - total_time).num_seconds()
    );

    let fails: Vec<_> = all_results.iter().filter(|elem| elem.is_err()).collect();
    if !fails.is_empty() {
        Err(anyhow!("Failed to process async operation(s): {fails:#?}!"))
    } else {
        Ok(())
    }
}
