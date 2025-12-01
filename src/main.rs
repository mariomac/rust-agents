mod export;
mod tracers;
mod transform;

use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let ct = CancellationToken::new();

    let (procs_output, transform_input) = tokio::sync::mpsc::channel(100);
    let (transform_output, export_input) = tokio::sync::mpsc::channel(100);
    tokio::join!(
        tracers::proc::trace_processes(ct.clone(), procs_output),
        transform::proc_renamer(ct.clone(), transform_input, transform_output),
        export::otel::exporter(ct.clone(), export_input),
    );
}
