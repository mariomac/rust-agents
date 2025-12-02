mod export;
mod tracers;
mod transform;

use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;

macro_rules! graph {
    {
        nodes: {
            $node_name:ident : $node_func:path
        }
    } => {
        println!("Instantiating {}: {}", stringify!($node_name), stringify!($node_func));
    };
}

#[tokio::main]
async fn main() {
    let a = 33;
    graph!{
        nodes: { procs: tracers::proc::trace_processes }
    };
    let ct = CancellationToken::new();

    let (procs_output, transform_input) = tokio::sync::mpsc::channel(100);
    let (transform_output, export_input) = tokio::sync::mpsc::channel(100);

    tokio::join!(
        tracers::proc::trace_processes(ct.clone(), procs_output),
        transform::proc_renamer(ct.clone(), transform_input, transform_output),
        export::otel::exporter(ct.clone(), export_input),
    );
}
