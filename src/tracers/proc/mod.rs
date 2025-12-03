use crate::tracers::datapoint::Resource;
use rouille::router;
use std::collections::HashMap;
use std::ffi::OsString;
use std::time::Duration;
use sysinfo::System;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use crate::tracers::datapoint;

pub async fn trace_processes(ct: CancellationToken, out: tokio::sync::mpsc::Sender<Vec<Resource>>) {

    let mut sys = System::new_all();
    loop {
        tokio::select!(
            _ = ct.cancelled() => {
                println!("process tracer cancelled");
                return
            },
            _ = sleep(Duration::from_secs(5)) => {
                // TODO: this is slow. Cache basic resources
                // TODO: support removal of processes
                sys.refresh_all();
                let procs = sys.processes();
                if let Err(e) = out
                    .send(
                        procs
                            .iter()
                            .map(proc_to_resource)
                            .collect(),
                    )
                    .await
                {
                    println!("error sending: {:?}", e);
                }
            },
        );
    }
}

fn proc_to_resource(p: (&sysinfo::Pid, &sysinfo::Process)) -> Resource {
    let proc = p.1;
    let hn = hostname::get().unwrap_or(OsString::from("unknown"));
    let mut res = Resource::new(format!("{}:{}", hn.to_string_lossy(), proc.pid()));
    res.attrs = vec![
        ("process.pid".to_string(), proc.pid().to_string()),
        ("process.executable.path".to_string(), proc.exe().unwrap().to_string_lossy().into_owned()),
    ];
    res.metrics = vec![
        datapoint::Counter {
            name: "process.cpu.time".to_string(),
            attrs: vec![("cpu.mode".to_string(), "total".to_string())],
            value: proc.accumulated_cpu_time(),
        }
    ];
    res

}
