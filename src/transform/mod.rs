use crate::tracers::proc::Process;
use tokio_util::sync::CancellationToken;

pub async fn proc_renamer(
    ct: CancellationToken,
    mut input: tokio::sync::mpsc::Receiver<Vec<Process>>,
    mut output: tokio::sync::mpsc::Sender<Vec<Process>>,
) {
    loop {
        tokio::select!(
            _ = ct.cancelled() => {
                println!("exporter cancelled");
                return;
            }
            Some(mut procs) = input.recv() => {
                for proc in &mut procs {
                    proc.name = format!("modify-{}", proc.name);
                }
                if let Err(e) = output.send(procs).await {
                    println!("error sending: {:?}", e);
                }
            }
        )
    }
}
