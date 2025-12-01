use std::time::Duration;
use sysinfo::System;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

pub struct Process {
    pub pid: u32,
    pub name: String,
}

pub async fn trace_processes(ct: CancellationToken, out: tokio::sync::mpsc::Sender<Vec<Process>>) {
    let mut sys = System::new_all();
    loop {
        tokio::select!(
            _ = ct.cancelled() => {
                println!("process tracer cancelled");
                return
            },
            _ = sleep(Duration::from_secs(5)) => {},
        );
        sys.refresh_all();
        let procs = sys.processes();
        if let Err(e) = out
            .send(
                procs
                    .iter()
                    .map(|(_, proc)| Process {
                        pid: proc.pid().as_u32(),
                        name: proc.name().to_os_string().into_string().unwrap_or_default(),
                    })
                    .collect(),
            )
            .await
        {
            println!("error sending: {:?}", e);
        }
    }
}
