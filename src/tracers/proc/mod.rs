use std::path::Path;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, System};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

pub struct Process {
    pub pid: u32,
    pub exe: String,
    pub cpu_time: u64,
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
        // TODO: this is slow
        sys.refresh_all();
        let procs = sys.processes();
        if let Err(e) = out
            .send(
                procs
                    .iter()
                    .map(|(_, proc)| Process {
                        pid: proc.pid().as_u32(),
                        exe: proc
                            .exe()
                            .map_or_else(String::new, |path| path.to_string_lossy().into_owned()),
                        cpu_time: proc.accumulated_cpu_time(),
                    })
                    .collect(),
            )
            .await
        {
            println!("error sending: {:?}", e);
        }
    }
}
